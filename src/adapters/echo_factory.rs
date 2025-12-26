//! Echo Adapter Factory

use crate::adapters::{
    Adapter, AdapterConfig, AdapterFactory,
    echo_adapter::EchoAdapter,
};
use crate::errors::Result;

/// Factory for creating EchoAdapter instances
pub struct EchoFactory;

impl EchoFactory {
    /// Create a new echo factory
    pub fn new() -> Self {
        Self
    }
}

impl Default for EchoFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl AdapterFactory for EchoFactory {
    fn adapter_type(&self) -> &str {
        "echo"
    }

    fn create(&self, config: AdapterConfig) -> Result<Box<dyn Adapter>> {
        let adapter = EchoAdapter::new(config);
        Ok(Box::new(adapter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_factory_type() {
        let factory = EchoFactory::new();
        assert_eq!(factory.adapter_type(), "echo");
    }

    #[test]
    fn test_echo_factory_create() {
        let factory = EchoFactory::new();
        let config = AdapterConfig::new("echo", "echo-001", "echo://");
        
        let adapter = factory.create(config).unwrap();
        assert_eq!(adapter.name(), "EchoAdapter");
        assert_eq!(adapter.adapter_id(), "echo-001");
    }

    #[test]
    fn test_echo_factory_validate() {
        let factory = EchoFactory::new();
        
        // Valid config
        let config = AdapterConfig::new("echo", "echo-002", "echo://");
        assert!(factory.validate_config(config).is_ok());
        
        // Invalid type
        let config = AdapterConfig::new("console", "echo-003", "echo://");
        assert!(factory.validate_config(config).is_err());
        
        // Disabled
        let config = AdapterConfig::new("echo", "echo-004", "echo://")
            .with_enabled(false);
        assert!(factory.validate_config(config).is_err());
    }
}
