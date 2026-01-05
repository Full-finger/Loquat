//! Unified event system with Simple and Group event classification
//!
//! This module provides the new event architecture that separates events into:
//! - Simple events: Events without complex content (e.g., heartbeat, lifecycle)
//! - Group events: Events with payload content (e.g., messages, notices, requests)

use crate::events::payloads::{
    MessagePayload, NoticePayload, RequestPayload,
    MessageContent, NoticeContent, RequestContent,
    UserInfo,
};
use crate::events::traits::{EventMetadata, EventSource};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// Simple Event
// ============================================================================

/// Simple event - an event without complex content
/// Used for notifications, status changes, system events, etc.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimpleEvent {
    /// Event type
    pub event_type: String,
    /// Event metadata
    pub metadata: EventMetadata,
}

impl SimpleEvent {
    /// Create new simple event
    pub fn new(event_type: impl Into<String>, metadata: EventMetadata) -> Self {
        Self {
            event_type: event_type.into(),
            metadata,
        }
    }
}

// ============================================================================
// Group Event
// ============================================================================

/// Group event - an event with payload content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GroupEvent {
    /// Message event
    Message(MessagePayload),
    /// Notice event
    Notice(NoticePayload),
    /// Request event
    Request(RequestPayload),
}

// ============================================================================
// Unified Event Enum
// ============================================================================

/// Unified event enum - new event architecture
/// Events are classified as either Simple (no content) or Group (with content)
/// 
/// This is the preferred event type to use in most scenarios, avoiding the need
/// to work with specific event types like MessageEvent, NoticeEvent, etc.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event_kind")]
#[serde(rename_all = "snake_case")]
pub enum UnifiedEvent {
    /// Simple event (no content)
    Simple(SimpleEvent),
    /// Group event (with content)
    Group(GroupEvent),
}

impl UnifiedEvent {
    // ========================================================================
    // Common accessors
    // ========================================================================

    /// Get event ID
    pub fn event_id(&self) -> &str {
        match self {
            UnifiedEvent::Simple(evt) => &evt.metadata.event_id,
            UnifiedEvent::Group(GroupEvent::Message(payload)) => &payload.metadata.event_id,
            UnifiedEvent::Group(GroupEvent::Notice(payload)) => &payload.metadata.event_id,
            UnifiedEvent::Group(GroupEvent::Request(payload)) => &payload.metadata.event_id,
        }
    }

    /// Get event type
    pub fn event_type(&self) -> &str {
        match self {
            UnifiedEvent::Simple(evt) => &evt.event_type,
            UnifiedEvent::Group(GroupEvent::Message(payload)) => payload.event_type(),
            UnifiedEvent::Group(GroupEvent::Notice(payload)) => payload.event_type(),
            UnifiedEvent::Group(GroupEvent::Request(payload)) => payload.event_type(),
        }
    }

    /// Get timestamp
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            UnifiedEvent::Simple(evt) => evt.metadata.timestamp,
            UnifiedEvent::Group(GroupEvent::Message(payload)) => payload.metadata.timestamp,
            UnifiedEvent::Group(GroupEvent::Notice(payload)) => payload.metadata.timestamp,
            UnifiedEvent::Group(GroupEvent::Request(payload)) => payload.metadata.timestamp,
        }
    }

    /// Get event source
    pub fn source(&self) -> &EventSource {
        match self {
            UnifiedEvent::Simple(evt) => &evt.metadata.source,
            UnifiedEvent::Group(GroupEvent::Message(payload)) => &payload.metadata.source,
            UnifiedEvent::Group(GroupEvent::Notice(payload)) => &payload.metadata.source,
            UnifiedEvent::Group(GroupEvent::Request(payload)) => &payload.metadata.source,
        }
    }

    /// Get user ID
    pub fn user_id(&self) -> Option<&str> {
        match self {
            UnifiedEvent::Simple(evt) => evt.metadata.user_id.as_deref(),
            UnifiedEvent::Group(GroupEvent::Message(payload)) => payload.metadata.user_id.as_deref(),
            UnifiedEvent::Group(GroupEvent::Notice(payload)) => payload.metadata.user_id.as_deref(),
            UnifiedEvent::Group(GroupEvent::Request(payload)) => payload.metadata.user_id.as_deref(),
        }
    }

    /// Get group ID
    pub fn group_id(&self) -> Option<&str> {
        match self {
            UnifiedEvent::Simple(evt) => evt.metadata.group_id.as_deref(),
            UnifiedEvent::Group(GroupEvent::Message(payload)) => payload.metadata.group_id.as_deref(),
            UnifiedEvent::Group(GroupEvent::Notice(payload)) => payload.metadata.group_id.as_deref(),
            UnifiedEvent::Group(GroupEvent::Request(payload)) => payload.metadata.group_id.as_deref(),
        }
    }

    /// Get self ID
    pub fn self_id(&self) -> Option<&str> {
        match self {
            UnifiedEvent::Simple(evt) => evt.metadata.self_id.as_deref(),
            UnifiedEvent::Group(GroupEvent::Message(payload)) => payload.metadata.self_id.as_deref(),
            UnifiedEvent::Group(GroupEvent::Notice(payload)) => payload.metadata.self_id.as_deref(),
            UnifiedEvent::Group(GroupEvent::Request(payload)) => payload.metadata.self_id.as_deref(),
        }
    }

    /// Get correlation ID
    pub fn correlation_id(&self) -> Option<&str> {
        match self {
            UnifiedEvent::Simple(evt) => evt.metadata.correlation_id.as_deref(),
            UnifiedEvent::Group(GroupEvent::Message(payload)) => payload.metadata.correlation_id.as_deref(),
            UnifiedEvent::Group(GroupEvent::Notice(payload)) => payload.metadata.correlation_id.as_deref(),
            UnifiedEvent::Group(GroupEvent::Request(payload)) => payload.metadata.correlation_id.as_deref(),
        }
    }

    // ========================================================================
    // Classification methods
    // ========================================================================

    /// Check if this is a simple event
    pub fn is_simple(&self) -> bool {
        matches!(self, UnifiedEvent::Simple(_))
    }

    /// Check if this is a group event
    pub fn is_group(&self) -> bool {
        matches!(self, UnifiedEvent::Group(_))
    }

    /// Get simple event if applicable
    pub fn as_simple(&self) -> Option<&SimpleEvent> {
        match self {
            UnifiedEvent::Simple(evt) => Some(evt),
            UnifiedEvent::Group(_) => None,
        }
    }

    /// Get group event if applicable
    pub fn as_group(&self) -> Option<&GroupEvent> {
        match self {
            UnifiedEvent::Group(evt) => Some(evt),
            UnifiedEvent::Simple(_) => None,
        }
    }

    // ========================================================================
    // Group event type checks
    // ========================================================================

    /// Check if this is a message event
    pub fn is_message(&self) -> bool {
        matches!(self, UnifiedEvent::Group(GroupEvent::Message(_)))
    }

    /// Check if this is a notice event
    pub fn is_notice(&self) -> bool {
        matches!(self, UnifiedEvent::Group(GroupEvent::Notice(_)))
    }

    /// Check if this is a request event
    pub fn is_request(&self) -> bool {
        matches!(self, UnifiedEvent::Group(GroupEvent::Request(_)))
    }

    /// Get message payload if applicable
    pub fn as_message(&self) -> Option<&MessagePayload> {
        match self {
            UnifiedEvent::Group(GroupEvent::Message(payload)) => Some(payload),
            _ => None,
        }
    }

    /// Get notice payload if applicable
    pub fn as_notice(&self) -> Option<&NoticePayload> {
        match self {
            UnifiedEvent::Group(GroupEvent::Notice(payload)) => Some(payload),
            _ => None,
        }
    }

    /// Get request payload if applicable
    pub fn as_request(&self) -> Option<&RequestPayload> {
        match self {
            UnifiedEvent::Group(GroupEvent::Request(payload)) => Some(payload),
            _ => None,
        }
    }

    // ========================================================================
    // Convenience constructors
    // ========================================================================

    /// Create a simple event
    pub fn simple(event_type: impl Into<String>, metadata: EventMetadata) -> Self {
        UnifiedEvent::Simple(SimpleEvent::new(event_type, metadata))
    }

    /// Create a text message event
    pub fn message_text(text: impl Into<String>, metadata: EventMetadata) -> Self {
        UnifiedEvent::Group(GroupEvent::Message(MessagePayload {
            subtype: crate::events::payloads::MessageSubtype::Text,
            content: MessageContent::Text {
                text: text.into(),
            },
            metadata,
        }))
    }

    /// Create an image message event
    pub fn message_image(
        url: impl Into<String>,
        caption: Option<String>,
        metadata: EventMetadata,
    ) -> Self {
        UnifiedEvent::Group(GroupEvent::Message(MessagePayload {
            subtype: crate::events::payloads::MessageSubtype::Image,
            content: MessageContent::Image {
                url: url.into(),
                caption,
            },
            metadata,
        }))
    }

    /// Create a heartbeat event
    pub fn heartbeat(interval: u32, metadata: EventMetadata) -> Self {
        UnifiedEvent::Simple(SimpleEvent {
            event_type: "meta.heartbeat".to_string(),
            metadata: metadata.with_extra("interval", interval),
        })
    }

    /// Create a lifecycle event
    pub fn lifecycle(phase: &str, metadata: EventMetadata) -> Self {
        UnifiedEvent::Simple(SimpleEvent {
            event_type: format!("meta.lifecycle.{}", phase),
            metadata,
        })
    }

    /// Create a connection status event
    pub fn connection_status(status: &str, metadata: EventMetadata) -> Self {
        UnifiedEvent::Simple(SimpleEvent {
            event_type: format!("meta.connection.{}", status),
            metadata,
        })
    }

    /// Create a group member join notice event
    pub fn notice_member_join(
        user_id: String,
        group_id: String,
        user_info: Option<UserInfo>,
        metadata: EventMetadata,
    ) -> Self {
        UnifiedEvent::Group(GroupEvent::Notice(NoticePayload {
            subtype: crate::events::payloads::NoticeSubtype::GroupMemberJoin,
            content: NoticeContent::GroupMemberJoin {
                user_id,
                group_id,
                user_info,
            },
            metadata,
        }))
    }

    /// Create a friend request event
    pub fn request_friend(
        from_user_id: String,
        comment: Option<String>,
        metadata: EventMetadata,
    ) -> Self {
        UnifiedEvent::Group(GroupEvent::Request(RequestPayload {
            subtype: crate::events::payloads::RequestSubtype::Friend,
            content: RequestContent::Friend {
                from_user_id,
                comment,
            },
            metadata,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_event_creation() {
        let metadata = EventMetadata::new("test.event")
            .with_source(EventSource::System);
        let event = UnifiedEvent::simple("meta.heartbeat", metadata);

        assert!(event.is_simple());
        assert!(!event.is_group());
        assert_eq!(event.event_type(), "meta.heartbeat");
    }

    #[test]
    fn test_message_text_event() {
        let metadata = EventMetadata::new("message.text")
            .with_user_id("user123")
            .with_group_id("group456");
        let event = UnifiedEvent::message_text("Hello world", metadata);

        assert!(event.is_group());
        assert!(event.is_message());
        assert!(!event.is_notice());
        assert_eq!(event.event_type(), "message.text");
        assert_eq!(event.user_id(), Some("user123"));
        assert_eq!(event.group_id(), Some("group456"));
    }

    #[test]
    fn test_message_content_access() {
        let metadata = EventMetadata::new("message.text");
        let event = UnifiedEvent::message_text("Hello", metadata);

        if let Some(payload) = event.as_message() {
            if let MessageContent::Text { text } = &payload.content {
                assert_eq!(text, "Hello");
            } else {
                panic!("Expected Text content");
            }
        } else {
            panic!("Expected message payload");
        }
    }

    #[test]
    fn test_heartbeat_event() {
        let metadata = EventMetadata::new("heartbeat");
        let event = UnifiedEvent::heartbeat(5000, metadata);

        assert!(event.is_simple());
        assert_eq!(event.event_type(), "meta.heartbeat");
    }

    #[test]
    fn test_notice_member_join() {
        let metadata = EventMetadata::new("notice.group.member.join")
            .with_user_id("user123")
            .with_group_id("group456");
        let event = UnifiedEvent::notice_member_join("user123".to_string(), "group456".to_string(), None, metadata);

        assert!(event.is_group());
        assert!(event.is_notice());
        assert_eq!(event.event_type(), "notice.group.member.join");
    }

    #[test]
    fn test_request_friend() {
        let metadata = EventMetadata::new("request.friend")
            .with_user_id("user123");
        let event = UnifiedEvent::request_friend("user123".to_string(), Some("Hi".to_string()), metadata);

        assert!(event.is_group());
        assert!(event.is_request());
        assert_eq!(event.event_type(), "request.friend");
    }

    #[test]
    fn test_serialization() {
        let metadata = EventMetadata::new("message.text");
        let event = UnifiedEvent::message_text("Hello", metadata);

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: UnifiedEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event, deserialized);
    }
}
