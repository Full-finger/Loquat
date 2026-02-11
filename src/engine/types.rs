//! Engine type definitions

use crate::channels::types::ChannelType;
use crate::events::Package;
use crate::logging::traits::LogLevel;
use crate::pools::PoolType;
use crate::routers::types::RouteTarget;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

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
    
    /// Pool enabled/disabled state
    pub pool_enabled: HashMap<PoolType, bool>,
    
    /// Maximum pool flow depth (prevents infinite loops)
    pub max_pool_depth: usize,
    
    /// Enable event system
    pub enable_events: bool,
    
    /// Processing timeout in milliseconds (None = no timeout)
    pub timeout_ms: Option<u64>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            auto_initialize: true,
            auto_route: true,
            auto_create_channels: true,
            enable_stats: true,
            log_level: "info".to_string(),
            pool_enabled: Self::default_pool_enabled(),
            max_pool_depth: 100,
            enable_events: true,
            timeout_ms: None,
        }
    }
}

impl EngineConfig {
    fn default_pool_enabled() -> HashMap<PoolType, bool> {
        let mut map = HashMap::new();
        // Enable all pools by default
        for pool_type in PoolType::processing_order() {
            map.insert(pool_type, true);
        }
        map
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
    
    /// Set pool enabled state
    pub fn with_pool_enabled(mut self, pool_type: PoolType, enabled: bool) -> Self {
        self.pool_enabled.insert(pool_type, enabled);
        self
    }
    
    /// Set maximum pool flow depth
    pub fn with_max_pool_depth(mut self, depth: usize) -> Self {
        self.max_pool_depth = depth;
        self
    }
    
    /// Enable/disable event system
    pub fn with_enable_events(mut self, enabled: bool) -> Self {
        self.enable_events = enabled;
        self
    }
    
    /// Set processing timeout
    pub fn with_timeout_ms(mut self, timeout_ms: Option<u64>) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
    
    /// Check if a pool is enabled
    pub fn is_pool_enabled(&self, pool_type: &PoolType) -> bool {
        *self.pool_enabled.get(pool_type).unwrap_or(&false)
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
    
    /// Active packages per pool type
    pub pools_active: HashMap<PoolType, usize>,
    
    /// Total events emitted
    pub events_emitted: usize,
    
    /// Error count by type
    pub errors_by_type: HashMap<String, usize>,
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
    
    /// Record pool activity
    pub fn record_pool_activity(&mut self, pool_type: PoolType) {
        *self.pools_active.entry(pool_type).or_insert(0) += 1;
    }
    
    /// Record an event emission
    pub fn record_event_emitted(&mut self) {
        self.events_emitted += 1;
    }
    
    /// Record an error
    pub fn record_error(&mut self, error_type: &str) {
        *self.errors_by_type.entry(error_type.to_string()).or_insert(0) += 1;
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
    /// Engine is not started
    Stopped,
    /// Engine is starting up
    Starting,
    /// Engine is running and ready to process
    Running,
    /// Engine is shutting down
    Stopping,
    /// Engine encountered an error
    Error,
}

impl EngineStatus {
    /// Check if engine is running (can process packages)
    pub fn is_running(&self) -> bool {
        matches!(self, EngineStatus::Running)
    }

    /// Check if engine is in a transitional state
    pub fn is_transitioning(&self) -> bool {
        matches!(self, EngineStatus::Starting | EngineStatus::Stopping)
    }
}

/// Processing context
#[derive(Debug, Clone)]
pub struct ProcessingContext {
    /// Channel type
    pub channel_type: Option<ChannelType>,
    
    /// Route target
    pub route_target: Option<RouteTarget>,
    
    /// Current pool in processing pipeline
    pub current_pool: Option<PoolType>,
    
    /// Current flow depth (prevents infinite loops)
    pub depth: usize,
    
    /// Processing errors encountered
    pub errors: Vec<ProcessingError>,
}

impl ProcessingContext {
    /// Create a new processing context
    pub fn new() -> Self {
        Self {
            channel_type: None,
            route_target: None,
            current_pool: None,
            depth: 0,
            errors: Vec::new(),
        }
    }
    
    /// Check if max depth is exceeded
    pub fn is_depth_exceeded(&self, max_depth: usize) -> bool {
        self.depth > max_depth
    }
    
    /// Add an error to the context
    pub fn add_error(&mut self, error: ProcessingError) {
        self.errors.push(error);
    }
    
    /// Increment depth
    pub fn increment_depth(&mut self) {
        self.depth += 1;
    }
}

/// Processing error
#[derive(Debug, Clone)]
pub struct ProcessingError {
    /// Pool where the error occurred
    pub pool_type: Option<PoolType>,
    
    /// Error message
    pub message: String,
    
    /// Error type
    pub error_type: String,
}

impl ProcessingError {
    /// Create a new processing error
    pub fn new(pool_type: Option<PoolType>, message: impl fmt::Display, error_type: &str) -> Self {
        Self {
            pool_type,
            message: message.to_string(),
            error_type: error_type.to_string(),
        }
    }
}

impl fmt::Display for ProcessingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(pool) = self.pool_type {
            write!(f, "[{}] {}: {}", pool, self.error_type, self.message)
        } else {
            write!(f, "[{}] {}", self.error_type, self.message)
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
        assert!(EngineStatus::Stopped.is_running() == false);
        assert!(EngineStatus::Starting.is_running() == false);
        assert!(EngineStatus::Running.is_running() == true);
        assert!(EngineStatus::Stopping.is_running() == false);
        assert!(EngineStatus::Error.is_running() == false);

        assert!(EngineStatus::Starting.is_transitioning() == true);
        assert!(EngineStatus::Stopping.is_transitioning() == true);
        assert!(EngineStatus::Running.is_transitioning() == false);
    }

    #[test]
    fn test_processing_context() {
        let context = ProcessingContext::new();
        assert!(context.channel_type.is_none());
        assert!(context.route_target.is_none());
        assert_eq!(context.depth, 0);
        assert!(context.errors.is_empty());
    }
    
    #[test]
    fn test_processing_context_depth() {
        let mut context = ProcessingContext::new();
        assert!(!context.is_depth_exceeded(10));
        
        context.increment_depth();
        assert_eq!(context.depth, 1);
        assert!(!context.is_depth_exceeded(10));
        
        for _ in 0..10 {
            context.increment_depth();
        }
        assert!(context.is_depth_exceeded(10));
    }
    
    #[test]
    fn test_processing_error() {
        let error = ProcessingError::new(Some(PoolType::Input), "Test error", "TestType");
        assert_eq!(error.pool_type, Some(PoolType::Input));
        assert_eq!(error.message, "Test error");
        assert_eq!(error.error_type, "TestType");
        
        let display = error.to_string();
        assert!(display.contains("input"));
        assert!(display.contains("TestType"));
        assert!(display.contains("Test error"));
    }
}
