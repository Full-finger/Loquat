//! Core adapter traits

use crate::adapters::{AdapterConfig, AdapterStatus, types::AdapterStatistics};
use crate::events::EventEnum;
use crate::errors::{LoquatError, Result};
use crate::actor::messages::AdapterMessage;
use std::fmt::Debug;
use tokio::sync::mpsc;

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
/// This trait provides synchronous methods for adapter interaction.
/// For asynchronous operations, use AdapterWrapper which wraps the trait object.
pub trait Adapter: Send + Sync + Debug + std::any::Any {
    /// Get adapter name
    fn name(&self) -> &str;
    
    /// Get adapter version
    fn version(&self) -> &str;
    
    /// Get adapter ID
    fn adapter_id(&self) -> &str;
    
    /// Get adapter configuration (synchronous, cached value)
    fn config(&self) -> AdapterConfig;
    
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
    
    /// Set event sender for this adapter
    fn set_event_sender(&self, sender: Option<mpsc::UnboundedSender<EventEnum>>);
    
    /// Send an event through event sender
    fn send_event(&self, event: EventEnum) -> Result<()>;
    
    /// Get as Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock adapter for testing
    #[derive(Debug)]
    struct MockAdapter;

    impl Adapter for MockAdapter {
        fn name(&self) -> &str {
            "MockAdapter"
        }

        fn version(&self) -> &str {
            "1.0.0"
        }

        fn adapter_id(&self) -> &str {
            "mock-001"
        }

        fn config(&self) -> AdapterConfig {
            AdapterConfig::new("mock", "mock-001", "ws://localhost")
        }
        
        fn status(&self) -> AdapterStatus {
            AdapterStatus::Running
        }
        
        fn statistics(&self) -> AdapterStatistics {
            AdapterStatistics::default()
        }

        fn set_event_sender(&self, _sender: Option<mpsc::UnboundedSender<EventEnum>>) {}
        
        fn send_event(&self, _event: EventEnum) -> Result<()> {
            Ok(())
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[test]
    fn test_adapter_trait() {
        let adapter = MockAdapter;
        
        assert_eq!(adapter.name(), "MockAdapter");
        assert_eq!(adapter.version(), "1.0.0");
        assert_eq!(adapter.adapter_id(), "mock-001");
        assert_eq!(adapter.status(), AdapterStatus::Running);
        assert!(adapter.is_running());
        assert!(adapter.is_connected());
        
        let stats = adapter.statistics();
        assert_eq!(stats.events_received, 0);
        assert_eq!(stats.events_sent, 0);
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.errors, 0);
        assert_eq!(stats.uptime_seconds, 0);
        assert!(stats.last_activity.is_none());
    }
}
