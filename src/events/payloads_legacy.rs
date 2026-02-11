//! Event payloads - content data for group events
//!
//! This module defines both legacy event payloads and the new universal payload system.

use crate::events::traits::EventMetadata;
use serde::{Deserialize, Serialize};
use std::any::Any;
use thiserror::Error;

// ============================================================================
// Message Payloads (Legacy - for event metadata)
// ============================================================================

/// Message payload structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessagePayload {
    /// Message subtype
    pub subtype: MessageSubtype,
    /// Message content
    pub content: MessageContent,
    /// Event metadata
    pub metadata: EventMetadata,
}

impl MessagePayload {
    /// Create new message payload
    pub fn new(subtype: MessageSubtype, content: MessageContent, metadata: EventMetadata) -> Self {
        Self {
            subtype,
            content,
            metadata,
        }
    }

    /// Get event type string
    pub fn event_type(&self) -> &str {
        match self.subtype {
            MessageSubtype::Text => "message.text",
            MessageSubtype::Image => "message.image",
            MessageSubtype::Voice => "message.voice",
            MessageSubtype::Video => "message.video",
            MessageSubtype::At => "message.at",
            MessageSubtype::Reply => "message.reply",
            MessageSubtype::Forward => "message.forward",
            MessageSubtype::File => "message.file",
            MessageSubtype::Location => "message.location",
            MessageSubtype::Sticker => "message.sticker",
            MessageSubtype::Markdown => "message.markdown",
        }
    }
}

/// Message subtypes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageSubtype {
    Text,
    Image,
    Voice,
    Video,
    At,
    Reply,
    Forward,
    File,
    Location,
    Sticker,
    Markdown,
}

/// Message content types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "content_type")]
#[serde(rename_all = "snake_case")]
pub enum MessageContent {
    Text {
        text: String,
    },
    Image {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        caption: Option<String>,
    },
    Voice {
        url: String,
        duration: u32,
    },
    Video {
        url: String,
        duration: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        cover_url: Option<String>,
    },
    At {
        text: String,
        at_list: Vec<String>,
    },
    Reply {
        reply_to: String,
        text: String,
    },
    Forward {
        forward_from: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
    },
    File {
        url: String,
        name: String,
        size: u64,
    },
    Location {
        latitude: f64,
        longitude: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        address: Option<String>,
    },
    Sticker {
        sticker_id: String,
    },
    Markdown {
        content: String,
    },
}

impl MessageContent {
    /// Get text content if applicable
    pub fn text(&self) -> Option<&str> {
        match self {
            MessageContent::Text { text } => Some(text),
            MessageContent::Reply { text, .. } => Some(text),
            MessageContent::Forward { text, .. } => text.as_deref(),
            MessageContent::Markdown { content } => Some(content),
            MessageContent::Image { caption, .. } => caption.as_deref(),
            MessageContent::At { text, .. } => Some(text),
            _ => None,
        }
    }

    /// Get media URL if applicable
    pub fn media_url(&self) -> Option<&str> {
        match self {
            MessageContent::Image { url, .. } => Some(url),
            MessageContent::Voice { url, .. } => Some(url),
            MessageContent::Video { url, .. } => Some(url),
            MessageContent::File { url, .. } => Some(url),
            MessageContent::Sticker { sticker_id } => Some(sticker_id),
            _ => None,
        }
    }
}

// ============================================================================
// Notice Payloads (Legacy)
// ============================================================================

/// Notice payload structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NoticePayload {
    /// Notice subtype
    pub subtype: NoticeSubtype,
    /// Notice content
    pub content: NoticeContent,
    /// Event metadata
    pub metadata: EventMetadata,
}

impl NoticePayload {
    /// Create new notice payload
    pub fn new(subtype: NoticeSubtype, content: NoticeContent, metadata: EventMetadata) -> Self {
        Self {
            subtype,
            content,
            metadata,
        }
    }

    /// Get event type string
    pub fn event_type(&self) -> &str {
        match self.subtype {
            NoticeSubtype::GroupMemberJoin => "notice.group.member.join",
            NoticeSubtype::GroupMemberLeave => "notice.group.member.leave",
            NoticeSubtype::GroupMemberKick => "notice.group.member.kick",
            NoticeSubtype::GroupMemberBan => "notice.group.member.ban",
            NoticeSubtype::GroupMemberMute => "notice.group.member.mute",
            NoticeSubtype::GroupNameChange => "notice.group.name.change",
            NoticeSubtype::FriendAdd => "notice.friend.add",
            NoticeSubtype::FriendDelete => "notice.friend.delete",
            NoticeSubtype::GroupInvite => "notice.group.invite",
            NoticeSubtype::GroupDisband => "notice.group.disband",
            NoticeSubtype::FriendRequestNotice => "notice.friend.request",
            NoticeSubtype::System => "notice.system",
        }
    }
}

/// Notice subtypes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NoticeSubtype {
    GroupMemberJoin,
    GroupMemberLeave,
    GroupMemberKick,
    GroupMemberBan,
    GroupMemberMute,
    GroupNameChange,
    FriendAdd,
    FriendDelete,
    GroupInvite,
    GroupDisband,
    FriendRequestNotice,
    System,
}

/// Notice content types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "content_type")]
#[serde(rename_all = "snake_case")]
pub enum NoticeContent {
    GroupMemberJoin {
        user_id: String,
        group_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        user_info: Option<UserInfo>,
    },
    GroupMemberLeave {
        user_id: String,
        group_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    GroupMemberKick {
        user_id: String,
        group_id: String,
        operator_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    GroupMemberBan {
        user_id: String,
        group_id: String,
        operator_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    GroupMemberMute {
        user_id: String,
        group_id: String,
        operator_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    GroupNameChange {
        old_name: String,
        new_name: String,
        operator_id: String,
    },
    FriendAdd {
        user_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        user_info: Option<UserInfo>,
    },
    FriendDelete {
        user_id: String,
    },
    GroupInvite {
        group_id: String,
        inviter_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        invitee_id: Option<String>,
    },
    GroupDisband {
        group_id: String,
        operator_id: String,
    },
    FriendRequestNotice {
        from_user_id: String,
    },
    System {
        notice_type: String,
        content: String,
    },
}

/// User information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age: Option<u32>,
}

// ============================================================================
// Request Payloads (Legacy)
// ============================================================================

/// Request payload structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RequestPayload {
    /// Request subtype
    pub subtype: RequestSubtype,
    /// Request content
    pub content: RequestContent,
    /// Event metadata
    pub metadata: EventMetadata,
}

impl RequestPayload {
    /// Create new request payload
    pub fn new(subtype: RequestSubtype, content: RequestContent, metadata: EventMetadata) -> Self {
        Self {
            subtype,
            content,
            metadata,
        }
    }

    /// Get event type string
    pub fn event_type(&self) -> &str {
        match self.subtype {
            RequestSubtype::Friend => "request.friend",
            RequestSubtype::GroupInvite => "request.group.invite",
            RequestSubtype::GroupJoin => "request.group.join",
            RequestSubtype::Approve => "request.approve",
            RequestSubtype::Reject => "request.reject",
            RequestSubtype::Cancel => "request.cancel",
        }
    }
}

/// Request subtypes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RequestSubtype {
    Friend,
    GroupInvite,
    GroupJoin,
    Approve,
    Reject,
    Cancel,
}

/// Request content types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "content_type")]
#[serde(rename_all = "snake_case")]
pub enum RequestContent {
    Friend {
        from_user_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        comment: Option<String>,
    },
    GroupInvite {
        inviter_id: String,
        group_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    GroupJoin {
        user_id: String,
        group_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    Approve {
        request_type: String,
        request_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        approver_id: Option<String>,
    },
    Reject {
        request_type: String,
        request_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        rejector_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    Cancel {
        request_type: String,
        request_id: String,
        canceled_by: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}

// ============================================================================
// Helper trait for payloads (Legacy - for event metadata)
// ============================================================================

/// Legacy Payload trait - common interface for event payloads
pub trait Payload {
    /// Get event metadata
    fn metadata(&self) -> &EventMetadata;
    /// Get event type string
    fn event_type(&self) -> &str;
}

impl Payload for MessagePayload {
    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn event_type(&self) -> &str {
        self.event_type()
    }
}

impl Payload for NoticePayload {
    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn event_type(&self) -> &str {
        self.event_type()
    }
}

impl Payload for RequestPayload {
    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn event_type(&self) -> &str {
        self.event_type()
    }
}

// ============================================================================
// Universal Payload System (Design Document v2.0)
// ============================================================================

/// Errors related to Payload operations
#[derive(Error, Debug)]
pub enum PayloadError {
    #[error("Payload type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },
    
    #[error("Failed to serialize payload: {0}")]
    SerializationError(String),
    
    #[error("Failed to deserialize payload: {0}")]
    DeserializationError(String),
    
    #[error("Unknown payload type: {0}")]
    UnknownType(String),
}

/// Type marker for payload types (object-safe)
pub trait PayloadType: Send + Sync + 'static {
    /// Get type name dynamically
    fn type_name(&self) -> &'static str;
    
    /// Get size estimate in bytes
    fn size_estimate(&self) -> usize;
}

/// Universal Payload trait for Package content (object-safe version)
/// This is v2.0 payload system from design document
pub trait UniversalPayload: PayloadType + std::fmt::Debug {
    /// Convert to Any for downcasting
    fn as_any(&self) -> &dyn Any;
    
    /// Convert to mutable Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Text payload for text messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPayload {
    /// Text content
    pub content: String,
    
    /// Text format (plain, markdown, etc.)
    #[serde(default)]
    pub format: TextFormat,
}

impl TextPayload {
    /// Create a new text payload
    pub fn new<S: Into<String>>(content: S) -> Self {
        Self {
            content: content.into(),
            format: TextFormat::Plain,
        }
    }
    
    /// Set text format
    pub fn with_format(mut self, format: TextFormat) -> Self {
        self.format = format;
        self
    }
}

impl PayloadType for TextPayload {
    fn type_name(&self) -> &'static str {
        "text"
    }
    
    fn size_estimate(&self) -> usize {
        self.content.len()
    }
}

impl UniversalPayload for TextPayload {
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Text format types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TextFormat {
    Plain,
    Markdown,
    Html,
    Json,
}

impl Default for TextFormat {
    fn default() -> Self {
        TextFormat::Plain
    }
}

/// Blob payload for binary data (images, files, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobPayload {
    /// Binary data
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
    
    /// MIME type
    #[serde(default)]
    pub mime_type: String,
    
    /// Optional URL (for large data)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl BlobPayload {
    /// Create a new blob payload
    pub fn new(data: Vec<u8>, mime_type: String) -> Self {
        Self {
            data,
            mime_type,
            url: None,
        }
    }
    
    /// Create from a URL (for large data)
    pub fn from_url(url: String, mime_type: String) -> Self {
        Self {
            data: Vec::new(),
            mime_type,
            url: Some(url),
        }
    }
}

impl PayloadType for BlobPayload {
    fn type_name(&self) -> &'static str {
        "blob"
    }
    
    fn size_estimate(&self) -> usize {
        self.data.len()
    }
}

impl UniversalPayload for BlobPayload {
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Event payload for structured events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPayload {
    /// Event type
    pub event_type: String,
    
    /// Event data (flexible JSON structure)
    pub data: serde_json::Value,
}

impl EventPayload {
    /// Create a new event payload
    pub fn new<S: Into<String>>(event_type: S, data: serde_json::Value) -> Self {
        Self {
            event_type: event_type.into(),
            data,
        }
    }
    
    /// Get typed data from event
    pub fn get_data<T: serde::de::DeserializeOwned>(&self) -> Result<T, PayloadError> {
        serde_json::from_value(self.data.clone())
            .map_err(|e| PayloadError::DeserializationError(e.to_string()))
    }
}

impl PayloadType for EventPayload {
    fn type_name(&self) -> &'static str {
        "event"
    }
    
    fn size_estimate(&self) -> usize {
        self.data.to_string().len()
    }
}

impl UniversalPayload for EventPayload {
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Wrapper for boxed payload
pub type BoxedPayload = Box<dyn UniversalPayload>;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_payload_text() {
        let metadata = EventMetadata::new("message.text")
            .with_user_id("user123")
            .with_group_id("group456");

        let payload = MessagePayload {
            subtype: MessageSubtype::Text,
            content: MessageContent::Text {
                text: "Hello world".to_string(),
            },
            metadata,
        };

        assert_eq!(payload.event_type(), "message.text");
        assert_eq!(payload.content.text(), Some("Hello world"));
    }

    #[test]
    fn test_notice_payload_member_join() {
        let metadata = EventMetadata::new("notice.group.member.join")
            .with_user_id("user123")
            .with_group_id("group456");

        let payload = NoticePayload {
            subtype: NoticeSubtype::GroupMemberJoin,
            content: NoticeContent::GroupMemberJoin {
                user_id: "user123".to_string(),
                group_id: "group456".to_string(),
                user_info: None,
            },
            metadata,
        };

        assert_eq!(payload.event_type(), "notice.group.member.join");
    }

    #[test]
    fn test_request_payload_friend() {
        let metadata = EventMetadata::new("request.friend")
            .with_user_id("user123");

        let payload = RequestPayload {
            subtype: RequestSubtype::Friend,
            content: RequestContent::Friend {
                from_user_id: "user123".to_string(),
                comment: Some("Let's be friends".to_string()),
            },
            metadata,
        };

        assert_eq!(payload.event_type(), "request.friend");
    }

    #[test]
    fn test_serialization() {
        let metadata = EventMetadata::new("message.text");
        let payload = MessagePayload {
            subtype: MessageSubtype::Text,
            content: MessageContent::Text {
                text: "Hello".to_string(),
            },
            metadata,
        };

        let json = serde_json::to_string(&payload).unwrap();
        let deserialized: MessagePayload = serde_json::from_str(&json).unwrap();

        assert_eq!(payload, deserialized);
    }

    #[test]
    fn test_text_payload() {
        let payload = TextPayload::new("Hello world");
        assert_eq!(payload.type_name(), "text");
        assert_eq!(payload.content, "Hello world");
    }

    #[test]
    fn test_blob_payload() {
        let data = vec![1u8, 2u8, 3u8];
        let payload = BlobPayload::new(data.clone(), "application/octet-stream".to_string());
        assert_eq!(payload.type_name(), "blob");
        assert_eq!(payload.data, data);
    }

    #[test]
    fn test_event_payload() {
        let data = serde_json::json!({"key": "value"});
        let payload = EventPayload::new("test.event", data.clone());
        assert_eq!(payload.type_name(), "event");
        assert_eq!(payload.data, data);
    }
}
