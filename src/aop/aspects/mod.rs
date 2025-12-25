//! Concrete aspect implementations

pub mod error_tracking;
pub mod logging;
pub mod performance;

pub use error_tracking::*;
pub use logging::*;
pub use performance::*;

use crate::aop::traits::Aspect;
use async_trait::async_trait;

/// Base aspect with common functionality
pub struct BaseAspect {
    name: &'static str,
    enabled: bool,
}

impl BaseAspect {
    /// Create a new base aspect
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            enabled: true,
        }
    }

    /// Create a disabled aspect
    pub fn disabled(name: &'static str) -> Self {
        Self {
            name,
            enabled: false,
        }
    }

    /// Enable or disable the aspect
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if aspect is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Check if aspect should run for given operation
    pub fn should_run(&self, operation: &str) -> bool {
        self.enabled && self.applies_to(operation)
    }

    /// Get aspect name
    pub fn name(&self) -> &'static str {
        self.name
    }
}

#[async_trait]
impl Aspect for BaseAspect {
    async fn before(&self, _operation: &str) -> crate::aop::traits::AopResult<()> {
        Ok(())
    }

    async fn after(&self, _operation: &str, _result: &crate::aop::traits::AopResult<()>) -> crate::aop::traits::AopResult<()> {
        Ok(())
    }

    async fn on_error(&self, _operation: &str, _error: &crate::errors::AopError) -> crate::aop::traits::AopResult<()> {
        Ok(())
    }

    fn applies_to(&self, _operation: &str) -> bool {
        true // Base aspect applies to all operations by default
    }
}

impl std::fmt::Debug for BaseAspect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BaseAspect")
            .field("name", &self.name)
            .field("enabled", &self.enabled)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_base_aspect() {
        let mut aspect = BaseAspect::new("TestAspect");
        
        assert_eq!(aspect.name(), "TestAspect");
        assert!(aspect.is_enabled());
        assert!(aspect.should_run("any_operation"));
        
        aspect.set_enabled(false);
        assert!(!aspect.is_enabled());
        assert!(!aspect.should_run("any_operation"));
    }

    #[tokio::test]
    async fn test_base_aspect_applies_to() {
        let aspect = BaseAspect::new("TestAspect");
        assert!(aspect.applies_to("any_operation"));
    }
}
