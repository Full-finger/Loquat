//! Plugin trait definition

use async_trait::async_trait;
use std::fmt::Debug;
use crate::plugins::types::PluginType;

/// Plugin trait - all plugins must implement this
#[async_trait]
pub trait Plugin: Send + Sync + Debug {
    /// Get plugin name
    fn name(&self) -> &str;
    
    /// Get plugin version
    fn version(&self) -> &str;
    
    /// Get plugin type
    fn plugin_type(&self) -> PluginType;
    
    /// Get plugin description
    fn description(&self) -> Option<&str> {
        None
    }
    
    /// Get plugin author
    fn author(&self) -> Option<&str> {
        None
    }
    
    /// Get dependencies (other plugins)
    fn dependencies(&self) -> Vec<String> {
        Vec::new()
    }
    
    /// Initialize plugin
    async fn init(&mut self) -> crate::errors::Result<()> {
        Ok(())
    }
    
    /// Load of plugin (called after init)
    async fn load(&mut self) -> crate::errors::Result<()> {
        Ok(())
    }
    
    /// Unload of plugin
    async fn unload(&mut self) -> crate::errors::Result<()> {
        Ok(())
    }
    
    /// Reload of plugin (hot-reload)
    async fn reload(&mut self) -> crate::errors::Result<()> {
        self.unload().await?;
        self.load().await
    }
    
    /// Check if plugin is ready to handle events
    fn is_ready(&self) -> bool {
        true
    }
    
    /// Handle plugin-specific configuration update
    async fn update_config(&mut self, _config: serde_json::Value) -> crate::errors::Result<()> {
        // Default: ignore config updates
        Ok(())
    }
    
    /// Get plugin health status
    fn health_status(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}

/// Plugin health status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginHealth {
    /// Plugin is healthy
    Healthy,
    /// Plugin is degraded but functional
    Degraded { reason: String },
    /// Plugin is unhealthy
    Unhealthy { reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::types::PluginType;

    #[derive(Debug)]
    struct MockPlugin {
        name: String,
        version: String,
        plugin_type: PluginType,
        initialized: bool,
    }

    #[async_trait]
    impl Plugin for MockPlugin {
        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> &str {
            &self.version
        }

        fn plugin_type(&self) -> PluginType {
            self.plugin_type.clone()
        }

        async fn init(&mut self) -> crate::errors::Result<()> {
            self.initialized = true;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_mock_plugin() {
        let plugin = MockPlugin {
            name: "test_plugin".to_string(),
            version: "1.0.0".to_string(),
            plugin_type: PluginType::Native,
            initialized: false,
        };

        assert_eq!(plugin.name(), "test_plugin");
        assert_eq!(plugin.version(), "1.0.0");
        assert_eq!(plugin.plugin_type(), PluginType::Native);
        assert!(!plugin.initialized);
        assert!(plugin.is_ready());
    }

    #[tokio::test]
    async fn test_plugin_init() {
        let mut plugin = MockPlugin {
            name: "test_plugin".to_string(),
            version: "1.0.0".to_string(),
            plugin_type: PluginType::Native,
            initialized: false,
        };

        assert!(plugin.init().await.is_ok());
        assert!(plugin.initialized);
    }

    #[tokio::test]
    async fn test_plugin_reload() {
        let mut plugin = MockPlugin {
            name: "test_plugin".to_string(),
            version: "1.0.0".to_string(),
            plugin_type: PluginType::Native,
            initialized: true,
        };

        assert!(plugin.reload().await.is_ok());
    }

    #[test]
    fn test_plugin_health() {
        let plugin = MockPlugin {
            name: "test_plugin".to_string(),
            version: "1.0.0".to_string(),
            plugin_type: PluginType::Native,
            initialized: true,
        };

        assert_eq!(plugin.health_status(), PluginHealth::Healthy);
    }
}
