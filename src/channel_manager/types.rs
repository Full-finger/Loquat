//! Channel manager type definitions

use crate::channels::types::ChannelType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Channel information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    /// Channel type (with identifier)
    pub channel_type: ChannelType,
    
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    
    /// Last used timestamp
    pub last_used: DateTime<Utc>,
}

impl ChannelInfo {
    /// Create a new channel info
    pub fn new(channel_type: ChannelType) -> Self {
        let now = Utc::now();
        Self {
            channel_type,
            created_at: now,
            last_used: now,
        }
    }
    
    /// Update last used timestamp
    pub fn touch(&mut self) {
        self.last_used = Utc::now();
    }
    
    /// Get age in seconds
    pub fn age_seconds(&self) -> i64 {
        (Utc::now() - self.created_at).num_seconds()
    }
    
    /// Get idle time in seconds
    pub fn idle_seconds(&self) -> i64 {
        (Utc::now() - self.last_used).num_seconds()
    }
}

/// Channel manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelManagerConfig {
    /// Maximum number of channels (0 = unlimited)
    pub max_channels: usize,
    
    /// Channel timeout in seconds (0 = no timeout)
    pub channel_timeout: u64,
    
    /// Enable automatic channel creation
    pub auto_create: bool,
    
    /// Cleanup interval in seconds
    pub cleanup_interval: u64,
}

impl Default for ChannelManagerConfig {
    fn default() -> Self {
        Self {
            max_channels: 100,
            channel_timeout: 300, // 5 minutes
            auto_create: true,
            cleanup_interval: 60, // 1 minute
        }
    }
}

impl ChannelManagerConfig {
    /// Create a new channel manager config
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set max channels
    pub fn with_max_channels(mut self, max: usize) -> Self {
        self.max_channels = max;
        self
    }
    
    /// Set channel timeout (seconds)
    pub fn with_channel_timeout(mut self, timeout: u64) -> Self {
        self.channel_timeout = timeout;
        self
    }
    
    /// Enable or disable auto-create
    pub fn with_auto_create(mut self, enabled: bool) -> Self {
        self.auto_create = enabled;
        self
    }
    
    /// Set cleanup interval (seconds)
    pub fn with_cleanup_interval(mut self, interval: u64) -> Self {
        self.cleanup_interval = interval;
        self
    }
}

/// Channel statistics
#[derive(Debug, Clone, Default)]
pub struct ChannelStats {
    /// Total channels created
    pub total_created: usize,
    
    /// Total channels removed
    pub total_removed: usize,
    
    /// Current active channels
    pub active_channels: usize,
    
    /// Peak channels count
    pub peak_channels: usize,
}

impl ChannelStats {
    /// Create new channel stats
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Record channel creation
    pub fn record_created(&mut self, current_count: usize) {
        self.total_created += 1;
        self.active_channels = current_count;
        if current_count > self.peak_channels {
            self.peak_channels = current_count;
        }
    }
    
    /// Record channel removal
    pub fn record_removed(&mut self, current_count: usize) {
        self.total_removed += 1;
        self.active_channels = current_count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::ChannelType;

    #[test]
    fn test_channel_info_creation() {
        let channel_type = ChannelType::group("test_group");
        let info = ChannelInfo::new(channel_type);

        assert_eq!(info.age_seconds(), 0);
        assert_eq!(info.idle_seconds(), 0);
    }

    #[test]
    fn test_channel_info_touch() {
        let mut info = ChannelInfo::new(ChannelType::private("user123"));
        let created_at = info.last_used;
        
        // Simulate some time passing (in real test, would need sleep)
        info.touch();
        
        assert!(info.last_used >= created_at);
    }

    #[test]
    fn test_channel_manager_config_default() {
        let config = ChannelManagerConfig::default();
        assert_eq!(config.max_channels, 100);
        assert_eq!(config.channel_timeout, 300);
        assert!(config.auto_create);
        assert_eq!(config.cleanup_interval, 60);
    }

    #[test]
    fn test_channel_manager_config_builder() {
        let config = ChannelManagerConfig::new()
            .with_max_channels(200)
            .with_channel_timeout(600)
            .with_auto_create(false)
            .with_cleanup_interval(120);

        assert_eq!(config.max_channels, 200);
        assert_eq!(config.channel_timeout, 600);
        assert!(!config.auto_create);
        assert_eq!(config.cleanup_interval, 120);
    }

    #[test]
    fn test_channel_stats() {
        let mut stats = ChannelStats::new();

        stats.record_created(1);
        assert_eq!(stats.total_created, 1);
        assert_eq!(stats.active_channels, 1);
        assert_eq!(stats.peak_channels, 1);

        stats.record_created(2);
        assert_eq!(stats.peak_channels, 2);

        stats.record_removed(1);
        assert_eq!(stats.total_removed, 1);
        assert_eq!(stats.active_channels, 1);
    }
}
