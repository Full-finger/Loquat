//! Adapter module for Loquat framework
//! 
//! Provides a unified interface for integrating different messaging platforms
//! (QQ, WeChat, Telegram, etc.) into Loquat event system.

// Core module with base types and traits
pub mod core;

// Adapter implementations
pub mod napcat;

// Actor-based adapter support
pub mod actor;

// Utility functions
pub mod utils;

// Re-export core types for convenience
pub use core::*;

// Re-export adapter implementations
pub use napcat::*;

// Re-export actor module
pub use actor::*;

// Re-export utilities
pub use utils::*;
