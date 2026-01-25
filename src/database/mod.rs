//! Database module for Loquat framework
//! 
//! Provides SQLite-based persistence for events, plugins, adapters, and logs

pub mod connection;
pub mod models;
pub mod repository;

// Re-export commonly used types
pub use connection::{DatabaseConnection, DatabaseConfig};
pub use models::*;
pub use repository::{EventRepository, PluginRepository, AdapterRepository, LogRepository};

/// Database module version
pub const DB_VERSION: &str = "1.0.0";
