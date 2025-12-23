//! Message events for Loquat event system
//!
//! Message events include text, image, voice, video, at messages, etc.

use crate::events::traits::{Event, EventMetadata, EventSource};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::fmt::Debug;

/// Message event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageEvent {
    /// Text message
    Text {
        /// Message content
        text: String,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Image message
    Image {
        /// Image URL
        url: String,
        /// Optional caption
        caption: Option<String>,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Voice message
    Voice {
        /// Voice URL
        url: String,
        /// Duration in seconds
        duration: u32,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Video message
    Video {
        /// Video URL
        url: String,
        /// Duration in seconds
        duration: u32,
        /// Optional cover URL
        cover_url: Option<String>,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// At (@) message
    At {
        /// Message content
        text: String,
        /// List of @mentioned users
        at_list: Vec<String>,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Reply message
    Reply {
        /// Replied message ID
        reply_to: String,
        /// Message content
        text: String,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Forward message
    Forward {
        /// Forwarded message ID
        forward_from: String,
        /// Message content
        text: Option<String>,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// File message
    File {
        /// File URL
        url: String,
        /// File name
        name: String,
        /// File size in bytes
        size: u64,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Location message
    Location {
        /// Latitude
        latitude: f64,
        /// Longitude
        longitude: f64,
        /// Address description
        address: Option<String>,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Sticker message
    Sticker {
        /// Sticker ID
        sticker_id: String,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Markdown message
    Markdown {
        /// Markdown content
        content: String,
        /// Event metadata
        metadata: EventMetadata,
    },
}

impl Event for MessageEvent {
    fn event_id(&self) -> &str {
        match self {
            MessageEvent::Text { metadata, .. } => &metadata.event_id,
            MessageEvent::Image { metadata, .. } => &metadata.event_id,
            MessageEvent::Voice { metadata, .. } => &metadata.event_id,
            MessageEvent::Video { metadata, .. } => &metadata.event_id,
            MessageEvent::At { metadata, .. } => &metadata.event_id,
            MessageEvent::Reply { metadata, .. } => &metadata.event_id,
            MessageEvent::Forward { metadata, .. } => &metadata.event_id,
            MessageEvent::File { metadata, .. } => &metadata.event_id,
            MessageEvent::Location { metadata, .. } => &metadata.event_id,
            MessageEvent::Sticker { metadata, .. } => &metadata.event_id,
            MessageEvent::Markdown { metadata, .. } => &metadata.event_id,
        }
    }
    
    fn event_type(&self) -> &str {
        match self {
            MessageEvent::Text { .. } => "message.text",
            MessageEvent::Image { .. } => "message.image",
            MessageEvent::Voice { .. } => "message.voice",
            MessageEvent::Video { .. } => "message.video",
            MessageEvent::At { .. } => "message.at",
            MessageEvent::Reply { .. } => "message.reply",
            MessageEvent::Forward { .. } => "message.forward",
            MessageEvent::File { .. } => "message.file",
            MessageEvent::Location { .. } => "message.location",
            MessageEvent::Sticker { .. } => "message.sticker",
            MessageEvent::Markdown { .. } => "message.markdown",
        }
    }
    
    fn timestamp(&self) -> DateTime<Utc> {
        match self {
            MessageEvent::Text { metadata, .. } => metadata.timestamp,
            MessageEvent::Image { metadata, .. } => metadata.timestamp,
            MessageEvent::Voice { metadata, .. } => metadata.timestamp,
            MessageEvent::Video { metadata, .. } => metadata.timestamp,
            MessageEvent::At { metadata, .. } => metadata.timestamp,
            MessageEvent::Reply { metadata, .. } => metadata.timestamp,
            MessageEvent::Forward { metadata, .. } => metadata.timestamp,
            MessageEvent::File { metadata, .. } => metadata.timestamp,
            MessageEvent::Location { metadata, .. } => metadata.timestamp,
            MessageEvent::Sticker { metadata, .. } => metadata.timestamp,
            MessageEvent::Markdown { metadata, .. } => metadata.timestamp,
        }
    }
    
    fn source(&self) -> EventSource {
        match self {
            MessageEvent::Text { metadata, .. } => metadata.source.clone(),
            MessageEvent::Image { metadata, .. } => metadata.source.clone(),
            MessageEvent::Voice { metadata, .. } => metadata.source.clone(),
            MessageEvent::Video { metadata, .. } => metadata.source.clone(),
            MessageEvent::At { metadata, .. } => metadata.source.clone(),
            MessageEvent::Reply { metadata, .. } => metadata.source.clone(),
            MessageEvent::Forward { metadata, .. } => metadata.source.clone(),
            MessageEvent::File { metadata, .. } => metadata.source.clone(),
            MessageEvent::Location { metadata, .. } => metadata.source.clone(),
            MessageEvent::Sticker { metadata, .. } => metadata.source.clone(),
            MessageEvent::Markdown { metadata, .. } => metadata.source.clone(),
        }
    }
    
    fn user_id(&self) -> Option<&str> {
        match self {
            MessageEvent::Text { metadata, .. } => metadata.user_id.as_deref(),
            MessageEvent::Image { metadata, .. } => metadata.user_id.as_deref(),
            MessageEvent::Voice { metadata, .. } => metadata.user_id.as_deref(),
            MessageEvent::Video { metadata, .. } => metadata.user_id.as_deref(),
            MessageEvent::At { metadata, .. } => metadata.user_id.as_deref(),
            MessageEvent::Reply { metadata, .. } => metadata.user_id.as_deref(),
            MessageEvent::Forward { metadata, .. } => metadata.user_id.as_deref(),
            MessageEvent::File { metadata, .. } => metadata.user_id.as_deref(),
            MessageEvent::Location { metadata, .. } => metadata.user_id.as_deref(),
            MessageEvent::Sticker { metadata, .. } => metadata.user_id.as_deref(),
            MessageEvent::Markdown { metadata, .. } => metadata.user_id.as_deref(),
        }
    }
    
    fn group_id(&self) -> Option<&str> {
        match self {
            MessageEvent::Text { metadata, .. } => metadata.group_id.as_deref(),
            MessageEvent::Image { metadata, .. } => metadata.group_id.as_deref(),
            MessageEvent::Voice { metadata, .. } => metadata.group_id.as_deref(),
            MessageEvent::Video { metadata, .. } => metadata.group_id.as_deref(),
            MessageEvent::At { metadata, .. } => metadata.group_id.as_deref(),
            MessageEvent::Reply { metadata, .. } => metadata.group_id.as_deref(),
            MessageEvent::Forward { metadata, .. } => metadata.group_id.as_deref(),
            MessageEvent::File { metadata, .. } => metadata.group_id.as_deref(),
            MessageEvent::Location { metadata, .. } => metadata.group_id.as_deref(),
            MessageEvent::Sticker { metadata, .. } => metadata.group_id.as_deref(),
            MessageEvent::Markdown { metadata, .. } => metadata.group_id.as_deref(),
        }
    }
    
    fn self_id(&self) -> Option<&str> {
        match self {
            MessageEvent::Text { metadata, .. } => metadata.self_id.as_deref(),
            MessageEvent::Image { metadata, .. } => metadata.self_id.as_deref(),
            MessageEvent::Voice { metadata, .. } => metadata.self_id.as_deref(),
            MessageEvent::Video { metadata, .. } => metadata.self_id.as_deref(),
            MessageEvent::At { metadata, .. } => metadata.self_id.as_deref(),
            MessageEvent::Reply { metadata, .. } => metadata.self_id.as_deref(),
            MessageEvent::Forward { metadata, .. } => metadata.self_id.as_deref(),
            MessageEvent::File { metadata, .. } => metadata.self_id.as_deref(),
            MessageEvent::Location { metadata, .. } => metadata.self_id.as_deref(),
            MessageEvent::Sticker { metadata, .. } => metadata.self_id.as_deref(),
            MessageEvent::Markdown { metadata, .. } => metadata.self_id.as_deref(),
        }
    }
    
    fn correlation_id(&self) -> Option<&str> {
        match self {
            MessageEvent::Text { metadata, .. } => metadata.correlation_id.as_deref(),
            MessageEvent::Image { metadata, .. } => metadata.correlation_id.as_deref(),
            MessageEvent::Voice { metadata, .. } => metadata.correlation_id.as_deref(),
            MessageEvent::Video { metadata, .. } => metadata.correlation_id.as_deref(),
            MessageEvent::At { metadata, .. } => metadata.correlation_id.as_deref(),
            MessageEvent::Reply { metadata, .. } => metadata.correlation_id.as_deref(),
            MessageEvent::Forward { metadata, .. } => metadata.correlation_id.as_deref(),
            MessageEvent::File { metadata, .. } => metadata.correlation_id.as_deref(),
            MessageEvent::Location { metadata, .. } => metadata.correlation_id.as_deref(),
            MessageEvent::Sticker { metadata, .. } => metadata.correlation_id.as_deref(),
            MessageEvent::Markdown { metadata, .. } => metadata.correlation_id.as_deref(),
        }
    }
}

impl MessageEvent {
    /// Get message content (text or fallback to type)
    pub fn content(&self) -> Option<&str> {
        match self {
            MessageEvent::Text { text, .. } => Some(text),
            MessageEvent::Reply { text, .. } => Some(text),
            MessageEvent::Forward { text, .. } => text.as_deref(),
            MessageEvent::Markdown { content, .. } => Some(content),
            MessageEvent::Image { caption, .. } => caption.as_deref(),
            MessageEvent::At { text, .. } => Some(text),
            _ => None,
        }
    }
    
    /// Get media URL if applicable
    pub fn media_url(&self) -> Option<&str> {
        match self {
            MessageEvent::Image { url, .. } => Some(url),
            MessageEvent::Voice { url, .. } => Some(url),
            MessageEvent::Video { url, .. } => Some(url),
            MessageEvent::File { url, .. } => Some(url),
            MessageEvent::Sticker { sticker_id, .. } => Some(sticker_id),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_event_text() {
        let metadata = EventMetadata::new("message.text")
            .with_user_id("user123")
            .with_group_id("group456");
        
        let event = MessageEvent::Text {
            text: "Hello world".to_string(),
            metadata: metadata.clone(),
        };
        
        assert_eq!(event.event_type(), "message.text");
        assert_eq!(event.content(), Some("Hello world"));
        assert_eq!(event.media_url(), None);
    }
    
    #[test]
    fn test_message_event_image() {
        let metadata = EventMetadata::new("message.image");
        
        let event = MessageEvent::Image {
            url: "https://example.com/image.jpg".to_string(),
            caption: Some("A beautiful image".to_string()),
            metadata: metadata.clone(),
        };
        
        assert_eq!(event.event_type(), "message.image");
        assert_eq!(event.content(), Some("A beautiful image"));
        assert_eq!(event.media_url(), Some("https://example.com/image.jpg"));
    }
    
    #[test]
    fn test_message_event_at() {
        let metadata = EventMetadata::new("message.at");
        
        let event = MessageEvent::At {
            text: "@all Hello".to_string(),
            at_list: vec!["user1".to_string(), "user2".to_string()],
            metadata: metadata.clone(),
        };
        
        assert_eq!(event.event_type(), "message.at");
        assert_eq!(event.content(), Some("@all Hello"));
        assert_eq!(event.user_id(), None);
    }
}
