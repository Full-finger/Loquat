//! Core traits for Aspect-Oriented Programming

use async_trait::async_trait;
use crate::errors::{AopError, Result};
use std::fmt::Debug;

/// Core aspect trait for AOP functionality
#[async_trait]
pub trait Aspect: Send + Sync + Debug {
    /// Execute before the target method
    async fn before(&self, operation: &str) -> Result<()> {
        Ok(())
    }

    /// Execute after the target method successfully completes
    async fn after(&self, operation: &str, result: &Result<()>) -> Result<()> {
        Ok(())
    }


    /// Handle exceptions/errors from the target method
    async fn on_error(&self, operation: &str, error: &AopError) -> Result<()> {
        // Default: just log the error
        eprintln!("Aspect error in {}: {:?}", operation, error);
        Ok(())
    }

    /// Get the name of this aspect
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Check if this aspect applies to the given operation
    fn applies_to(&self, operation: &str) -> bool {
        true // Default: apply to all operations
    }
}

/// Trait for objects that can be proxied with aspects
pub trait Proxyable: Send + Sync {
    type Output;

    /// Execute with aspect weaving
    fn execute_with_aspects<F, R>(&self, aspects: &[std::sync::Arc<dyn Aspect>], operation: &str, f: F) -> Result<R>
    where
        F: FnOnce() -> Result<R> + Send,
        R: Send;

    /// Get a reference to the underlying target
    fn target(&self) -> &Self::Output;
}

/// Aspect execution context
#[derive(Debug, Clone)]
pub struct AspectContext {
    /// Operation name
    pub operation: String,
    
    /// Component name
    pub component: Option<String>,
    
    /// Execution start time
    pub start_time: chrono::DateTime<chrono::Utc>,
    
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl AspectContext {
    /// Create a new aspect context
    pub fn new(operation: &str) -> Self {
        Self {
            operation: operation.to_string(),
            component: None,
            start_time: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set the component
    pub fn with_component(mut self, component: &str) -> Self {
        self.component = Some(component.to_string());
        self
    }

    /// Add metadata
    pub fn with_metadata<K: Into<String>, V: serde::Serialize>(mut self, key: K, value: V) -> Result<Self> {
        let serialized = serde_json::to_value(value)
            .map_err(|e| AopError::ExecutionFailed(e.to_string()))?;
        self.metadata.insert(key.into(), serialized);
        Ok(self)
    }

    /// Get execution duration
    pub fn duration(&self) -> chrono::Duration {
        chrono::Utc::now() - self.start_time
    }
}

/// Result of aspect execution
#[derive(Debug, Clone)]
pub struct AspectResult {
    /// Context information
    pub context: AspectContext,
    
    /// Whether the operation was successful
    pub success: bool,
    
    /// Error information if failed
    pub error: Option<AopError>,
    
    /// Execution duration
    pub duration: chrono::Duration,
    
    /// Additional data from aspects
    pub data: std::collections::HashMap<String, serde_json::Value>,
}

impl AspectResult {
    /// Create a successful aspect result
    pub fn success(context: AspectContext) -> Self {
        let duration = context.duration();
        Self {
            context,
            success: true,
            error: None,
            duration,
            data: std::collections::HashMap::new(),
        }
    }

    /// Create a failed aspect result
    pub fn failure(context: AspectContext, error: AopError) -> Self {
        let duration = context.duration();
        Self {
            context,
            success: false,
            error: Some(error),
            duration,
            data: std::collections::HashMap::new(),
        }
    }

    /// Add data to the result
    pub fn with_data<K: Into<String>, V: serde::Serialize>(mut self, key: K, value: V) -> Result<Self> {
        let serialized = serde_json::to_value(value)
            .map_err(|e| AopError::ExecutionFailed(e.to_string()))?;
        self.data.insert(key.into(), serialized);
        Ok(self)
    }
}

/// Trait for aspect chains
#[async_trait]
pub trait AspectChain: Send + Sync {
    /// Add an aspect to the chain
    fn add_aspect(&mut self, aspect: std::sync::Arc<dyn Aspect>);
    
    /// Execute the aspect chain
    async fn execute(&self, context: AspectContext) -> Result<AspectResult>;
}

/// Simple aspect chain implementation
#[derive(Debug)]
pub struct SimpleAspectChain {
    aspects: Vec<std::sync::Arc<dyn Aspect>>,
}

impl SimpleAspectChain {
    /// Create a new aspect chain
    pub fn new() -> Self {
        Self {
            aspects: Vec::new(),
        }
    }

    /// Create a chain from a vector of aspects
    pub fn from_vec(aspects: Vec<std::sync::Arc<dyn Aspect>>) -> Self {
        Self { aspects }
    }

    /// Get the number of aspects in the chain
    pub fn len(&self) -> usize {
        self.aspects.len()
    }

    /// Check if the chain is empty
    pub fn is_empty(&self) -> bool {
        self.aspects.is_empty()
    }

    /// Get an aspect by index
    pub fn get(&self, index: usize) -> Option<&std::sync::Arc<dyn Aspect>> {
        self.aspects.get(index)
    }

    /// Remove an aspect by index
    pub fn remove(&mut self, index: usize) -> Option<std::sync::Arc<dyn Aspect>> {
        if index < self.aspects.len() {
            Some(self.aspects.remove(index))
        } else {
            None
        }
    }

    /// Clear all aspects
    pub fn clear(&mut self) {
        self.aspects.clear();
    }
}

impl Default for SimpleAspectChain {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AspectChain for SimpleAspectChain {
    fn add_aspect(&mut self, aspect: std::sync::Arc<dyn Aspect>) {
        self.aspects.push(aspect);
    }

    async fn execute(&self, context: AspectContext) -> Result<AspectResult> {
        let current_context = context.clone();
        
        // Execute before advice for all applicable aspects
        for aspect in &self.aspects {
            if aspect.applies_to(&current_context.operation) {
                aspect.before(&current_context.operation).await?;
            }
        }

        // In a real implementation, this would execute the target function
        // For now, we'll just return success
        let result = AspectResult::success(current_context);

        // Execute after advice for all applicable aspects
        for aspect in &self.aspects {
            if aspect.applies_to(&result.context.operation) {
                aspect.after(&result.context.operation, &Ok(())).await?;
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[derive(Debug)]
    struct TestAspect {
        name: &'static str,
    }

    impl TestAspect {
        fn new(name: &'static str) -> Self {
            Self { name }
        }
    }

    #[async_trait]
    impl Aspect for TestAspect {
        fn name(&self) -> &'static str {
            self.name
        }

        async fn before(&self, operation: &str) -> Result<()> {
            println!("{}: Before {}", self.name, operation);
            Ok(())
        }

        async fn after(&self, operation: &str, _result: &Result<()>) -> Result<()> {
            println!("{}: After {}", self.name, operation);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_aspect_trait() {
        let aspect = TestAspect::new("TestAspect");
        
        assert_eq!(aspect.name(), "TestAspect");
        
        aspect.before("test_operation").await.unwrap();
        aspect.after("test_operation", &Ok(())).await.unwrap();
    }

    #[tokio::test]
    async fn test_aspect_chain() {
        let mut chain = SimpleAspectChain::new();
        let aspect1 = Arc::new(TestAspect::new("Aspect1"));
        let aspect2 = Arc::new(TestAspect::new("Aspect2"));
        
        chain.add_aspect(aspect1.clone());
        chain.add_aspect(aspect2.clone());
        
        assert_eq!(chain.len(), 2);
        
        let context = AspectContext::new("test_operation");
        let result = chain.execute(context).await;
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_aspect_context() {
        let context = AspectContext::new("test_op")
            .with_component("test_component")
            .with_metadata("test_key", "test_value")
            .unwrap();
        
        assert_eq!(context.operation, "test_op");
        assert_eq!(context.component.as_ref().unwrap(), "test_component");
        assert!(context.metadata.contains_key("test_key"));
    }

    #[test]
    fn test_aspect_result() {
        let context = AspectContext::new("test_op");
        let success_result = AspectResult::success(context.clone());
        
        assert!(success_result.success);
        assert!(success_result.error.is_none());
        
        let error = AopError::ExecutionFailed("Test error".to_string());
        let failure_result = AspectResult::failure(context, error);
        
        assert!(!failure_result.success);
        assert!(failure_result.error.is_some());
    }
}
