//! Core adapter traits

use crate::adapters::{AdapterConfig, AdapterStatus};
use crate::events::EventEnum;
use crate::errors::{LoquatError, Result};
use std::fmt::Debug;

/// Message target for sending messages
#[derive(Debug, Clone)]
pub enum Target {
    /// Private message target
    User {
        /// User ID
        user_id: String,
    },
    /// Group message target
    Group {
        /// Group ID
        group_id: String,
    },
    /// Channel message target
    Channel {
        /// Channel ID
        channel_id: String,
    },
}

/// Message for sending through adapter
#[derive(Debug, Clone)]
pub enum Message {
    /// Text message
    Text {
        /// Message content
        content: String,
    },
    /// Image message
    Image {
        /// Image URL or data
        url: String,
        /// Optional caption
        caption: Option<String>,
    },
    /// Voice message
    Voice {
        /// Voice URL or data
        url: String,
        /// Duration in seconds
        duration: u32,
    },
    /// Video message
    Video {
        /// Video URL or data
        url: String,
        /// Duration in seconds
        duration: u32,
        /// Optional cover URL
        cover_url: Option<String>,
    },
    /// Sticker message
    Sticker {
        /// Sticker ID
        sticker_id: String,
    },
}

/// Core adapter trait - all platform adapters must implement this
/// 
/// Note: This trait is object-safe and can be used as `dyn Adapter`.
pub trait Adapter: Send + Sync + Debug {
    /// Get adapter name
    fn name(&self) -> &str;
    
    /// Get adapter version
    fn version(&self) -> &str;
    
    /// Get adapter ID
    fn adapter_id(&self) -> &str;
    
    /// Get adapter configuration
    fn config(&self) -> &AdapterConfig;
    
    /// Get adapter status
    fn status(&self) -> AdapterStatus;
    
    /// Check if adapter is running
    fn is_running(&self) -> bool {
        self.status() == AdapterStatus::Running
    }
    
    /// Check if adapter is connected
    fn is_connected(&self) -> bool {
        self.status().is_active()
    }
    
    /// Get statistics about adapter
    fn statistics(&self) -> AdapterStatistics;
}

/// Adapter statistics
#[derive(Debug, Clone)]
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
    
    /// Last activity timestamp
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
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
}
