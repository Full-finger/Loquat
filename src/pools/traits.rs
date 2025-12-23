//! Pool trait definition

use crate::events::Package;
use crate::pools::PoolType;
use crate::workers::{OutputSafe, WorkerRegistration};
use std::fmt::Debug;

/// Pool trait - manages workers and processes packages
#[async_trait::async_trait]
pub trait Pool: Send + Sync + Debug + OutputSafe<Package> {
    /// Get pool ID
    fn pool_id(&self) -> &str;
    
    /// Get pool type
    fn pool_type(&self) -> PoolType;
    
    /// Register a worker
    fn register(&mut self, registration: WorkerRegistration) -> crate::errors::Result<()>;
    
    /// Unregister a worker by name
    fn unregister(&mut self, name: &str) -> crate::errors::Result<()>;
    
    /// Get all worker names
    fn worker_names(&self) -> Vec<String>;
    
    /// Get worker count
    fn worker_count(&self) -> usize;
    
    /// Check if worker exists
    fn has_worker(&self, name: &str) -> bool;
    
    /// Change worker priority
    fn set_worker_priority(&mut self, name: &str, new_priority: u32) -> crate::errors::Result<()>;
    
    /// Process a batch of packages asynchronously
    /// Returns packages that should go to the next pool
    async fn process_batch(&self, packages: Vec<Package>) -> Vec<Package>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    #[derive(Debug)]
    struct MockPool {
        id: String,
        pool_type: PoolType,
    }

    impl OutputSafe<Package> for MockPool {
        fn is_output_safe(&self, _output: &Package) -> bool {
            true
        }
    }

    #[async_trait]
    impl Pool for MockPool {
        fn pool_id(&self) -> &str {
            &self.id
        }

        fn pool_type(&self) -> PoolType {
            self.pool_type
        }

        fn register(&mut self, _registration: WorkerRegistration) -> crate::errors::Result<()> {
            Ok(())
        }

        fn unregister(&mut self, _name: &str) -> crate::errors::Result<()> {
            Ok(())
        }

        fn worker_names(&self) -> Vec<String> {
            vec![]
        }

        fn worker_count(&self) -> usize {
            0
        }

        fn has_worker(&self, _name: &str) -> bool {
            false
        }

        fn set_worker_priority(&mut self, _name: &str, _new_priority: u32) -> crate::errors::Result<()> {
            Ok(())
        }

        async fn process_batch(&self, packages: Vec<Package>) -> Vec<Package> {
            packages
        }
    }

    #[test]
    fn test_mock_pool() {
        let pool = MockPool {
            id: "test_pool".to_string(),
            pool_type: PoolType::Input,
        };

        assert_eq!(pool.pool_id(), "test_pool");
        assert_eq!(pool.pool_type(), PoolType::Input);
        assert_eq!(pool.worker_count(), 0);
        assert!(!pool.has_worker("any_worker"));
    }
}
