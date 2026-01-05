//! REPL Context - Provides access to application components

use crate::config::LoquatConfig;
use crate::engine::traits::Engine;
use crate::logging::traits::Logger;
use crate::adapters::core::manager::AdapterManager;
use crate::plugins::PluginManager;
use std::sync::Arc;

/// REPL Context - holds references to core application components
#[derive(Clone)]
pub struct ReplContext {
    /// Plugin manager reference
    pub plugin_manager: Option<Arc<PluginManager>>,
    /// Adapter manager reference
    pub adapter_manager: Option<Arc<AdapterManager>>,
    /// Engine reference
    pub engine: Option<Arc<dyn Engine>>,
    /// Logger reference
    pub logger: Arc<dyn Logger>,
    /// Configuration
    pub config: LoquatConfig,
    /// Application start time
    pub start_time: std::time::Instant,
}

impl ReplContext {
    /// Create a new REPL context
    pub fn new(
        plugin_manager: Option<Arc<PluginManager>>,
        adapter_manager: Option<Arc<AdapterManager>>,
        engine: Option<Arc<dyn Engine>>,
        logger: Arc<dyn Logger>,
        config: LoquatConfig,
        start_time: std::time::Instant,
    ) -> Self {
        Self {
            plugin_manager,
            adapter_manager,
            engine,
            logger,
            config,
            start_time,
        }
    }
}

impl std::fmt::Debug for ReplContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplContext")
            .field("plugin_manager", &self.plugin_manager.is_some())
            .field("adapter_manager", &self.adapter_manager.is_some())
            .field("engine", &self.engine.is_some())
            .field("logger", &"<Logger>")
            .field("config", &self.config)
            .field("start_time", &self.start_time)
            .finish()
    }
}
