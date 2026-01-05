//! Mock Test Adapter Factory

use crate::adapters::core::factory::AdapterFactory;
use crate::adapters::core::config::AdapterConfig;
use crate::errors::{Result, LoquatError, ConfigError};
use super::adapter::MockTestAdapter;

/// Factory for creating MockTestAdapter instances
pub struct MockTestFactory;

impl AdapterFactory for MockTestFactory {
    fn adapter_type(&self) -> &str {
        "mock_test"
    }

    fn create(&self, config: AdapterConfig) -> Result<Box<dyn crate::adapters::core::Adapter>> {
        // 调用默认的 validate_config
        self.validate_config(config.clone())?;
        
        // 创建 MockTestAdapter 实例
        let adapter = MockTestAdapter::new(config);
        
        Ok(Box::new(adapter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_test_factory_type() {
        let factory = MockTestFactory;
        assert_eq!(factory.adapter_type(), "mock_test");
    }

    #[test]
    fn test_mock_test_factory_create() {
        let factory = MockTestFactory;
        let config = AdapterConfig::new("mock_test", "test-001", "mock://test");
        
        let result = factory.create(config);
        assert!(result.is_ok());
        
        let adapter = result.unwrap();
        assert_eq!(adapter.name(), "MockTestAdapter");
        assert_eq!(adapter.adapter_id(), "test-001");
    }

    #[test]
    fn test_mock_test_factory_validate_invalid_type() {
        let factory = MockTestFactory;
        let config = AdapterConfig::new("console", "test-001", "mock://test");
        
        let result = factory.validate_config(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_test_factory_create_disabled() {
        let factory = MockTestFactory;
        let config = AdapterConfig::new("mock_test", "test-001", "mock://test")
            .with_enabled(false);
        
        let result = factory.validate_config(config);
        assert!(result.is_err());
    }
}
