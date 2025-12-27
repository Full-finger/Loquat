//! Console Adapter Factory

use crate::adapters::{
    Adapter, AdapterConfig, AdapterFactory,
};
use crate::errors::Result;
use super::console_adapter::ConsoleAdapter;

/// Factory for creating ConsoleAdapter instances
pub struct ConsoleAdapterFactory;

impl AdapterFactory for ConsoleAdapterFactory {
    fn adapter_type(&self) -> &str {
        "console"
    }

    fn create(&self, config: AdapterConfig) -> Result<Box<dyn Adapter>> {
        Ok(Box::new(ConsoleAdapter::new(config)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_factory_type() {
        let factory = ConsoleAdapterFactory;
        assert_eq!(factory.adapter_type(), "console");
    }

    #[test]
    fn test_console_factory_create() {
        let factory = ConsoleAdapterFactory;
        let config = AdapterConfig::new("console", "console-001", "stdio://");
        
        let result = factory.create(config);
        assert!(result.is_ok());
        
        let adapter = result.unwrap();
        assert_eq!(adapter.name(), "ConsoleAdapter");
        assert_eq!(adapter.adapter_id(), "console-001");
    }
}
