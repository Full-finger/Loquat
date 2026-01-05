//! Console Adapter implementation
//!
//! This adapter reads input from stdin and writes output to stdout.

pub mod adapter;
pub mod factory;

pub use adapter::ConsoleAdapter;
pub use factory::ConsoleAdapterFactory;
