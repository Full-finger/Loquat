//! Core event traits for Loquat event system

use std::fmt::Debug;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

/// Event source - where the event originated from
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventSource {
    /// Direct user message
    User,
    /// System generated event
    System,
    /// API callback event
    Api,
    /// Worker processing event
    Worker(String),
    /// Unknown source
    Unknown,
}

/// Base trait for all events
pub trait Event: Send + Sync + Debug + Serialize + for<'de> Deserialize<'de> {
    /// Get event unique identifier
    fn event_id(&self) -> &str;
    
    /// Get event type name
    fn event_type(&self) -> &str;
    
    /// Get event timestamp
    fn timestamp(&self) -> DateTime<Utc>;
    
    /// Get event source
    fn source(&self) -> EventSource;
    
    /// Get user ID who triggered this event
    fn user_id(&self) -> Option<&str>;
    
    /// Get group ID if this is a group event
    fn group_id(&self) -> Option<&str>;
    
    /// Get self ID (bot's own ID)
    fn self_id(&self) -> Option<&str>;
    
    /// Get correlation ID for linking related events
    fn correlation_id(&self) -> Option<&str>;
}

/// Common metadata for events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventMetadata {
    /// Unique event identifier
    pub event_id: String,
    
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Event source
    pub source: EventSource,
    
    /// User ID who triggered the event
    pub user_id: Option<String>,
    
    /// Group ID (for group events)
    pub group_id: Option<String>,
    
    /// Self ID (bot's own ID)
    pub self_id: Option<String>,
    
    /// Correlation ID for linking events
    pub correlation_id: Option<String>,
    
    /// Additional custom metadata
    pub extra: serde_json::Value,
}

impl EventMetadata {
    /// Create new event metadata
    pub fn new(event_type: &str) -> Self {
        Self {
            event_id: format!("{}-{}-{}", event_type, Utc::now().timestamp_millis(), uuid::Uuid::new_v4()),
            timestamp: Utc::now(),
            source: EventSource::Unknown,
            user_id: None,
            group_id: None,
            self_id: None,
            correlation_id: None,
            extra: serde_json::json!({}),
        }
    }
    
    /// Set event source
    pub fn with_source(mut self, source: EventSource) -> Self {
        self.source = source;
        self
    }
    
    /// Set user ID
    pub fn with_user_id(mut self, user_id: &str) -> Self {
        self.user_id = Some(user_id.to_string());
        self
    }
    
    /// Set group ID
    pub fn with_group_id(mut self, group_id: &str) -> Self {
        self.group_id = Some(group_id.to_string());
        self
    }
    
    /// Set self ID
    pub fn with_self_id(mut self, self_id: &str) -> Self {
        self.self_id = Some(self_id.to_string());
        self
    }
    
    /// Set correlation ID
    pub fn with_correlation_id(mut self, correlation_id: &str) -> Self {
        self.correlation_id = Some(correlation_id.to_string());
        self
    }
    
    /// Add extra metadata
    pub fn with_extra<K: Into<String>, V: Serialize>(mut self, key: K, value: V) -> Self {
        if let Ok(obj) = serde_json::to_value(value) {
            self.extra[key.into()] = obj;
        }
        self
    }
}

impl Default for EventMetadata {
    fn default() -> Self {
        Self::new("unknown")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_metadata_creation() {
        let metadata = EventMetadata::new("test_event")
            .with_user_id("user123")
            .with_group_id("group456")
            .with_source(EventSource::User);
        
        assert_eq!(metadata.user_id, Some("user123".to_string()));
        assert_eq!(metadata.group_id, Some("group456".to_string()));
        assert_eq!(metadata.source, EventSource::User);
    }
    
    #[test]
    fn test_event_metadata_with_extra() {
        let metadata = EventMetadata::new("test_event")
            .with_extra("custom_key", "custom_value");
        
        assert_eq!(metadata.extra["custom_key"], "custom_value");
    }
}
