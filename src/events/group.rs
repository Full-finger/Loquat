//! Group structure - contains events
//!
//! Group contains an array of Event objects.

use crate::events::Event;
use serde::{Serialize, Deserialize};
use std::fmt::Debug;

/// Group - contains an array of Event objects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    /// Events - array of event objects
    pub events: Vec<Box<dyn Event>>,
    
    /// Group ID
    pub group_id: String,
    
    /// Group type/category
    pub group_type: Option<String>,
}

impl Group {
    /// Create a new group
    pub fn new(group_id: &str) -> Self {
        Self {
            events: Vec::new(),
            group_id: group_id.to_string(),
            group_type: None,
        }
    }
    
    /// Create a new group with type
    pub fn with_type(group_id: &str, group_type: &str) -> Self {
        Self {
            events: Vec::new(),
            group_id: group_id.to_string(),
            group_type: Some(group_type.to_string()),
        }
    }
    
    /// Add an event
    pub fn with_event(mut self, event: Box<dyn Event>) -> Self {
        self.events.push(event);
        self
    }
    
    /// Add multiple events
    pub fn with_events(mut self, events: Vec<Box<dyn Event>>) -> Self {
        self.events.extend(events);
        self
    }
    
    /// Set group type
    pub fn set_group_type(mut self, group_type: &str) -> Self {
        self.group_type = Some(group_type.to_string());
        self
    }
}

impl Default for Group {
    fn default() -> Self {
        Self::new("default")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock event for testing
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct MockEvent {
        pub event_id: String,
        pub timestamp: chrono::DateTime<chrono::Utc>,
    }

    impl Event for MockEvent {
        fn event_id(&self) -> &str {
            &self.event_id
        }
        
        fn event_type(&self) -> &str {
            "mock"
        }
        
        fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
            self.timestamp
        }
        
        fn source(&self) -> crate::events::EventSource {
            crate::events::EventSource::Unknown
        }
        
        fn user_id(&self) -> Option<&str> {
            None
        }
        
        fn group_id(&self) -> Option<&str> {
            None
        }
        
        fn self_id(&self) -> Option<&str> {
            None
        }
        
        fn correlation_id(&self) -> Option<&str> {
            None
        }
    }

    #[test]
    fn test_group_creation() {
        let group = Group::new("test_group");
        
        assert!(group.events.is_empty());
        assert_eq!(group.group_id, "test_group");
        assert!(group.group_type.is_none());
    }
    
    #[test]
    fn test_group_with_type() {
        let group = Group::with_type("test_group", "message");
        
        assert_eq!(group.group_id, "test_group");
        assert_eq!(group.group_type, Some("message".to_string()));
    }
    
    #[test]
    fn test_group_builder() {
        let mock_event = MockEvent {
            event_id: "evt-1".to_string(),
            timestamp: chrono::Utc::now(),
        };
        
        let group = Group::new("test_group")
            .with_event(Box::new(mock_event.clone()));
        
        assert_eq!(group.events.len(), 1);
    }
}
