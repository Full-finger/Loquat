//! Echo Adapter implementation
//!
//! This adapter echoes back received messages.

pub mod adapter;
pub mod factory;

pub use adapter::EchoAdapter;
pub use factory::EchoAdapterFactory;
