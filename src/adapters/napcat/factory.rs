//! NapCat Adapter Factory

use crate::adapters::core::{
    Adapter, AdapterConfig, AdapterFactory,
};
use crate::errors::Result;
use super::adapter::NapCatAdapter;

/// Factory for creating NapCatAdapter instances
pub struct NapCatAdapterFactory;

impl AdapterFactory for NapCatAdapterFactory {
    fn adapter_type(&self) -> &str {
        "napcat"
    }

    fn create(&self, config: AdapterConfig) -> Result<Box<dyn Adapter>> {
        Ok(Box::new(NapCatAdapter::new(config)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_napcat_factory_type() {
        let factory = NapCatAdapterFactory;
        assert_eq!(factory.adapter_type(), "napcat");
    }

    #[test]
    fn test_napcat_factory_create() {
        let factory = NapCatAdapterFactory;
        let config = AdapterConfig::new("napcat", "napcat-001", "ws://localhost:3001");
        
        let result = factory.create(config);
        assert!(result.is_ok());
        
        let adapter = result.unwrap();
        assert_eq!(adapter.name(), "NapCatAdapter");
        assert_eq!(adapter.adapter_id(), "napcat-001");
    }
}
