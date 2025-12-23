//! Channel manager trait definition

use async_trait::async_trait;
use crate::channels::types::ChannelType;
use crate::channel_manager::{ChannelStats, ChannelManagerConfig};
use crate::errors::Result;
use std::fmt::Debug;
use std::sync::Arc;

/// Channel manager trait - manages multiple channel instances
#[async_trait]
pub trait ChannelManager: Send + Sync + Debug {
    /// Get or create a channel by identifier
    async fn get_or_create_channel(&self, channel_type: &ChannelType) -> Result<Arc<dyn crate::streams::Stream>>;
    
    /// Get an existing channel (returns None if not exists)
    async fn get_channel(&self, channel_type: &ChannelType) -> Result<Option<Arc<dyn crate::streams::Stream>>>;
    
    /// Remove a channel
    async fn remove_channel(&self, channel_type: &ChannelType) -> Result<()>;
    
    /// Get all channel identifiers
    async fn list_channels(&self) -> Result<Vec<ChannelType>>;
    
    /// Get channel count
    async fn channel_count(&self) -> Result<usize>;
    
    /// Check if channel exists
    async fn has_channel(&self, channel_type: &ChannelType) -> Result<bool>;
    
    /// Clear all channels
    async fn clear_all(&self) -> Result<()>;
    
    /// Get manager configuration
    fn config(&self) -> &ChannelManagerConfig;
    
    /// Update manager configuration
    async fn set_config(&mut self, config: ChannelManagerConfig) -> Result<()>;
    
    /// Get channel statistics
    fn stats(&self) -> ChannelStats;
    
    /// Perform cleanup (remove idle channels)
    async fn cleanup(&self) -> Result<usize>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct MockChannelManager {
        config: ChannelManagerConfig,
        stats: Arc<ChannelStats>,
    }

    #[async_trait]
    impl ChannelManager for MockChannelManager {
        async fn get_or_create_channel(&self, _channel_type: &ChannelType) -> Result<Arc<dyn crate::streams::Stream>> {
            Err(crate::errors::LoquatError::Io("not implemented".to_string()))
        }

        async fn get_channel(&self, _channel_type: &ChannelType) -> Result<Option<Arc<dyn crate::streams::Stream>>> {
            Ok(None)
        }

        async fn remove_channel(&self, _channel_type: &ChannelType) -> Result<()> {
            Ok(())
        }

        async fn list_channels(&self) -> Result<Vec<ChannelType>> {
            Ok(vec![])
        }

        async fn channel_count(&self) -> Result<usize> {
            Ok(0)
        }

        async fn has_channel(&self, _channel_type: &ChannelType) -> Result<bool> {
            Ok(false)
        }

        async fn clear_all(&self) -> Result<()> {
            Ok(())
        }

        fn config(&self) -> &ChannelManagerConfig {
            &self.config
        }

        async fn set_config(&mut self, config: ChannelManagerConfig) -> Result<()> {
            self.config = config;
            Ok(())
        }

        fn stats(&self) -> ChannelStats {
            (*self.stats).clone()
        }

        async fn cleanup(&self) -> Result<usize> {
            Ok(0)
        }
    }

    #[tokio::test]
    async fn test_mock_manager() {
        let manager = MockChannelManager {
            config: ChannelManagerConfig::new(),
            stats: Arc::new(ChannelStats::new()),
        };

        assert_eq!(manager.channel_count().await.unwrap(), 0);
        assert!(!manager.has_channel(&ChannelType::group("test")).await.unwrap());
    }
}
