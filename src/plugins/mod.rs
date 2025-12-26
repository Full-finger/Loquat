//! Plugin system for Loquat framework
//! 
//! Supports Rust/Python/JS plugins with hot-reload capability

pub mod types;
pub mod traits;
pub mod registry;
pub mod loader;
pub mod manager;

// Export all types
pub use types::PluginInfo;
pub use types::PluginLoadResult;
pub use types::PluginStatus;
pub use types::PluginType;
pub use types::PluginMetadata;

// Export other modules
pub use traits::*;
pub use registry::*;
pub use loader::*;

// Export plugin_manager (from manager module)
pub use manager::PluginManager;
pub use manager::HotReloadManager;
