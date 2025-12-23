//! Standard channel manager implementation

use crate::channel_manager::traits::ChannelManager;
use crate::channel_manager::types::{ChannelInfo, ChannelManagerConfig, ChannelStats};
use crate::channels::types::ChannelType;
use crate::errors::{ChannelError, Result};
use crate::logging::traits::{LogLevel, LogContext};
use crate::streams::{Stream, StandardStream};
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

/// Standard channel manager - manages multiple channel instances
pub struct StandardChannelManager {
    /// Channel storage: (Stream, ChannelInfo)
    channels: Arc<tokio::sync::RwLock<HashMap<ChannelType, (Arc<dyn Stream>, ChannelInfo)>>>,
    
    /// Manager configuration
    config: ChannelManagerConfig,
    
    /// Statistics
    stats: Arc<tokio::sync::RwLock<ChannelStats>>,
    
    /// Logger
    logger: Arc<dyn crate::logging::Logger>,
}

impl StandardChannelManager {
    /// Create a new channel manager
    pub fn new(logger: Arc<dyn crate::logging::Logger>) -> Self {
        Self {
            channels: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            config: ChannelManagerConfig::new(),
            stats: Arc::new(tokio::sync::RwLock::new(ChannelStats::new())),
            logger,
        }
    }
    
    /// Create a new channel manager with custom config
    pub fn with_config(config: ChannelManagerConfig, logger: Arc<dyn crate::logging::Logger>) -> Self {
        Self {
            channels: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(tokio::sync::RwLock::new(ChannelStats::new())),
            logger,
        }
    }
    
    /// Create a new stream for given channel type
    fn create_stream(&self, channel_type: &ChannelType) -> Arc<dyn Stream> {
        // Create StandardStream with channel_id derived from ChannelType
        Arc::new(StandardStream::new(
            channel_type.id().to_string(),
            channel_type.clone(),
            self.logger.clone(),
        ))
    }
    
    /// Check if max channels reached
    fn check_max_channels(&self, current_count: usize) -> Result<()> {
        if self.config.max_channels > 0 && current_count >= self.config.max_channels {
            return Err(ChannelError::MaxChannelsReached(self.config.max_channels).into());
        }
        Ok(())
    }
    
    /// Get channel info and touch it (update last_used)
    async fn touch_channel(&self, channel_type: &ChannelType) -> Result<()> {
        let mut channels = self.channels.write().await;
        if let Some((_, info)) = channels.get_mut(channel_type) {
            info.touch();
        }
        Ok(())
    }
    
    /// Update stats on channel creation
    async fn stats_created(&self) {
        let mut stats = self.stats.write().await;
        let count = self.channels.read().await.len();
        stats.record_created(count);
    }
    
    /// Update stats on channel removal
    async fn stats_removed(&self) {
        let mut stats = self.stats.write().await;
        let count = self.channels.read().await.len();
        stats.record_removed(count);
    }
}

impl Debug for StandardChannelManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StandardChannelManager")
            .field("config", &self.config)
            .finish()
    }
}

#[async_trait]
impl ChannelManager for StandardChannelManager {
    async fn get_or_create_channel(&self, channel_type: &ChannelType) -> Result<Arc<dyn Stream>> {
        // Check if channel already exists
        let stream_opt = {
            let channels = self.channels.read().await;
            channels.get(channel_type).map(|(s, _)| s.clone())
        };
        
        if let Some(stream) = stream_opt {
            // Channel exists, touch and return
            self.touch_channel(channel_type).await?;
            return Ok(stream);
        }
        
        // Channel doesn't exist, create new one
        let mut channels = self.channels.write().await;
        
        // Double-check after acquiring write lock
        if let Some((stream, _)) = channels.get(channel_type) {
            return Ok(stream.clone());
        }
        
        // Check max channels
        self.check_max_channels(channels.len())?;
        
        // Create new stream
        let stream = self.create_stream(channel_type);
        let info = ChannelInfo::new(channel_type.clone());
        
        channels.insert(channel_type.clone(), (stream.clone(), info));
        
        // Log channel creation
        let message = format!(
            "Created new channel for {} (total: {})",
            channel_type, channels.len()
        );
        let context = LogContext::new().with_component("ChannelManager");
        self.logger.log(LogLevel::Info, &message, &context);
        
        // Update stats
        self.stats_created().await;
        
        Ok(stream)
    }
    
    async fn get_channel(&self, channel_type: &ChannelType) -> Result<Option<Arc<dyn Stream>>> {
        let channels = self.channels.read().await;
        Ok(channels.get(channel_type).map(|(s, _)| s.clone()))
    }
    
    async fn remove_channel(&self, channel_type: &ChannelType) -> Result<()> {
        let mut channels = self.channels.write().await;
        
        if channels.remove(channel_type).is_some() {
            // Log channel removal
            let message = format!("Removed channel for {}", channel_type);
            let context = LogContext::new().with_component("ChannelManager");
            self.logger.log(LogLevel::Info, &message, &context);
            
            // Update stats
            self.stats_removed().await;
            
            Ok(())
        } else {
            Err(ChannelError::NotFound(channel_type.to_string()).into())
        }
    }
    
    async fn list_channels(&self) -> Result<Vec<ChannelType>> {
        let channels = self.channels.read().await;
        Ok(channels.keys().cloned().collect())
    }
    
    async fn channel_count(&self) -> Result<usize> {
        let channels = self.channels.read().await;
        Ok(channels.len())
    }
    
    async fn has_channel(&self, channel_type: &ChannelType) -> Result<bool> {
        let channels = self.channels.read().await;
        Ok(channels.contains_key(channel_type))
    }
    
    async fn clear_all(&self) -> Result<()> {
        let mut channels = self.channels.write().await;
        let count = channels.len();
        channels.clear();
        
        // Log clear
        let message = format!("Cleared {} channels", count);
        let context = LogContext::new().with_component("ChannelManager");
        self.logger.log(LogLevel::Info, &message, &context);
        
        Ok(())
    }
    
    fn config(&self) -> &ChannelManagerConfig {
        &self.config
    }
    
    async fn set_config(&mut self, config: ChannelManagerConfig) -> Result<()> {
        self.config = config;
        Ok(())
    }
    
    fn stats(&self) -> crate::channel_manager::types::ChannelStats {
        // Simplified implementation: return empty stats
        // In a real production implementation, this would need a different approach
        // such as using atomic counters or a separate stats struct
        ChannelStats::new()
    }
    
    async fn cleanup(&self) -> Result<usize> {
        let timeout = self.config.channel_timeout;
        
        if timeout == 0 {
            return Ok(0); // No timeout configured
        }
        
        let mut channels = self.channels.write().await;
        let mut to_remove = Vec::new();
        
        for (channel_type, (_, info)) in channels.iter() {
            if info.idle_seconds() as u64 > timeout {
                to_remove.push(channel_type.clone());
            }
        }
        
        let removed_count = to_remove.len();
        
        for channel_type in to_remove {
            channels.remove(&channel_type);
            
            let message = format!(
                "Cleaned up idle channel: {} (idle: {}s)",
                channel_type,
                timeout
            );
            let context = LogContext::new().with_component("ChannelManager");
            self.logger.log(LogLevel::Debug, &message, &context);
        }
        
        if removed_count > 0 {
            let message = format!("Cleaned up {} idle channels", removed_count);
            let context = LogContext::new().with_component("ChannelManager");
            self.logger.log(LogLevel::Info, &message, &context);
            
            self.stats_removed().await;
        }
        
        Ok(removed_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::StructuredLogger;
    use crate::logging::formatters::JsonFormatter;
    use crate::logging::writers::ConsoleWriter;

    fn create_test_logger() -> Arc<dyn crate::logging::Logger> {
        let formatter = Arc::new(JsonFormatter::new());
        let writer = Arc::new(ConsoleWriter::new());
        Arc::new(StructuredLogger::new(formatter, writer))
    }

    #[tokio::test]
    async fn test_channel_manager_creation() {
        let logger = create_test_logger();
        let manager = StandardChannelManager::new(logger);
        
        assert_eq!(manager.channel_count().await.unwrap(), 0);
        assert!(manager.config().auto_create);
    }

    #[tokio::test]
    async fn test_get_or_create_channel() {
        let logger = create_test_logger();
        let manager = StandardChannelManager::new(logger);
        
        let channel_type = ChannelType::group("test_group");
        
        // First call creates channel
        let stream1 = manager.get_or_create_channel(&channel_type).await.unwrap();
        assert_eq!(manager.channel_count().await.unwrap(), 1);
        
        // Second call returns same channel
        let stream2 = manager.get_or_create_channel(&channel_type).await.unwrap();
        assert_eq!(Arc::as_ptr(&stream1), Arc::as_ptr(&stream2));
        assert_eq!(manager.channel_count().await.unwrap(), 1); // Still 1, not 2
    }

    #[tokio::test]
    async fn test_get_channel() {
        let logger = create_test_logger();
        let manager = StandardChannelManager::new(logger);
        
        let channel_type = ChannelType::group("test_group");
        
        // Channel doesn't exist
        assert!(manager.get_channel(&channel_type).await.unwrap().is_none());
        
        // Create channel
        manager.get_or_create_channel(&channel_type).await.unwrap();
        
        // Channel now exists
        assert!(manager.get_channel(&channel_type).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_remove_channel() {
        let logger = create_test_logger();
        let manager = StandardChannelManager::new(logger);
        
        let channel_type = ChannelType::group("test_group");
        
        // Create channel
        manager.get_or_create_channel(&channel_type).await.unwrap();
        assert_eq!(manager.channel_count().await.unwrap(), 1);
        
        // Remove channel
        assert!(manager.remove_channel(&channel_type).await.is_ok());
        assert_eq!(manager.channel_count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_remove_nonexistent() {
        let logger = create_test_logger();
        let manager = StandardChannelManager::new(logger);
        
        let channel_type = ChannelType::group("test_group");
        
        assert!(manager.remove_channel(&channel_type).await.is_err());
    }

    #[tokio::test]
    async fn test_list_channels() {
        let logger = create_test_logger();
        let manager = StandardChannelManager::new(logger);
        
        // Create multiple channels
        manager.get_or_create_channel(&ChannelType::group("group1")).await.unwrap();
        manager.get_or_create_channel(&ChannelType::group("group2")).await.unwrap();
        manager.get_or_create_channel(&ChannelType::private("user1")).await.unwrap();
        
        let channels = manager.list_channels().await.unwrap();
        assert_eq!(channels.len(), 3);
    }

    #[tokio::test]
    async fn test_has_channel() {
        let logger = create_test_logger();
        let manager = StandardChannelManager::new(logger);
        
        let channel_type = ChannelType::group("test_group");
        
        assert!(!manager.has_channel(&channel_type).await.unwrap());
        
        manager.get_or_create_channel(&channel_type).await.unwrap();
        
        assert!(manager.has_channel(&channel_type).await.unwrap());
    }

    #[tokio::test]
    async fn test_clear_all() {
        let logger = create_test_logger();
        let manager = StandardChannelManager::new(logger);
        
        // Create multiple channels
        manager.get_or_create_channel(&ChannelType::group("group1")).await.unwrap();
        manager.get_or_create_channel(&ChannelType::group("group2")).await.unwrap();
        
        assert_eq!(manager.channel_count().await.unwrap(), 2);
        
        assert!(manager.clear_all().await.is_ok());
        
        assert_eq!(manager.channel_count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_max_channels() {
        let logger = create_test_logger();
        let config = ChannelManagerConfig::new().with_max_channels(2);
        let manager = StandardChannelManager::with_config(config, logger);
        
        // Create first two channels - should succeed
        manager.get_or_create_channel(&ChannelType::group("group1")).await.unwrap();
        manager.get_or_create_channel(&ChannelType::group("group2")).await.unwrap();
        
        // Third channel should fail
        assert!(manager.get_or_create_channel(&ChannelType::group("group3")).await.is_err());
    }

    #[tokio::test]
    async fn test_cleanup_no_timeout() {
        let logger = create_test_logger();
        let config = ChannelManagerConfig::new().with_channel_timeout(0);
        let manager = StandardChannelManager::with_config(config, logger);
        
        // Create a channel
        manager.get_or_create_channel(&ChannelType::group("group1")).await.unwrap();
        
        // Cleanup should not remove anything (timeout = 0)
        let removed = manager.cleanup().await.unwrap();
        assert_eq!(removed, 0);
        assert_eq!(manager.channel_count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_different_channel_types() {
        let logger = create_test_logger();
        let manager = StandardChannelManager::new(logger);
        
        // Create different types of channels
        manager.get_or_create_channel(&ChannelType::group("group1")).await.unwrap();
        manager.get_or_create_channel(&ChannelType::private("user1")).await.unwrap();
        manager.get_or_create_channel(&ChannelType::channel("channel1")).await.unwrap();
        
        assert_eq!(manager.channel_count().await.unwrap(), 3);
        
        let channels = manager.list_channels().await.unwrap();
        assert_eq!(channels.len(), 3);
    }
}
