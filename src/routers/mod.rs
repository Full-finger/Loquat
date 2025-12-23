//! Routing module for directing packages to appropriate channels and adapters

pub mod types;
pub mod traits;
pub mod router;

pub use types::*;
pub use traits::*;
pub use router::*;
