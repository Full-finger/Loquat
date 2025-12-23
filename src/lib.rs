//! Loquat - A clean Rust web service framework with AOP and logging support
//!
//! This library provides a clean architecture for building web services with
//! Aspect-Oriented Programming (AOP) and comprehensive logging capabilities.

pub mod aop;
pub mod config;
pub mod errors;
pub mod logging;
pub mod web;
pub mod events;
pub mod adapters;

pub use aop::*;
pub use config::*;
pub use errors::*;
pub use logging::*;
pub use web::*;
pub use events::*;
pub use adapters::*;

/// Re-export common types for convenience
pub mod prelude {
    pub use crate::aop::{Aspect, Proxyable};
    pub use crate::logging::{Logger, LogLevel, LogContext};
    pub use crate::errors::{LoquatError, Result};
    pub use crate::events::{Event, EventMetadata, EventSource, Package};
}
