//! Pools module for Loquat framework
//!
//! Pools manage workers and process packages through them.
//! There are 9 pool types that process packages in sequence.

pub mod traits;
pub mod types;
pub mod standard_pool;
pub mod validator;

// Re-export Pool trait for external use
pub use traits::Pool;
pub use types::*;
pub use standard_pool::*;
