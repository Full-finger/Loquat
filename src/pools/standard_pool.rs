//! Standard pool implementation

use crate::errors::{ConfigError, LoquatError};
use crate::events::Package;
use crate::pools::traits::Pool;
use crate::pools::PoolType;
use crate::pools::validator::PoolValidator;
use crate::workers::OutputSafe;
use crate::workers::WorkerRegistration;
use crate::workers::WorkerResult;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

/// Standard pool implementation
pub struct StandardPool {
    pool_id: String,
    pool_type: PoolType,
    workers: Vec<WorkerRegistration>,
    worker_index: HashMap<String, usize>, // worker_name -> index in workers vec
    logger: Arc<dyn crate::logging::Logger>,
    validator: PoolValidator,
}

impl StandardPool {
    /// Create a new standard pool
    pub fn new(pool_type: PoolType, logger: Arc<dyn crate::logging::Logger>) -> Self {
        Self {
            pool_id: format!("pool_{}", pool_type),
            pool_type,
            workers: Vec::new(),
            worker_index: HashMap::new(),
            logger,
            validator: PoolValidator::new(),
        }
    }
    
    /// Create a new pool with custom ID
    pub fn with_id(pool_id: String, pool_type: PoolType, logger: Arc<dyn crate::logging::Logger>) -> Self {
        Self {
            pool_id,
            pool_type,
            workers: Vec::new(),
            worker_index: HashMap::new(),
            logger,
            validator: PoolValidator::new(),
        }
    }
    
    /// Get workers sorted by priority
    pub fn workers_sorted(&self) -> &[WorkerRegistration] {
        &self.workers
    }
    
    /// Get a worker by name
    pub fn get_worker(&self, name: &str) -> Option<&WorkerRegistration> {
        self.worker_index.get(name).map(|&idx| &self.workers[idx])
    }
    
    /// Get a mutable worker by name
    pub fn get_worker_mut(&mut self, name: &str) -> Option<&mut WorkerRegistration> {
        if let Some(&idx) = self.worker_index.get(name) {
            Some(&mut self.workers[idx])
        } else {
            None
        }
    }
    
    /// Sort workers by priority
    fn sort_workers(&mut self) {
        self.workers.sort_by_key(|w| w.priority);
        // Update index
        self.worker_index.clear();
        for (idx, w) in self.workers.iter().enumerate() {
            self.worker_index.insert(w.worker.name().to_string(), idx);
        }
    }
}

impl Debug for StandardPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StandardPool")
            .field("pool_id", &self.pool_id)
            .field("pool_type", &self.pool_type)
            .field("workers", &self.workers.len())
            .finish()
    }
}

impl OutputSafe<Package> for StandardPool {
    fn is_output_safe(&self, output: &Package) -> bool {
        !self.workers.iter().any(|w| w.worker.is_output_safe(output))
    }
}

#[async_trait]
impl Pool for StandardPool {
    fn pool_id(&self) -> &str {
        &self.pool_id
    }
    
    fn pool_type(&self) -> PoolType {
        self.pool_type
    }
    
    fn register(&mut self, registration: WorkerRegistration) -> crate::errors::Result<()> {
        let worker_name = registration.worker.name();
        
        // Check if worker already exists
        if self.worker_index.contains_key(worker_name) {
            return Err(LoquatError::Config(
                ConfigError::InvalidFormat(format!(
                    "Worker '{}' already exists in pool '{}'",
                    worker_name, self.pool_id
                ))
            ));
        }
        
        // Check if priority is unique
        if self.workers.iter().any(|w| w.priority == registration.priority) {
            return Err(LoquatError::Config(
                ConfigError::InvalidFormat(format!(
                    "Worker priority {} already exists in pool '{}'",
                    registration.priority, self.pool_id
                ))
            ));
        }
        
        self.workers.push(registration);
        self.sort_workers();
        
        Ok(())
    }
    
    fn unregister(&mut self, name: &str) -> crate::errors::Result<()> {
        if let Some(&idx) = self.worker_index.get(name) {
            self.workers.remove(idx);
            self.sort_workers();
            Ok(())
        } else {
            Err(LoquatError::Config(
                ConfigError::MissingRequired(format!(
                    "Worker '{}' not found in pool '{}'",
                    name, self.pool_id
                ))
            ))
        }
    }
    
    fn worker_names(&self) -> Vec<String> {
        self.workers.iter().map(|w| w.worker.name().to_string()).collect()
    }
    
    fn worker_count(&self) -> usize {
        self.workers.len()
    }
    
    fn has_worker(&self, name: &str) -> bool {
        self.worker_index.contains_key(name)
    }
    
    fn set_worker_priority(&mut self, name: &str, new_priority: u32) -> crate::errors::Result<()> {
        // Check if new priority is already used
        if let Some(&idx) = self.worker_index.get(name) {
            if self.workers.iter().any(|w| w.priority == new_priority && w.worker.name() != name) {
                return Err(LoquatError::Config(
                    ConfigError::InvalidFormat(format!(
                        "Worker priority {} already used in pool '{}'",
                        new_priority, self.pool_id
                    ))
                ));
            }
            self.workers[idx].priority = new_priority;
            self.sort_workers();
            Ok(())
        } else {
            Err(LoquatError::Config(
                ConfigError::MissingRequired(format!(
                    "Worker '{}' not found in pool '{}'",
                    name, self.pool_id
                ))
            ))
        }
    }
    
    async fn process_batch(&self, packages: Vec<Package>) -> Vec<Package> {
        let mut next_pool_packages: Vec<Package> = Vec::new();
        let mut current_pool_packages: Vec<Package> = packages;
        
        // Process packages through workers until no more modifications
        while !current_pool_packages.is_empty() {
            let mut next_batch: Vec<Package> = Vec::new();
            
            for package in current_pool_packages {
                let mut processed = false;
                
                // Iterate through workers in priority order
                for worker in &self.workers {
                    // Check if worker matches any target site in package
                    if worker.matches_any(&package.target_sites) {
                        // Worker matches, process (clone to preserve ownership)
                        let result = worker.worker.handle_batch(vec![package.clone()]).await;
                        match result {
                            WorkerResult::Release => {
                                // Worker completed, package moves to next pool
                                next_pool_packages.push(package.clone());
                                processed = true;
                                break; // Break out of worker loop
                            }
                            WorkerResult::Modify(new_packages) => {
                                // Modified packages continue in current pool
                                for new_pkg in new_packages {
                                    // Validate output safety
                                    if worker.worker.is_output_safe(&new_pkg) {
                                        next_batch.push(new_pkg);
                                    } else {
                                        // Log dead loop warning
                                        self.validator.log_dead_loop_warning(
                                            self.logger.as_ref(),
                                            worker.worker.name(),
                                            &new_pkg,
                                        );
                                    }
                                }
                                processed = true;
                                break; // Break out of worker loop
                            }
                        }
                    }
                    // No match, continue to next worker
                }
                
                // If no worker processed the package, move to next pool
                if !processed {
                    next_pool_packages.push(package);
                }
            }
            
            // Move next_batch to current_pool_packages for next iteration
            current_pool_packages = next_batch;
        }
        
        next_pool_packages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::TargetSite;
    use crate::logging::StructuredLogger;
    use crate::workers::WorkerType;

    #[derive(Debug)]
    struct TestWorker {
        name: String,
        worker_type: WorkerType,
    }

    #[async_trait]
    impl crate::workers::Worker for TestWorker {
        fn name(&self) -> &str {
            &self.name
        }

        fn worker_type(&self) -> WorkerType {
            self.worker_type.clone()
        }

        fn matches(&self, _target_site: &TargetSite) -> bool {
            true
        }

        async fn handle_batch(&self, _packages: Vec<Package>) -> WorkerResult {
            WorkerResult::release()
        }
    }

    fn create_test_pool(pool_type: PoolType) -> StandardPool {
        use crate::logging::formatters::JsonFormatter;
        use crate::logging::writers::ConsoleWriter;
        let formatter = Arc::new(JsonFormatter::new());
        let writer = Arc::new(ConsoleWriter::new());
        let logger = Arc::new(StructuredLogger::new(formatter, writer));
        StandardPool::new(pool_type, logger)
    }

    #[test]
    fn test_pool_creation() {
        let pool = create_test_pool(PoolType::Input);
        assert_eq!(pool.pool_id(), "pool_input");
        assert_eq!(pool.pool_type(), PoolType::Input);
        assert_eq!(pool.worker_count(), 0);
    }

    #[test]
    fn test_pool_with_custom_id() {
        use crate::logging::formatters::JsonFormatter;
        use crate::logging::writers::ConsoleWriter;
        let formatter = Arc::new(JsonFormatter::new());
        let writer = Arc::new(ConsoleWriter::new());
        let logger = Arc::new(StructuredLogger::new(formatter, writer));
        let pool = StandardPool::with_id("custom_id".to_string(), PoolType::Process, logger);
        assert_eq!(pool.pool_id(), "custom_id");
    }

    #[tokio::test]
    async fn test_register_worker() {
        let mut pool = create_test_pool(PoolType::Input);
        
        let worker = Box::new(TestWorker {
            name: "test_worker".to_string(),
            worker_type: WorkerType::Input,
        });
        let registration = WorkerRegistration::new(worker, crate::workers::MatchingRule::All, 0);
        
        assert!(pool.register(registration).is_ok());
        assert_eq!(pool.worker_count(), 1);
        assert!(pool.has_worker("test_worker"));
    }

    #[test]
    fn test_register_duplicate_worker() {
        let mut pool = create_test_pool(PoolType::Input);
        
        let worker1 = Box::new(TestWorker {
            name: "test_worker".to_string(),
            worker_type: WorkerType::Input,
        });
        let reg1 = WorkerRegistration::new(worker1, crate::workers::MatchingRule::All, 0);
        pool.register(reg1).unwrap();
        
        let worker2 = Box::new(TestWorker {
            name: "test_worker".to_string(),
            worker_type: WorkerType::Input,
        });
        let reg2 = WorkerRegistration::new(worker2, crate::workers::MatchingRule::All, 1);
        
        assert!(pool.register(reg2).is_err());
    }

    #[test]
    fn test_unregister_worker() {
        let mut pool = create_test_pool(PoolType::Input);
        
        let worker = Box::new(TestWorker {
            name: "test_worker".to_string(),
            worker_type: WorkerType::Input,
        });
        let registration = WorkerRegistration::new(worker, crate::workers::MatchingRule::All, 0);
        pool.register(registration).unwrap();
        
        assert!(pool.unregister("test_worker").is_ok());
        assert_eq!(pool.worker_count(), 0);
        assert!(!pool.has_worker("test_worker"));
    }

    #[tokio::test]
    async fn test_set_worker_priority() {
        let mut pool = create_test_pool(PoolType::Input);
        
        let worker = Box::new(TestWorker {
            name: "test_worker".to_string(),
            worker_type: WorkerType::Input,
        });
        let registration = WorkerRegistration::new(worker, crate::workers::MatchingRule::All, 0);
        pool.register(registration).unwrap();
        
        assert!(pool.set_worker_priority("test_worker", 5).is_ok());
        assert_eq!(pool.get_worker("test_worker").unwrap().priority, 5);
    }

    #[tokio::test]
    async fn test_process_batch_no_workers() {
        let pool = create_test_pool(PoolType::Input);
        let packages = vec![Package::new()];
        
        let result = pool.process_batch(packages).await;
        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn test_process_batch_release() {
        let mut pool = create_test_pool(PoolType::Input);
        
        let worker = Box::new(TestWorker {
            name: "test_worker".to_string(),
            worker_type: WorkerType::Input,
        });
        let registration = WorkerRegistration::new(worker, crate::workers::MatchingRule::All, 0);
        pool.register(registration).unwrap();
        
        let packages = vec![Package::new()];
        let result = pool.process_batch(packages).await;
        
        assert_eq!(result.len(), 1); // Package released to next pool
    }
}
