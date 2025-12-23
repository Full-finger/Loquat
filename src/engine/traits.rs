//! Engine trait definition

use async_trait::async_trait;
use crate::channels::types::ChannelType;
use crate::engine::types::{EngineConfig, EngineStats, EngineState, ProcessingContext};
use crate::errors::Result;
use crate::events::Package;
use std::sync::Arc;

/// Engine trait - core orchestration interface
#[async_trait]
pub trait Engine: Send + Sync + std::fmt::Debug {
    /// Get engine configuration
    fn config(&self) -> &EngineConfig;
    
    /// Get engine statistics
    fn stats(&self) -> EngineStats;
    
    /// Get engine state (synchronous clone)
    fn state(&self) -> EngineState;
    
    /// Set engine configuration
    async fn set_config(&mut self, config: EngineConfig) -> Result<()>;
    
    /// Start
    async fn start(&mut self) -> Result<()>;
    
    /// Stop
    async fn stop(&mut self) -> Result<()>;
    
    /// Process a package
    async fn process(&mut self, package: Package) -> Result<Package>;
    
    /// Get a channel by type
    async fn get_channel(&self, channel_type: &ChannelType) -> Result<Option<Arc<dyn crate::streams::Stream>>>;
    
    /// Check if engine is running
    fn is_running(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MockEngine {
        config: EngineConfig,
        stats: EngineStats,
        state: EngineState,
        running: bool,
    }

    #[async_trait]
    impl Engine for MockEngine {
        fn config(&self) -> &EngineConfig {
            &self.config
        }

        fn stats(&self) -> EngineStats {
            self.stats.clone()
        }

        fn state(&self) -> EngineState {
            self.state.clone()
        }

        async fn set_config(&mut self, config: EngineConfig) -> Result<()> {
            self.config = config;
            Ok(())
        }

        async fn start(&mut self) -> Result<()> {
            self.running = true;
            self.state = EngineState {
                status: crate::engine::types::EngineStatus::Processing,
                last_error: None,
            };
            Ok(())
        }

        async fn stop(&mut self) -> Result<()> {
            self.running = false;
            self.state = EngineState {
                status: crate::engine::types::EngineStatus::Idle,
                last_error: None,
            };
            Ok(())
        }

        async fn process(&mut self, package: Package) -> Result<Package> {
            if self.running {
                Ok(package)
            } else {
                Err(crate::errors::LoquatError::Unknown("Engine not running".to_string()).into())
            }
        }

        async fn get_channel(&self, _channel_type: &ChannelType) -> Result<Option<Arc<dyn crate::streams::Stream>>> {
            Ok(None)
        }

        fn is_running(&self) -> bool {
            self.running
        }
    }

    #[test]
    fn test_mock_engine() {
        let engine = MockEngine {
            config: EngineConfig::new(),
            stats: EngineStats::new(),
            state: EngineState {
                status: crate::engine::types::EngineStatus::Idle,
                last_error: None,
            },
            running: false,
        };

        assert!(!engine.is_running());
        assert_eq!(engine.config().auto_initialize, true);
    }
}
