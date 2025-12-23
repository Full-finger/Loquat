//! Streams module for Loquat framework
//!
//! Streams process packages through 9 pools in sequence.
//! Each channel has a unique stream.

pub mod traits;
pub mod standard_stream;
pub mod processor;

pub use traits::*;
pub use standard_stream::*;
