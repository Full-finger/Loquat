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

use crate::errors::Result;

/// AOP manager for coordinating aspects and proxies
pub struct AopManager {
    aspects: Vec<std::sync::Arc<dyn crate::aop::traits::Aspect>>,
}

impl AopManager {
    /// Create a new AOP manager
    pub fn new() -> Self {
        Self {
            aspects: Vec::new(),
        }
    }

    /// Add an aspect to manager (generic version)
    pub fn add_aspect<A: crate::aop::traits::Aspect + Send + Sync + 'static>(&mut self, aspect: std::sync::Arc<A>) {
        self.aspects.push(aspect);
    }

    /// Create a proxy with all registered aspects
    pub fn create_proxy<T>(&self, target: T) -> crate::aop::proxy::AopProxy<T>
    where
        T: Send + Sync,
    {
        crate::aop::proxy::AopProxy::new(target, self.aspects.clone())
    }

    /// Apply aspects to a function
    pub async fn apply_aspects<F, R>(&self, operation: &str, f: F) -> Result<R>
    where
        F: FnOnce() -> Result<R> + Send,
        R: Send,
    {
        // Execute before advice for all aspects
        for aspect in &self.aspects {
            aspect.before(operation).await?;
        }

        // Execute target function
        let result = f();

        // Execute after advice for all aspects
        for aspect in &self.aspects {
            let unit_result: Result<()> = result.as_ref().map(|_| ()).map_err(|e| crate::errors::LoquatError::Aop(crate::errors::AopError::ExecutionFailed(e.to_string())));
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
    /// Create an AOP manager with default aspects
    pub fn create_manager() -> AopManager {
        AopManager::new()
    }

    /// Create an AOP manager with logging
    pub fn create_with_logging(logger: std::sync::Arc<dyn crate::logging::traits::Logger>) -> AopManager {
        let mut manager = AopManager::new();
        manager.add_aspect(std::sync::Arc::new(crate::aop::aspects::LoggingAspect::new(std::sync::Arc::clone(&logger))));
        manager
    }

    /// Create an AOP manager with error tracking
    pub fn create_with_error_tracking(logger: std::sync::Arc<dyn crate::logging::traits::Logger>) -> AopManager {
        let mut manager = AopManager::new();
        manager.add_aspect(std::sync::Arc::new(crate::aop::aspects::ErrorTrackingAspect::new(std::sync::Arc::clone(&logger))));
        manager
    }

    /// Create an AOP manager with performance monitoring
    pub fn create_with_performance(logger: std::sync::Arc<dyn crate::logging::traits::Logger>) -> AopManager {
        let mut manager = AopManager::new();
        manager.add_aspect(std::sync::Arc::new(crate::aop::aspects::PerformanceAspect::new(std::sync::Arc::clone(&logger))));
        manager
    }

    /// Create a full-featured AOP manager with all aspects
    pub fn create_full(logger: std::sync::Arc<dyn crate::logging::traits::Logger>) -> AopManager {
        let mut manager = AopManager::new();
        manager.add_aspect(std::sync::Arc::new(crate::aop::aspects::LoggingAspect::new(std::sync::Arc::clone(&logger))));
        manager.add_aspect(std::sync::Arc::new(crate::aop::aspects::ErrorTrackingAspect::new(std::sync::Arc::clone(&logger))));
        manager.add_aspect(std::sync::Arc::new(crate::aop::aspects::PerformanceAspect::new(std::sync::Arc::clone(&logger))));
        manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_aop_manager_creation() {
        let manager = AopManager::new();
        assert_eq!(manager.aspects.len(), 0);
    }

    #[tokio::test]
    async fn test_aop_manager_add_aspect() {
        let writer = std::sync::Arc::new(crate::logging::writers::ConsoleWriter::new());
        let formatter = std::sync::Arc::new(crate::logging::formatters::TextFormatter::detailed());
        let logger: std::sync::Arc<dyn crate::logging::traits::Logger> = std::sync::Arc::new(crate::logging::logger::StructuredLogger::new(formatter, writer));

        let mut manager = AopManager::new();
        let aspect = std::sync::Arc::new(crate::aop::aspects::LoggingAspect::new(std::sync::Arc::clone(&logger)));
        
        manager.add_aspect(aspect);
        assert_eq!(manager.aspects.len(), 1);
        
        let result = manager.apply_aspects("test_operation", || {
            Ok(42)
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_aop_factory() {
        let writer = std::sync::Arc::new(crate::logging::writers::ConsoleWriter::new());
        let formatter = std::sync::Arc::new(crate::logging::formatters::TextFormatter::detailed());
        let logger: std::sync::Arc<dyn crate::logging::traits::Logger> = std::sync::Arc::new(crate::logging::logger::StructuredLogger::new(formatter, writer));
        
        let manager = AopFactory::create_manager();
        assert_eq!(manager.aspects.len(), 0);
        
        let logging_manager = AopFactory::create_with_logging(std::sync::Arc::clone(&logger));
        assert_eq!(logging_manager.aspects.len(), 1);
        
        let full_manager = AopFactory::create_full(std::sync::Arc::clone(&logger));
        assert_eq!(full_manager.aspects.len(), 3);
    }
}
