//! Engine trait definition

use async_trait::async_trait;
use crate::channels::types::ChannelType;
use crate::engine::types::{EngineConfig, EngineStats, EngineState, ProcessingContext};
use crate::engine::events::{EngineEvent, EventCallback, EventSubscription, CloneableEventCallback};
use crate::errors::Result;
use crate::events::Package;
use crate::pools::{Pool, PoolType};
use std::collections::HashMap;
use std::sync::Arc;

/// Engine trait - core orchestration interface
#[async_trait]
pub trait Engine: Send + Sync + std::fmt::Debug {
    /// Get engine configuration
    fn config(&self) -> &EngineConfig;
    
    /// Get engine statistics
    fn stats(&self) -> EngineStats;
    
    /// Get engine state (asynchronous, with proper lock acquisition)
    async fn state(&self) -> EngineState;
    
    /// Try to get engine state without blocking (non-blocking, may return stale data)
    /// This is deprecated in favor of async state() method
    #[deprecated(since = "0.1.1", note = "Use async state() method instead for consistent state")]
    fn try_state(&self) -> EngineState;
    
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
    
    // Pool management
    
    /// Register a pool for a specific pool type
    async fn register_pool(&mut self, pool_type: PoolType, pool: Arc<dyn Pool>) -> Result<()>;
    
    /// Unregister a pool for a specific pool type
    async fn unregister_pool(&mut self, pool_type: PoolType) -> Result<()>;
    
    /// Get all registered pools
    fn get_pools(&self) -> HashMap<PoolType, Arc<dyn Pool>>;
    
    // Event management
    
    /// Emit an event to all subscribers
    async fn emit_event(&self, event: EngineEvent) -> Result<()>;
    
    /// Subscribe to events matching a pattern
    async fn subscribe(&mut self, event_pattern: String, callback: CloneableEventCallback) -> Result<EventSubscription>;
    
    /// Unsubscribe from events
    async fn unsubscribe(&mut self, subscription_id: &str) -> Result<()>;
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

        async fn state(&self) -> EngineState {
            self.state.clone()
        }

        fn try_state(&self) -> EngineState {
            self.state.clone()
        }

        async fn set_config(&mut self, config: EngineConfig) -> Result<()> {
            self.config = config;
            Ok(())
        }

        async fn start(&mut self) -> Result<()> {
            self.running = true;
            self.state = EngineState {
                status: crate::engine::types::EngineStatus::Running,
                last_error: None,
            };
            Ok(())
        }

        async fn stop(&mut self) -> Result<()> {
            self.running = false;
            self.state = EngineState {
                status: crate::engine::types::EngineStatus::Stopped,
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

        async fn register_pool(&mut self, _pool_type: PoolType, _pool: Arc<dyn Pool>) -> Result<()> {
            Ok(())
        }

        async fn unregister_pool(&mut self, _pool_type: PoolType) -> Result<()> {
            Ok(())
        }

        fn get_pools(&self) -> HashMap<PoolType, Arc<dyn Pool>> {
            HashMap::new()
        }

        async fn emit_event(&self, _event: EngineEvent) -> Result<()> {
            Ok(())
        }

        async fn subscribe(&mut self, _event_pattern: String, _callback: CloneableEventCallback) -> Result<EventSubscription> {
            Ok(EventSubscription {
                id: "test-subscription".to_string(),
                event_pattern: "*".to_string(),
                callback: CloneableEventCallback::new(MockCallback),
            })
        }

        async fn unsubscribe(&mut self, _subscription_id: &str) -> Result<()> {
            Ok(())
        }
    }
    
    #[derive(Debug, Clone)]
    struct MockCallback;
    
    #[async_trait]
    impl EventCallback for MockCallback {
        async fn handle(&self, _event: EngineEvent) {
        }
    }

    #[test]
    fn test_mock_engine() {
        let engine = MockEngine {
            config: EngineConfig::new(),
            stats: EngineStats::new(),
            state: EngineState {
                status: crate::engine::types::EngineStatus::Stopped,
                last_error: None,
            },
            running: false,
        };

        assert!(!engine.is_running());
        assert_eq!(engine.config().auto_initialize, true);
    }
}
