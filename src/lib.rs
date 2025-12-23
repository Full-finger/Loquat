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
pub mod workers;
pub mod pools;
pub mod channels;
pub mod streams;
pub mod routers;
pub mod plugins;
pub mod channel_manager;
pub mod engine;

pub use aop::*;
pub use config::*;
pub use errors::*;
pub use logging::*;
pub use web::*;
pub use events::*;
pub use adapters::*;
pub use workers::*;
pub use pools::*;
pub use channels::*;
pub use streams::*;
pub use routers::*;
pub use plugins::*;
pub use channel_manager::*;
pub use engine::*;

/// Re-export common types for convenience
pub mod prelude {
    pub use crate::aop::{Aspect, Proxyable};
    pub use crate::logging::{Logger, LogLevel, LogContext};
    pub use crate::errors::{LoquatError, Result};
    pub use crate::events::{Event, EventMetadata, EventSource, Package};
    pub use crate::plugins::{Plugin, PluginManager, PluginType, PluginConfig};
    pub use crate::engine::{Engine, EngineConfig, EngineStats, EngineState};
}
