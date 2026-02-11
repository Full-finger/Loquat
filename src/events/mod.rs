//! Event module for Loquat framework
//! 
//! Provides stream-based event processing with Package/Block/Group hierarchy,
//! designed for instant messaging scenarios similar to onebot/napcat.

pub mod traits;
pub mod package;
pub mod message;
pub mod notice;
pub mod request;
pub mod meta;
pub mod target_site;
pub mod block;
pub mod group;
pub mod event_enum;

// New event architecture modules
pub mod payloads_legacy; // Legacy event payloads (MessagePayload, NoticePayload, RequestPayload)
pub mod event;

// Re-export commonly used types
pub use package::*;
pub use message::*;
pub use notice::*;
pub use request::*;
pub use meta::*;
pub use target_site::*;
pub use block::*;
pub use group::*;
pub use event_enum::*;
pub use traits::*;

// Re-export commonly used types explicitly
pub use crate::events::package::Package;
pub use crate::events::traits::{Event, EventSource, EventMetadata};
pub use crate::events::event_enum::{Status, EventEnum};

/// New unified event type (preferred) - separates events into Simple and Group
pub use crate::events::event::UnifiedEvent;

// ============================================================================
// Legacy Payloads (Deprecated - will be removed in v3.0)
// ============================================================================

/// Legacy event payloads (deprecated - use crate::payloads instead)
/// 
/// # Migration Guide
/// Old code:
/// ```rust,ignore
/// use loquat::events::MessagePayload;
/// ```
/// 
/// New code:
/// ```rust,ignore
/// use loquat::payloads::TextPayload;
/// ```
#[deprecated(since = "0.2.0", note = "Use crate::payloads module instead")]
pub mod payloads {
    // Re-export from payloads_legacy for backward compatibility
    pub use super::payloads_legacy::*;
}
