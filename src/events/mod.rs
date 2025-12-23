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

pub use traits::*;
pub use package::*;
pub use message::*;
pub use notice::*;
pub use request::*;
pub use meta::*;
pub use target_site::*;
pub use block::*;
pub use group::*;
pub use event_enum::*;

/// Re-export commonly used types
pub use crate::events::package::Package;
pub use crate::events::traits::Event;
pub use crate::events::event_enum::{Status, EventEnum};
