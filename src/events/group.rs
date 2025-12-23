//! Group structure - contains events
//!
//! Group contains an array of EventEnum objects.

use crate::events::EventEnum;
use serde::{Serialize, Deserialize};

/// Group - contains an array of EventEnum objects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    /// Events - array of event objects
    pub events: Vec<EventEnum>,
    
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
    pub fn with_event(mut self, event: EventEnum) -> Self {
        self.events.push(event);
        self
    }
    
    /// Add multiple events
    pub fn with_events(mut self, events: Vec<EventEnum>) -> Self {
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
    use crate::events::message::MessageEvent;
    use crate::events::traits::EventMetadata;

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
        let message_event = EventEnum::Message(MessageEvent::Text {
            text: "Hello".to_string(),
            metadata: EventMetadata::default(),
        });
        
        let group = Group::new("test_group")
            .with_event(message_event);
        
        assert_eq!(group.events.len(), 1);
    }
}
