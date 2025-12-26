//! Console Adapter Factory

use crate::adapters::{
    Adapter, AdapterConfig, AdapterFactory,
    console_adapter::ConsoleAdapter,
};
use crate::errors::Result;

/// Factory for creating ConsoleAdapter instances
pub struct ConsoleFactory;

impl ConsoleFactory {
    /// Create a new console factory
    pub fn new() -> Self {
        Self
    }
}

impl Default for ConsoleFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl AdapterFactory for ConsoleFactory {
    fn adapter_type(&self) -> &str {
        "console"
    }

    fn create(&self, config: AdapterConfig) -> Result<Box<dyn Adapter>> {
        let adapter = ConsoleAdapter::new(config);
        Ok(Box::new(adapter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_factory_type() {
        let factory = ConsoleFactory::new();
        assert_eq!(factory.adapter_type(), "console");
    }

    #[test]
    fn test_console_factory_create() {
        let factory = ConsoleFactory::new();
        let config = AdapterConfig::new("console", "console-001", "stdio://");
        
        let adapter = factory.create(config).unwrap();
        assert_eq!(adapter.name(), "ConsoleAdapter");
        assert_eq!(adapter.adapter_id(), "console-001");
    }

    #[test]
    fn test_console_factory_validate() {
        let factory = ConsoleFactory::new();
        
        // Valid config
        let config = AdapterConfig::new("console", "console-002", "stdio://");
        assert!(factory.validate_config(config).is_ok());
        
        // Invalid type
        let config = AdapterConfig::new("http", "console-003", "stdio://");
        assert!(factory.validate_config(config).is_err());
        
        // Disabled
        let config = AdapterConfig::new("console", "console-004", "stdio://")
            .with_enabled(false);
        assert!(factory.validate_config(config).is_err());
    }
}
