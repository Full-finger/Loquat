//! Router trait definition

use crate::events::Package;
use crate::routers::types::{RouteResult, RouterConfig};
use async_trait::async_trait;
use std::fmt::Debug;

/// Router trait - determines routing for packages
#[async_trait]
pub trait Router: Send + Sync + Debug {
    /// Get router ID
    fn router_id(&self) -> &str;
    
    /// Get router configuration
    fn config(&self) -> &RouterConfig;
    
    /// Update router configuration
    fn set_config(&mut self, config: RouterConfig) -> crate::errors::Result<()>;
    
    /// Route a single package
    /// Returns the routing result with state to apply
    async fn route_package(&self, package: &Package) -> RouteResult;
    
    /// Route multiple packages
    async fn route_batch(&self, packages: &[Package]) -> Vec<RouteResult> {
        let mut results = Vec::new();
        for package in packages {
            results.push(self.route_package(package).await);
        }
        results
    }
    
    /// Check if routing is enabled
    fn is_enabled(&self) -> bool {
        self.config().auto_route
    }
    
    /// Enable or disable routing
    fn set_enabled(&mut self, enabled: bool) {
        let mut config = self.config().clone();
        config.auto_route = enabled;
        let _ = self.set_config(config);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routers::types::{RouteState, RouteTarget};

    #[derive(Debug)]
    struct MockRouter {
        id: String,
        config: RouterConfig,
    }

    #[async_trait]
    impl Router for MockRouter {
        fn router_id(&self) -> &str {
            &self.id
        }

        fn config(&self) -> &RouterConfig {
            &self.config
        }

        fn set_config(&mut self, config: RouterConfig) -> crate::errors::Result<()> {
            self.config = config;
            Ok(())
        }

        async fn route_package(&self, _package: &Package) -> RouteResult {
            let state = RouteState::new()
                .with_initialized()
                .with_adapter_target(RouteTarget::Adapter("mock_adapter".to_string()));
            RouteResult::success(state)
        }
    }

    #[test]
    fn test_mock_router() {
        let config = RouterConfig::new();
        let router = MockRouter {
            id: "test_router".to_string(),
            config,
        };

        assert_eq!(router.router_id(), "test_router");
        assert!(router.is_enabled());
    }

    #[test]
    fn test_router_set_enabled() {
        let config = RouterConfig::new().with_auto_initialize(false);
        let mut router = MockRouter {
            id: "test_router".to_string(),
            config,
        };

        router.set_enabled(false);
        assert!(!router.is_enabled());
    }

    #[test]
    fn test_router_set_config() {
        let config = RouterConfig::new();
        let mut router = MockRouter {
            id: "test_router".to_string(),
            config,
        };

        let new_config = RouterConfig::new().with_default_adapter("new_default");
        assert!(router.set_config(new_config.clone()).is_ok());
        assert_eq!(router.config().default_adapter, Some("new_default".to_string()));
    }
}
