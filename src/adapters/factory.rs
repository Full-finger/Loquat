//! Adapter factory for creating adapter instances

use crate::adapters::config::AdapterConfig;
use crate::errors::{ConfigError, LoquatError, Result};
use std::sync::RwLock;

/// Adapter factory trait - creates adapter instances from configuration
pub trait AdapterFactory: Send + Sync {
    /// Get supported adapter type
    fn adapter_type(&self) -> &str;

    /// Create an adapter instance from configuration
    fn create(&self, config: AdapterConfig) -> Result<Box<dyn crate::adapters::Adapter>>;

    /// Validate adapter configuration
    fn validate_config(&self, config: AdapterConfig) -> Result<()> {
        // Check if adapter type matches
        if config.adapter_type != self.adapter_type() {
            return Err(LoquatError::Config(
                ConfigError::InvalidFormat(
                    format!("Invalid adapter type: expected {}, got {}",
                            self.adapter_type(), config.adapter_type)
                )
            ));
        }

        // Check if adapter is enabled
        if !config.enabled {
            return Err(LoquatError::Config(
                ConfigError::InvalidFormat(
                    format!("Adapter {} is disabled", config.adapter_id)
                )
            ));
        }

        Ok(())
    }
}

/// Adapter factory registry - manages multiple adapter factories
pub struct AdapterFactoryRegistry {
    factories: RwLock<std::collections::HashMap<String, Box<dyn AdapterFactory>>>,
}

impl AdapterFactoryRegistry {
    /// Create a new factory registry
    pub fn new() -> Self {
        Self {
            factories: RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Register a factory for an adapter type
    pub fn register(&self, factory: Box<dyn AdapterFactory>) -> Result<()> {
        let adapter_type = factory.adapter_type().to_string();
        let mut factories = self.factories.write().map_err(|e| {
            LoquatError::Internal(format!("Failed to acquire write lock: {}", e))
        })?;
        factories.insert(adapter_type, factory);
        Ok(())
    }

    /// Unregister a factory for an adapter type
    pub fn unregister(&mut self, adapter_type: &str) -> Option<Box<dyn AdapterFactory>> {
        self.factories.write().unwrap().remove(adapter_type)
    }

    /// Check if an adapter type is registered
    pub fn is_registered(&self, adapter_type: &str) -> bool {
        self.factories.read().unwrap().contains_key(adapter_type)
    }

    /// Get list of registered adapter types
    pub fn registered_types(&self) -> Vec<String> {
        self.factories.read().unwrap().keys().cloned().collect()
    }

    /// Create an adapter from configuration
    pub fn create(&self, config: AdapterConfig) -> Result<Box<dyn crate::adapters::Adapter>> {
        let factories = self.factories.read().map_err(|e| {
            LoquatError::Internal(format!("Failed to acquire read lock: {}", e))
        })?;
        let factory = factories.get(&config.adapter_type)
            .ok_or_else(|| LoquatError::Config(
                ConfigError::InvalidFormat(
                    format!("No factory registered for adapter type: {}", config.adapter_type)
                )
            ))?;

        factory.validate_config(config.clone())?;
        factory.create(config)
    }

    /// Validate an adapter configuration
    pub fn validate_config(&self, config: AdapterConfig) -> Result<()> {
        let factories = self.factories.read().map_err(|e| {
            LoquatError::Internal(format!("Failed to acquire read lock: {}", e))
        })?;
        let factory = factories.get(&config.adapter_type)
            .ok_or_else(|| LoquatError::Config(
                ConfigError::InvalidFormat(
                    format!("No factory registered for adapter type: {}", config.adapter_type)
                )
            ))?;

        factory.validate_config(config)
    }
}

impl Default for AdapterFactoryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock adapter for testing
    #[derive(Debug)]
    struct MockAdapter {
        config: AdapterConfig,
    }

    impl crate::adapters::Adapter for MockAdapter {
        fn name(&self) -> &str {
            "MockAdapter"
        }

        fn version(&self) -> &str {
            "1.0.0"
        }

        fn adapter_id(&self) -> &str {
            &self.config.adapter_id
        }

        fn config(&self) -> crate::adapters::config::AdapterConfig {
            self.config.clone()
        }

        fn status(&self) -> crate::adapters::AdapterStatus {
            crate::adapters::AdapterStatus::Ready
        }

        fn is_running(&self) -> bool {
            matches!(self.status(), crate::adapters::AdapterStatus::Running)
        }

        fn is_connected(&self) -> bool {
            matches!(self.status(), crate::adapters::AdapterStatus::Running | crate::adapters::AdapterStatus::Ready)
        }

        fn statistics(&self) -> crate::adapters::AdapterStatistics {
            crate::adapters::AdapterStatistics::default()
        }
    }

    /// Mock factory for testing
    struct MockFactory;

    impl AdapterFactory for MockFactory {
        fn adapter_type(&self) -> &str {
            "mock"
        }

        fn create(&self, config: AdapterConfig) -> Result<Box<dyn crate::adapters::Adapter>> {
            Ok(Box::new(MockAdapter { config }))
        }

        fn validate_config(&self, _config: AdapterConfig) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_factory_registry_creation() {
        let registry = AdapterFactoryRegistry::new();

        assert!(registry.registered_types().is_empty());
    }

    #[test]
    fn test_factory_registry_register() {
        let mut registry = AdapterFactoryRegistry::new();

        assert!(registry.register(Box::new(MockFactory)).is_ok());
        assert!(registry.is_registered("mock"));
        assert_eq!(registry.registered_types(), vec!["mock".to_string()]);
    }

    #[test]
    fn test_factory_registry_unregister() {
        let mut registry = AdapterFactoryRegistry::new();
        registry.register(Box::new(MockFactory)).unwrap();

        assert!(registry.unregister("mock").is_some());
        assert!(!registry.is_registered("mock"));
    }

    #[test]
    fn test_factory_validate_config_valid() {
        let registry = AdapterFactoryRegistry::new();
        registry.register(Box::new(MockFactory)).unwrap();

        let config = AdapterConfig::new("mock", "test-001", "ws://localhost");
        assert!(registry.validate_config(config).is_ok());
    }

    #[test]
    fn test_factory_validate_config_invalid_type() {
        let registry = AdapterFactoryRegistry::new();
        registry.register(Box::new(MockFactory)).unwrap();

        let config = AdapterConfig::new("unknown", "test-001", "ws://localhost");
        assert!(registry.validate_config(config).is_err());
    }

    #[test]
    fn test_factory_validate_config_disabled() {
        let registry = AdapterFactoryRegistry::new();
        registry.register(Box::new(MockFactory)).unwrap();

        let config = AdapterConfig::new("mock", "test-001", "ws://localhost")
            .with_enabled(false);
        assert!(registry.validate_config(config).is_err());
    }

    #[test]
    fn test_factory_create_adapter() {
        let registry = AdapterFactoryRegistry::new();
        registry.register(Box::new(MockFactory)).unwrap();

        let config = AdapterConfig::new("mock", "test-001", "ws://localhost");
        let adapter = registry.create(config);

        assert!(adapter.is_ok());
        let adapter = adapter.unwrap();
        assert_eq!(adapter.name(), "MockAdapter");
        assert_eq!(adapter.adapter_id(), "test-001");
    }

    #[test]
    fn test_factory_create_adapter_no_factory() {
        let registry = AdapterFactoryRegistry::new();

        let config = AdapterConfig::new("unknown", "test-001", "ws://localhost");
        assert!(registry.validate_config(config).is_err());
    }
}
