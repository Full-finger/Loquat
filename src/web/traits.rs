//! Web service traits

use crate::errors::{Result, WebError};

/// Web service trait
#[async_trait::async_trait]
pub trait WebServiceTrait: Send + Sync {
    /// Start the web service
    async fn start(&self) -> Result<()>;

    /// Stop the web service
    async fn stop(&self) -> Result<()>;

    /// Check if the service is running
    fn is_running(&self) -> bool;

    /// Get the service address
    fn address(&self) -> String;
}

/// Application state that can be shared with web handlers
#[derive(Clone)]
pub struct AppState {
    /// Plugin manager reference
    pub plugin_manager: Option<crate::plugins::PluginManager>,
    /// Adapter manager reference
    pub adapter_manager: Option<crate::adapters::AdapterManager>,
    /// Logger
    pub logger: std::sync::Arc<dyn crate::logging::traits::Logger>,
    /// Config
    pub config: crate::config::loquat_config::LoquatConfig,
    /// Start time for uptime calculation
    pub start_time: std::time::Instant,
}
