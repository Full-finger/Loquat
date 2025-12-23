//! Aspect-Oriented Programming (AOP) module for Loquat framework
//!
//! Provides a clean AOP implementation following SOLID principles,
//! allowing for cross-cutting concerns like logging, error tracking,
//! and performance monitoring to be applied dynamically.

pub mod aspects;
pub mod proxy;
pub mod traits;

pub use aspects::*;
pub use proxy::*;
pub use traits::*;

use crate::errors::{AopError, LoquatError, Result};
use std::sync::Arc;

/// AOP manager for coordinating aspects and proxies
pub struct AopManager {
    aspects: Vec<Arc<dyn Aspect>>,
}

impl AopManager {
    /// Create a new AOP manager
    pub fn new() -> Self {
        Self {
            aspects: Vec::new(),
        }
    }

    /// Add an aspect to the manager
    pub fn add_aspect(&mut self, aspect: Arc<dyn Aspect>) {
        self.aspects.push(aspect);
    }

    /// Create a proxy with all registered aspects
    pub fn create_proxy<T>(&self, target: T) -> AopProxy<T>
    where
        T: Send + Sync,
    {
        AopProxy::new(target, self.aspects.clone())
    }

    /// Apply aspects to a function
    pub async fn apply_aspects<F, R>(&self, operation: &str, f: F) -> Result<R>
    where
        F: FnOnce() -> Result<R> + Send,
    {
        // Execute before advice for all aspects
        for aspect in &self.aspects {
            aspect.before(operation).await?;
        }

        // Execute the target function
        let result = f();

        // Execute after advice for all aspects
        for aspect in &self.aspects {
            let unit_result: Result<()> = result.as_ref().map(|_| ()).map_err(|e| LoquatError::Aop(AopError::ExecutionFailed(e.to_string())));
            aspect.after(operation, &unit_result).await?;
        }

        result
    }
}

impl Default for AopManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience functions for common AOP setups
pub struct AopFactory;

impl AopFactory {
    /// Create an AOP manager with logging and error tracking
    pub fn create_with_logging(logger: Arc<dyn crate::logging::traits::Logger>) -> AopManager {
        let mut manager = AopManager::new();
        
        // Add logging aspect
        manager.add_aspect(Arc::new(crate::aop::aspects::LoggingAspect::new(Arc::clone(&logger))));
        
        // Add error tracking aspect
        manager.add_aspect(Arc::new(crate::aop::aspects::ErrorTrackingAspect::new(Arc::clone(&logger))));
        
        manager
    }

    /// Create an AOP manager with performance monitoring
    pub fn create_with_performance(logger: Arc<dyn crate::logging::traits::Logger>) -> AopManager {
        let mut manager = AopManager::new();
        
        // Add performance aspect
        manager.add_aspect(Arc::new(crate::aop::aspects::PerformanceAspect::new(Arc::clone(&logger))));
        
        manager
    }

    /// Create a full-featured AOP manager
    pub fn create_full(logger: Arc<dyn crate::logging::traits::Logger>) -> AopManager {
        let mut manager = AopManager::new();
        
        manager.add_aspect(Arc::new(crate::aop::aspects::LoggingAspect::new(Arc::clone(&logger))));
        manager.add_aspect(Arc::new(crate::aop::aspects::ErrorTrackingAspect::new(Arc::clone(&logger))));
        manager.add_aspect(Arc::new(crate::aop::aspects::PerformanceAspect::new(Arc::clone(&logger))));
        
        manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aop::aspects::{LoggingAspect, ErrorTrackingAspect};
    use crate::logging::writers::ConsoleWriter;
    use crate::logging::formatters::TextFormatter;

    #[test]
    fn test_aop_manager_creation() {
        let manager = AopManager::new();
        assert_eq!(manager.aspects.len(), 0);
    }

    #[test]
    fn test_aop_manager_add_aspect() {
        let mut manager = AopManager::new();
        let writer = Arc::new(ConsoleWriter::new());
        let formatter = Arc::new(TextFormatter::detailed());
        let logger: Arc<dyn crate::logging::traits::Logger> = Arc::new(crate::logging::logger::StructuredLogger::new(formatter, writer));
        let aspect = Arc::new(LoggingAspect::new(Arc::clone(&logger)));
        
        manager.add_aspect(aspect.clone());
        assert_eq!(manager.aspects.len(), 1);
        assert_eq!(manager.aspects[0].as_ref() as *const dyn Aspect, aspect.as_ref() as *const dyn Aspect);
    }

    #[tokio::test]
    async fn test_aop_manager_apply_aspects() {
        let writer = Arc::new(ConsoleWriter::new());
        let formatter = Arc::new(TextFormatter::detailed());
        let logger: Arc<dyn crate::logging::traits::Logger> = Arc::new(crate::logging::logger::StructuredLogger::new(formatter, writer));
        let manager = AopFactory::create_with_logging(Arc::clone(&logger));
        
        let result = manager.apply_aspects("test_operation", || {
            Ok(42)
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_aop_factory() {
        let writer = Arc::new(ConsoleWriter::new());
        let formatter = Arc::new(TextFormatter::detailed());
        let logger: Arc<dyn crate::logging::traits::Logger> = Arc::new(crate::logging::logger::StructuredLogger::new(formatter, writer));
        
        let manager = AopFactory::create_with_logging(Arc::clone(&logger));
        assert_eq!(manager.aspects.len(), 2); // Logging + ErrorTracking
        
        let full_manager = AopFactory::create_full(Arc::clone(&logger));
        assert_eq!(full_manager.aspects.len(), 3); // Logging + ErrorTracking + Performance
    }
}
