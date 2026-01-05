//! Mock Test Adapter implementation
//!
//! This adapter generates test events periodically.

pub mod adapter;
pub mod factory;

pub use adapter::MockTestAdapter;
pub use factory::MockTestFactory;
