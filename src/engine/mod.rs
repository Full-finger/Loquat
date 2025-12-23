//! Loquat Engine - Core orchestration component
//!
//! The Loquat Engine is the core coordinator that orchestrates all modules:
//! - Receives input Package
//! - Routes to adapter target via Router
//! - Gets/creates Channel via ChannelManager
//! - Processes Package via Stream
//! - Outputs result

pub mod types;
pub mod traits;
pub mod engine;

pub use types::*;
pub use traits::*;
pub use engine::*;
