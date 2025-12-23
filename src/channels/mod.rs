//! Channels module for Loquat framework
//!
//! Channels represent abstract communication channels like group chats or private messages.
//! Each channel has a unique stream that processes packages through 9 pools.

pub mod types;
pub mod channel;

pub use types::*;
pub use channel::*;
