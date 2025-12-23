//! AOP Proxy implementation for applying aspects to objects

use crate::aop::traits::{Aspect, Proxyable};
use crate::errors::{AopError, LoquatError, Result};
use std::sync::Arc;

/// AOP Proxy that wraps a target object and applies aspects to its operations
#[derive(Debug)]
pub struct AopProxy<T> {
    target: T,
    aspects: Vec<Arc<dyn Aspect>>,
}

impl<T> AopProxy<T> {
    /// Create a new AOP proxy with the given target and aspects
    pub fn new(target: T, aspects: Vec<Arc<dyn Aspect>>) -> Self {
        Self { target, aspects }
    }

    /// Get a reference to the target object
    pub fn target(&self) -> &T {
        &self.target
    }

    /// Get a mutable reference to the target object
    pub fn target_mut(&mut self) -> &mut T {
        &mut self.target
    }

    /// Add an aspect to the proxy
    pub fn add_aspect(&mut self, aspect: Arc<dyn Aspect>) {
        self.aspects.push(aspect);
    }

    /// Get the aspects applied to this proxy
    pub fn aspects(&self) -> &[Arc<dyn Aspect>] {
        &self.aspects
    }

    /// Execute an operation with all aspects applied
    pub async fn execute_with_aspects<F, R>(&self, operation: &str, f: F) -> Result<R>
    where
        F: FnOnce(&T) -> Result<R> + Send,
    {
        // Execute before advice for all aspects
        for aspect in &self.aspects {
            aspect.before(operation).await?;
        }

        // Execute the target function
        let result = f(&self.target);

        // Execute after advice for all aspects
        for aspect in &self.aspects {
            let unit_result: Result<()> = result.as_ref().map(|_| ()).map_err(|e| LoquatError::Aop(AopError::ExecutionFailed(e.to_string())));
            aspect.after(operation, &unit_result).await?;
        }

        result
    }

    /// Execute an async operation with all aspects applied
    pub async fn execute_async_with_aspects<F, Fut, R>(&self, operation: &str, f: F) -> Result<R>
    where
        F: FnOnce(&T) -> Fut + Send,
        Fut: std::future::Future<Output = Result<R>> + Send,
    {
        // Execute before advice for all aspects
        for aspect in &self.aspects {
            aspect.before(operation).await?;
        }

        // Execute the target async function
        let result = f(&self.target).await;

        // Execute after advice for all aspects
        for aspect in &self.aspects {
            let unit_result: Result<()> = result.as_ref().map(|_| ()).map_err(|e| LoquatError::Aop(AopError::ExecutionFailed(e.to_string())));
            aspect.after(operation, &unit_result).await?;
        }

        result
    }

    /// Execute a mutable operation with all aspects applied
    pub async fn execute_mut_with_aspects<F, R>(&mut self, operation: &str, f: F) -> Result<R>
    where
        F: FnOnce(&mut T) -> Result<R> + Send,
    {
        // Execute before advice for all aspects
        for aspect in &self.aspects {
            aspect.before(operation).await?;
        }

        // Execute the target function
        let result = f(&mut self.target);

        // Execute after advice for all aspects
        for aspect in &self.aspects {
            let unit_result: Result<()> = result.as_ref().map(|_| ()).map_err(|e| LoquatError::Aop(AopError::ExecutionFailed(e.to_string())));
            aspect.after(operation, &unit_result).await?;
        }

        result
    }
}

impl<T: Clone> Clone for AopProxy<T> {
    fn clone(&self) -> Self {
        Self {
            target: self.target.clone(),
            aspects: self.aspects.clone(),
        }
    }
}

impl<T> Proxyable for AopProxy<T>
where
    T: Send + Sync,
{
    type Output = T;

    fn execute_with_aspects<F, R>(&self, aspects: &[Arc<dyn Aspect>], operation: &str, f: F) -> Result<R>
    where
        F: FnOnce() -> Result<R> + Send,
    {
        // For simplicity, we'll use a blocking approach here
        // In a real implementation, you might want to handle async differently
        let future = async {
            // Execute before advice for all aspects
            for aspect in aspects {
                aspect.before(operation).await?;
            }

            // Execute the target function
            let result = f();

            // Execute after advice for all aspects
            for aspect in aspects {
                let unit_result: Result<()> = result.as_ref().map(|_| ()).map_err(|e| LoquatError::Aop(AopError::ExecutionFailed(e.to_string())));
                aspect.after(operation, &unit_result).await?;
            }

            result
        };

        // Use a simple runtime for blocking execution
        // In production, you'd want to use a proper async runtime
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(future)
        })
    }

    fn target(&self) -> &Self::Output {
        &self.target
    }
}

/// Builder for creating AOP proxies
pub struct AopProxyBuilder<T> {
    target: Option<T>,
    aspects: Vec<Arc<dyn Aspect>>,
}

impl<T> AopProxyBuilder<T> {
    /// Create a new AOP proxy builder
    pub fn new() -> Self {
        Self {
            target: None,
            aspects: Vec::new(),
        }
    }

    /// Set the target object
    pub fn target(mut self, target: T) -> Self {
        self.target = Some(target);
        self
    }

    /// Add an aspect
    pub fn aspect(mut self, aspect: Arc<dyn Aspect>) -> Self {
        self.aspects.push(aspect);
        self
    }

    /// Add multiple aspects
    pub fn aspects(mut self, aspects: Vec<Arc<dyn Aspect>>) -> Self {
        self.aspects.extend(aspects);
        self
    }

    /// Build the proxy
    pub fn build(self) -> Result<AopProxy<T>> {
        let target = self.target.ok_or_else(|| {
            LoquatError::Aop(AopError::ProxyCreation("Target object is required for AOP proxy".to_string()))
        })?;

        Ok(AopProxy::new(target, self.aspects))
    }
}

impl<T> Default for AopProxyBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience functions for creating common proxy types
pub struct AopProxyFactory;

impl AopProxyFactory {
    /// Create a proxy with logging and error tracking
    pub fn create_with_logging<T>(
        target: T,
        logger: Arc<dyn crate::logging::traits::Logger>,
    ) -> AopProxy<T> {
        let aspects: Vec<Arc<dyn Aspect>> = vec![
            Arc::new(crate::aop::aspects::LoggingAspect::new(Arc::clone(&logger))),
            Arc::new(crate::aop::aspects::ErrorTrackingAspect::new(Arc::clone(&logger))),
        ];
        AopProxy::new(target, aspects)
    }

    /// Create a proxy with performance monitoring
    pub fn create_with_performance<T>(
        target: T,
        logger: Arc<dyn crate::logging::traits::Logger>,
    ) -> AopProxy<T> {
        let aspects: Vec<Arc<dyn Aspect>> = vec![
            Arc::new(crate::aop::aspects::PerformanceAspect::new(Arc::clone(&logger))),
        ];
        AopProxy::new(target, aspects)
    }

    /// Create a full-featured proxy with all aspects
    pub fn create_full<T>(
        target: T,
        logger: Arc<dyn crate::logging::traits::Logger>,
    ) -> AopProxy<T> {
        let aspects: Vec<Arc<dyn Aspect>> = vec![
            Arc::new(crate::aop::aspects::LoggingAspect::new(Arc::clone(&logger))),
            Arc::new(crate::aop::aspects::ErrorTrackingAspect::new(Arc::clone(&logger))),
            Arc::new(crate::aop::aspects::PerformanceAspect::new(Arc::clone(&logger))),
        ];
        AopProxy::new(target, aspects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aop::aspects::LoggingAspect;
    use crate::logging::writers::ConsoleWriter;
    use crate::logging::formatters::TextFormatter;

    #[test]
    fn test_aop_proxy_creation() {
        let target = "test_target";
        let aspects: Vec<Arc<dyn Aspect>> = vec![];
        let proxy = AopProxy::new(target, aspects);

        assert_eq!(proxy.target(), &"test_target");
        assert_eq!(proxy.aspects().len(), 0);
    }

    #[test]
    fn test_aop_proxy_builder() {
        let writer = Arc::new(ConsoleWriter::new());
        let formatter = Arc::new(TextFormatter::detailed());
        let logger: Arc<dyn crate::logging::traits::Logger> = Arc::new(crate::logging::logger::StructuredLogger::new(formatter, writer));

        let aspect = Arc::new(LoggingAspect::new(Arc::clone(&logger)));

        let proxy = AopProxyBuilder::new()
            .target("test_target")
            .aspect(aspect)
            .build()
            .unwrap();

        assert_eq!(proxy.target(), &"test_target");
        assert_eq!(proxy.aspects().len(), 1);
    }

    #[test]
    fn test_aop_proxy_builder_no_target() {
        let result: Result<AopProxy<&str>> = AopProxyBuilder::new().build();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_aop_proxy_execute_with_aspects() {
        let writer = Arc::new(ConsoleWriter::new());
        let formatter = Arc::new(TextFormatter::detailed());
        let logger: Arc<dyn crate::logging::traits::Logger> = Arc::new(crate::logging::logger::StructuredLogger::new(formatter, writer));

        let aspect = Arc::new(LoggingAspect::new(Arc::clone(&logger)));
        let proxy = AopProxy::new("test_target", vec![aspect]);

        let result = proxy.execute_with_aspects("test_operation", |target| {
            assert_eq!(*target, "test_target");
            Ok(42)
        }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_aop_proxy_factory() {
        let writer = Arc::new(ConsoleWriter::new());
        let formatter = Arc::new(TextFormatter::detailed());
        let logger: Arc<dyn crate::logging::traits::Logger> = Arc::new(crate::logging::logger::StructuredLogger::new(formatter, writer));

        let proxy = AopProxyFactory::create_with_logging("test_target", Arc::clone(&logger));
        assert_eq!(proxy.target(), &"test_target");
        assert_eq!(proxy.aspects().len(), 2); // Logging + ErrorTracking

        let full_proxy = AopProxyFactory::create_full("test_target", Arc::clone(&logger));
        assert_eq!(full_proxy.aspects().len(), 3); // Logging + ErrorTracking + Performance
    }
}
