//! Web service traits

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Web service trait
#[async_trait::async_trait]
pub trait WebServiceTrait: Send + Sync {
    /// Start the web service
    async fn start(&self) -> crate::errors::Result<()>;

    /// Stop the web service
    async fn stop(&self) -> crate::errors::Result<()>;

    /// Check if the service is running
    fn is_running(&self) -> bool;

    /// Get the service address
    fn address(&self) -> String;
}

/// Error tracker for monitoring system errors
#[derive(Clone, Debug)]
pub struct ErrorTracker {
    total_errors: Arc<AtomicU64>,
    critical_errors: Arc<AtomicU64>,
    last_error: Arc<std::sync::RwLock<Option<chrono::DateTime<chrono::Utc>>>>,
    last_critical: Arc<std::sync::RwLock<Option<chrono::DateTime<chrono::Utc>>>>,
}

impl ErrorTracker {
    /// Create a new error tracker
    pub fn new() -> Self {
        Self {
            total_errors: Arc::new(AtomicU64::new(0)),
            critical_errors: Arc::new(AtomicU64::new(0)),
            last_error: Arc::new(std::sync::RwLock::new(None)),
            last_critical: Arc::new(std::sync::RwLock::new(None)),
        }
    }

    /// Record a regular error
    pub fn record_error(&self) {
        self.total_errors.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut last) = self.last_error.write() {
            *last = Some(chrono::Utc::now());
        }
    }

    /// Record a critical error
    pub fn record_critical(&self) {
        self.critical_errors.fetch_add(1, Ordering::SeqCst);
        self.record_error();
        if let Ok(mut last) = self.last_critical.write() {
            *last = Some(chrono::Utc::now());
        }
    }

    /// Get error statistics
    pub fn get_stats(&self) -> crate::web::types::ErrorStats {
        let total = self.total_errors.load(Ordering::SeqCst);
        let critical = self.critical_errors.load(Ordering::SeqCst);
        let last_error = self.last_error.read().ok().and_then(|t| *t);
        let last_critical = self.last_critical.read().ok().and_then(|t| *t);

        crate::web::types::ErrorStats {
            total,
            critical,
            last_error,
            last_critical,
        }
    }

    /// Reset error counters
    pub fn reset(&self) {
        self.total_errors.store(0, Ordering::SeqCst);
        self.critical_errors.store(0, Ordering::SeqCst);
        if let Ok(mut last) = self.last_error.write() {
            *last = None;
        }
        if let Ok(mut last) = self.last_critical.write() {
            *last = None;
        }
    }
}

impl Default for ErrorTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Application state that can be shared with web handlers
#[derive(Clone)]
pub struct AppState {
    /// Plugin manager reference
    pub plugin_manager: Option<crate::plugins::PluginManager>,
    /// Adapter manager reference
    pub adapter_manager: Option<crate::adapters::AdapterManager>,
    /// Engine reference
    pub engine: Option<crate::engine::StandardEngine>,
    /// Logger
    pub logger: std::sync::Arc<dyn crate::logging::traits::Logger>,
    /// Config
    pub config: crate::config::loquat_config::LoquatConfig,
    /// Start time for uptime calculation
    pub start_time: std::time::Instant,
    /// Error tracker
    pub error_tracker: ErrorTracker,
    /// Web service running status
    pub web_running: Arc<std::sync::atomic::AtomicBool>,
}
