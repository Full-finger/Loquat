//! Channel Manager module for managing multiple channel instances
//! 
//! Manages dynamic creation and reuse of channels based on group_id/user_id/channel_id

pub mod types;
pub mod traits;
pub mod manager;

pub use types::*;
pub use traits::*;
pub use manager::*;
