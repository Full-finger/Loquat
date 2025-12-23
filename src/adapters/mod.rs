//! Adapter module for Loquat framework
//!
//! Provides a unified interface for integrating different messaging platforms
//! (QQ, WeChat, Telegram, etc.) into the Loquat event system.

pub mod traits;
pub mod config;
pub mod status;
pub mod converter;
pub mod factory;

pub use traits::*;
pub use config::*;
pub use status::*;
pub use converter::*;
pub use factory::*;
