//! Core adapter interfaces and types
//!
//! This module contains core traits, types, and management
//! logic for adapter system.

pub mod traits;
pub mod factory;
pub mod manager;
pub mod config;
pub mod status;
pub mod types;
pub mod state_manager;

pub use traits::*;
pub use factory::*;
pub use manager::*;
pub use config::*;
pub use status::*;
pub use types::*;
pub use state_manager::*;
