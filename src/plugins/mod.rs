//! Plugin system for Loquat framework
//! 
//! Supports Rust/Python/JS plugins with hot-reload capability

pub mod types;
pub mod traits;
pub mod registry;
pub mod loader;
pub mod manager;
pub mod plugin_manager;

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

// Export plugin_manager (new module without namespace conflict)
pub use plugin_manager::PluginManager;
pub use plugin_manager::HotReloadManager;
