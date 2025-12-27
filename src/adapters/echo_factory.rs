//! Echo Adapter Factory

use crate::adapters::{
    Adapter, AdapterConfig, AdapterFactory,
};
use crate::errors::Result;
use super::echo_adapter::EchoAdapter;

/// Factory for creating EchoAdapter instances
pub struct EchoAdapterFactory;

impl AdapterFactory for EchoAdapterFactory {
    fn adapter_type(&self) -> &str {
        "echo"
    }

    fn create(&self, config: AdapterConfig) -> Result<Box<dyn Adapter>> {
        Ok(Box::new(EchoAdapter::new(config)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_factory_type() {
        let factory = EchoAdapterFactory;
        assert_eq!(factory.adapter_type(), "echo");
    }

    #[test]
    fn test_echo_factory_create() {
        let factory = EchoAdapterFactory;
        let config = AdapterConfig::new("echo", "echo-001", "echo://");
        
        let result = factory.create(config);
        assert!(result.is_ok());
        
        let adapter = result.unwrap();
        assert_eq!(adapter.name(), "EchoAdapter");
        assert_eq!(adapter.adapter_id(), "echo-001");
    }
}
