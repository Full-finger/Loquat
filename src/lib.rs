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
pub mod payloads;
pub mod adapters;
pub mod workers;
pub mod pools;
pub mod channels;
pub mod streams;
pub mod routers;
pub mod plugins;
pub mod channel_manager;
pub mod engine;
pub mod shutdown;
pub mod utils;
pub mod cli;
pub mod repl;
pub mod tui;
pub mod database;

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
pub use shutdown::*;
pub use database::*;

/// Re-export common types for convenience
pub mod prelude {
    pub use crate::aop::{Aspect, Proxyable};
    pub use crate::logging::{Logger, LogLevel, LogContext};
    pub use crate::errors::{LoquatError, Result};
    pub use crate::events::{Event, EventMetadata, EventSource, Package};
    pub use crate::plugins::{Plugin, PluginManager, PluginType};
    pub use crate::config::loquat_config::PluginConfig;
    pub use crate::engine::{Engine, EngineConfig, EngineStats, EngineState};
    pub use crate::database::{
        DatabaseConnection, DatabaseConfig,
        EventRepository, PluginRepository, AdapterRepository, LogRepository
    };
}
