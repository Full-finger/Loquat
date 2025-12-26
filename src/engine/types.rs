//! Engine type definitions

use crate::channels::types::ChannelType;
use crate::events::Package;
use crate::logging::traits::LogLevel;
use crate::routers::types::RouteTarget;
use serde::{Deserialize, Serialize};

/// Engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    /// Auto-initialize packages
    pub auto_initialize: bool,
    
    /// Auto-route packages
    pub auto_route: bool,
    
    /// Auto-create channels
    pub auto_create_channels: bool,
    
    /// Enable statistics
    pub enable_stats: bool,
    
    /// Log level
    pub log_level: String,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            auto_initialize: true,
            auto_route: true,
            auto_create_channels: true,
            enable_stats: true,
            log_level: "info".to_string(),
        }
    }
}

impl EngineConfig {
    /// Create a new engine config
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set auto-initialize
    pub fn with_auto_initialize(mut self, enabled: bool) -> Self {
        self.auto_initialize = enabled;
        self
    }
    
    /// Set auto-route
    pub fn with_auto_route(mut self, enabled: bool) -> Self {
        self.auto_route = enabled;
        self
    }
    
    /// Set auto-create channels
    pub fn with_auto_create_channels(mut self, enabled: bool) -> Self {
        self.auto_create_channels = enabled;
        self
    }
    
    /// Set enable stats
    pub fn with_enable_stats(mut self, enabled: bool) -> Self {
        self.enable_stats = enabled;
        self
    }
    
    /// Set log level
    pub fn with_log_level(mut self, level: &str) -> Self {
        self.log_level = level.to_string();
        self
    }
}

/// Engine statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EngineStats {
    /// Total packages processed
    pub total_packages: usize,
    
    /// Total packages successfully processed
    pub successful_packages: usize,
    
    /// Total packages failed
    pub failed_packages: usize,
    
    /// Total channels created
    pub total_channels_created: usize,
    
    /// Current active channels
    pub active_channels: usize,
    
    /// Average processing time (ms)
    pub avg_processing_time_ms: u64,
}

impl EngineStats {
    /// Create new engine stats
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Record a package processing
    pub fn record_package(&mut self, success: bool) {
        self.total_packages += 1;
        if success {
            self.successful_packages += 1;
        } else {
            self.failed_packages += 1;
        }
    }
    
    /// Record a channel creation
    pub fn record_channel(&mut self) {
        self.total_channels_created += 1;
    }
    
    /// Update average processing time
    pub fn update_avg_time(&mut self, time_ms: u64) {
        let n = self.total_packages as u64;
        if n > 0 {
            let current_avg = self.avg_processing_time_ms;
            self.avg_processing_time_ms = (current_avg * (n - 1) + time_ms) / n;
        }
    }
}

/// Engine state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineState {
    /// Current status
    pub status: EngineStatus,
    
    /// Last error message
    pub last_error: Option<String>,
}

/// Engine status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineStatus {
    /// Engine is idle
    Idle,
    /// Engine is processing
    Processing,
    /// Engine is stopped
    Stopped,
    /// Engine has error
    Error,
}

impl EngineStatus {
    /// Check if engine is running
    pub fn is_running(&self) -> bool {
        matches!(self, EngineStatus::Processing)
    }
}

/// Processing context
#[derive(Debug, Clone)]
pub struct ProcessingContext {
    /// Channel type
    pub channel_type: Option<ChannelType>,
    
    /// Route target
    pub route_target: Option<RouteTarget>,
}

impl ProcessingContext {
    /// Create a new processing context
    pub fn new() -> Self {
        Self {
            channel_type: None,
            route_target: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_config_default() {
        let config = EngineConfig::new();
        assert!(config.auto_initialize);
        assert!(config.auto_route);
        assert!(config.auto_create_channels);
    }

    #[test]
    fn test_engine_config_builder() {
        let config = EngineConfig::new()
            .with_auto_initialize(false)
            .with_auto_route(false)
            .with_auto_create_channels(false)
            .with_enable_stats(false)
            .with_log_level("debug");

        assert!(!config.auto_initialize);
        assert!(!config.auto_route);
        assert!(!config.auto_create_channels);
        assert!(!config.enable_stats);
        assert_eq!(config.log_level, "debug");
    }

    #[test]
    fn test_engine_stats() {
        let mut stats = EngineStats::new();
        
        stats.record_package(true);
        stats.record_channel();
        assert_eq!(stats.total_packages, 1);
        assert_eq!(stats.successful_packages, 1);
        assert_eq!(stats.total_channels_created, 1);
        
        stats.record_package(true);
        stats.update_avg_time(100);
        stats.update_avg_time(200);
        assert_eq!(stats.total_packages, 2);
        // First update: avg = (0 * 0 + 100) / 1 = 100
        // Second update: avg = (100 * 1 + 200) / 2 = 125
        assert_eq!(stats.avg_processing_time_ms, 125);
    }

    #[test]
    fn test_engine_status() {
        assert!(EngineStatus::Idle.is_running() == false);
        assert!(EngineStatus::Processing.is_running() == true);
        assert!(EngineStatus::Stopped.is_running() == false);
        assert!(EngineStatus::Error.is_running() == false);
    }

    #[test]
    fn test_processing_context() {
        let context = ProcessingContext::new();
        assert!(context.channel_type.is_none());
        assert!(context.route_target.is_none());
    }
}
