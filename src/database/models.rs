//! Database models for Loquat framework

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Event record stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    /// Unique identifier
    pub id: i64,
    /// Package ID from event system
    pub package_id: String,
    /// Event type (message, notice, request, etc.)
    pub event_type: String,
    /// Source adapter
    pub source: String,
    /// Target site (user, group, channel)
    pub target_site: String,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Raw event data (JSON)
    pub raw_data: String,
    /// Processing status
    pub status: EventStatus,
    /// Error message if processing failed
    pub error_message: Option<String>,
    /// Created at timestamp
    pub created_at: DateTime<Utc>,
}

/// Event processing status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum EventStatus {
    /// Event received but not yet processed
    Pending = 0,
    /// Event is being processed
    Processing = 1,
    /// Event processed successfully
    Success = 2,
    /// Event processing failed
    Failed = 3,
}

impl EventStatus {
    /// Convert from i32
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => EventStatus::Pending,
            1 => EventStatus::Processing,
            2 => EventStatus::Success,
            3 => EventStatus::Failed,
            _ => EventStatus::Pending,
        }
    }

    /// Convert to i32
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }
}

/// Plugin record stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRecord {
    /// Unique identifier
    pub id: i64,
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin type (Rust, Python, JS)
    pub plugin_type: String,
    /// Plugin file path
    pub file_path: String,
    /// Plugin status
    pub status: PluginStatus,
    /// Last loaded timestamp
    pub last_loaded_at: Option<DateTime<Utc>>,
    /// Load count
    pub load_count: i32,
    /// Error message if load failed
    pub error_message: Option<String>,
    /// Metadata (JSON)
    pub metadata: Option<String>,
    /// Created at timestamp
    pub created_at: DateTime<Utc>,
    /// Updated at timestamp
    pub updated_at: DateTime<Utc>,
}

/// Plugin status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum PluginStatus {
    /// Plugin not loaded
    Unloaded = 0,
    /// Plugin loading
    Loading = 1,
    /// Plugin loaded successfully
    Loaded = 2,
    /// Plugin load failed
    Failed = 3,
    /// Plugin disabled
    Disabled = 4,
}

impl PluginStatus {
    /// Convert from i32
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => PluginStatus::Unloaded,
            1 => PluginStatus::Loading,
            2 => PluginStatus::Loaded,
            3 => PluginStatus::Failed,
            4 => PluginStatus::Disabled,
            _ => PluginStatus::Unloaded,
        }
    }

    /// Convert to i32
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }
}

/// Adapter record stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterRecord {
    /// Unique identifier
    pub id: i64,
    /// Adapter ID
    pub adapter_id: String,
    /// Adapter type (console, napcat, etc.)
    pub adapter_type: String,
    /// Adapter configuration (JSON)
    pub config: String,
    /// Adapter status
    pub status: AdapterStatus,
    /// Connected flag
    pub connected: bool,
    /// Last started timestamp
    pub last_started_at: Option<DateTime<Utc>>,
    /// Last stopped timestamp
    pub last_stopped_at: Option<DateTime<Utc>>,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Created at timestamp
    pub created_at: DateTime<Utc>,
    /// Updated at timestamp
    pub updated_at: DateTime<Utc>,
}

/// Adapter status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum AdapterStatus {
    /// Adapter not initialized
    Uninitialized = 0,
    /// Adapter initializing
    Initializing = 1,
    /// Adapter ready
    Ready = 2,
    /// Adapter running
    Running = 3,
    /// Adapter stopping
    Stopping = 4,
    /// Adapter stopped
    Stopped = 5,
    /// Adapter error
    Error = 6,
}

impl AdapterStatus {
    /// Convert from i32
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => AdapterStatus::Uninitialized,
            1 => AdapterStatus::Initializing,
            2 => AdapterStatus::Ready,
            3 => AdapterStatus::Running,
            4 => AdapterStatus::Stopping,
            5 => AdapterStatus::Stopped,
            6 => AdapterStatus::Error,
            _ => AdapterStatus::Uninitialized,
        }
    }

    /// Convert to i32
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }
}

/// Log record stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRecord {
    /// Unique identifier
    pub id: i64,
    /// Log level (Trace, Debug, Info, Warn, Error)
    pub level: String,
    /// Log message
    pub message: String,
    /// Component that generated the log
    pub component: Option<String>,
    /// Log timestamp
    pub timestamp: DateTime<Utc>,
    /// Additional context (JSON)
    pub context: Option<String>,
}

/// Statistics record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsRecord {
    /// Unique identifier
    pub id: i64,
    /// Statistic name
    pub name: String,
    /// Statistic value (as string to support various types)
    pub value: String,
    /// Statistic timestamp
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_status_conversion() {
        assert_eq!(EventStatus::from_i32(0), EventStatus::Pending);
        assert_eq!(EventStatus::from_i32(1), EventStatus::Processing);
        assert_eq!(EventStatus::from_i32(2), EventStatus::Success);
        assert_eq!(EventStatus::from_i32(3), EventStatus::Failed);
        assert_eq!(EventStatus::from_i32(999), EventStatus::Pending);

        assert_eq!(EventStatus::Pending.as_i32(), 0);
        assert_eq!(EventStatus::Processing.as_i32(), 1);
        assert_eq!(EventStatus::Success.as_i32(), 2);
        assert_eq!(EventStatus::Failed.as_i32(), 3);
    }

    #[test]
    fn test_plugin_status_conversion() {
        assert_eq!(PluginStatus::from_i32(0), PluginStatus::Unloaded);
        assert_eq!(PluginStatus::from_i32(1), PluginStatus::Loading);
        assert_eq!(PluginStatus::from_i32(2), PluginStatus::Loaded);
        assert_eq!(PluginStatus::from_i32(3), PluginStatus::Failed);
        assert_eq!(PluginStatus::from_i32(4), PluginStatus::Disabled);

        assert_eq!(PluginStatus::Unloaded.as_i32(), 0);
        assert_eq!(PluginStatus::Loading.as_i32(), 1);
        assert_eq!(PluginStatus::Loaded.as_i32(), 2);
        assert_eq!(PluginStatus::Failed.as_i32(), 3);
        assert_eq!(PluginStatus::Disabled.as_i32(), 4);
    }

    #[test]
    fn test_adapter_status_conversion() {
        assert_eq!(AdapterStatus::from_i32(0), AdapterStatus::Uninitialized);
        assert_eq!(AdapterStatus::from_i32(1), AdapterStatus::Initializing);
        assert_eq!(AdapterStatus::from_i32(2), AdapterStatus::Ready);
        assert_eq!(AdapterStatus::from_i32(3), AdapterStatus::Running);
        assert_eq!(AdapterStatus::from_i32(4), AdapterStatus::Stopping);
        assert_eq!(AdapterStatus::from_i32(5), AdapterStatus::Stopped);
        assert_eq!(AdapterStatus::from_i32(6), AdapterStatus::Error);

        assert_eq!(AdapterStatus::Uninitialized.as_i32(), 0);
        assert_eq!(AdapterStatus::Initializing.as_i32(), 1);
        assert_eq!(AdapterStatus::Ready.as_i32(), 2);
        assert_eq!(AdapterStatus::Running.as_i32(), 3);
        assert_eq!(AdapterStatus::Stopping.as_i32(), 4);
        assert_eq!(AdapterStatus::Stopped.as_i32(), 5);
        assert_eq!(AdapterStatus::Error.as_i32(), 6);
    }
}
