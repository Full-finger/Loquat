//! Adapter types

use crate::adapters::config::AdapterConfig;
use crate::adapters::status::AdapterStatus;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Adapter statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterStatistics {
    /// Number of events received
    pub events_received: u64,

    /// Number of events sent
    pub events_sent: u64,

    /// Number of messages sent
    pub messages_sent: u64,

    /// Number of errors encountered
    pub errors: u64,

    /// Uptime in seconds
    pub uptime_seconds: u64,

    /// Last activity timestamp (Unix timestamp)
    pub last_activity: Option<i64>,
}

impl Default for AdapterStatistics {
    fn default() -> Self {
        Self {
            events_received: 0,
            events_sent: 0,
            messages_sent: 0,
            errors: 0,
            uptime_seconds: 0,
            last_activity: None,
        }
    }
}

/// Adapter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterInfo {
    /// Adapter ID
    pub adapter_id: String,

    /// Adapter name
    pub name: String,

    /// Adapter version
    pub version: String,

    /// Current status
    pub status: AdapterStatus,

    /// Adapter type
    pub adapter_type: String,

    /// Adapter configuration
    pub config: AdapterConfig,

    /// Statistics
    pub statistics: AdapterStatistics,

    /// Load time (Unix timestamp)
    pub loaded_at: i64,
}

impl AdapterInfo {
    /// Create a new adapter info
    pub fn new(
        adapter_id: String,
        name: String,
        version: String,
        status: AdapterStatus,
        adapter_type: String,
        config: AdapterConfig,
        statistics: AdapterStatistics,
        loaded_at: i64,
    ) -> Self {
        Self {
            adapter_id,
            name,
            version,
            status,
            adapter_type,
            config,
            statistics,
            loaded_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_statistics_default() {
        let stats = AdapterStatistics::default();

        assert_eq!(stats.events_received, 0);
        assert_eq!(stats.events_sent, 0);
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.errors, 0);
        assert_eq!(stats.uptime_seconds, 0);
        assert!(stats.last_activity.is_none());
    }

    #[test]
    fn test_adapter_info_creation() {
        let config = AdapterConfig::new("test", "test-001", "ws://localhost");
        let stats = AdapterStatistics::default();
        let loaded_at = 1703500000;

        let info = AdapterInfo::new(
            "test-001".to_string(),
            "Test Adapter".to_string(),
            "1.0.0".to_string(),
            AdapterStatus::Ready,
            "test".to_string(),
            config,
            stats,
            loaded_at,
        );

        assert_eq!(info.adapter_id, "test-001");
        assert_eq!(info.name, "Test Adapter");
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.status, AdapterStatus::Ready);
        assert_eq!(info.adapter_type, "test");
        assert_eq!(info.loaded_at, loaded_at);
    }
}
