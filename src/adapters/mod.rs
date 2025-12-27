//! Adapter module for Loquat framework
//! 
//! Provides a unified interface for integrating different messaging platforms
//! (QQ, WeChat, Telegram, etc.) into Loquat event system.

pub mod traits;
pub mod config;
pub mod status;
pub mod converter;
pub mod factory;
pub mod manager;
pub mod types;
pub mod console_adapter;
pub mod console_factory;
pub mod echo_adapter;
pub mod echo_factory;
pub mod state_manager;

pub use traits::*;
pub use config::*;
pub use status::*;
pub use converter::*;
pub use factory::*;
pub use manager::*;
pub use types::*;
pub use console_adapter::*;
pub use console_factory::*;
pub use echo_adapter::*;
pub use echo_factory::*;
