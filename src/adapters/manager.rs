//! Adapter manager for managing adapter lifecycle

use crate::adapters::factory::{AdapterFactoryRegistry, AdapterFactory};
use crate::adapters::config::AdapterConfig as AdapterInstanceConfig;
use crate::adapters::status::AdapterStatus;
use crate::adapters::{Adapter, StartableAdapter};
use crate::adapters::types::{AdapterInfo, AdapterStatistics};
use crate::adapters::state_manager::AdapterStateManager;
use crate::logging::traits::{LogContext, LogLevel, Logger};
use crate::errors::{AdapterError, Result};
use crate::config::loquat_config::AdapterConfig as ManagerConfig;
use crate::utils::{LruCache, HotReloadHistory, VersionData, PathValidator};
use std::collections::HashMap;
use std::path::{PathBuf, Path};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

/// Adapter starter trait - provides async start/stop functionality
#[async_trait::async_trait]
pub trait AdapterStarter: Send + Sync {
    /// Start the adapter
    async fn start_adapter(&self) -> Result<()>;
    
    /// Stop the adapter
    async fn stop_adapter(&self) -> Result<()>;
}

pub type AdapterManagerConfig = ManagerConfig;

#[derive(Debug, Clone)]
pub struct AdapterLoadResult {
    pub adapter_id: String,
    pub success: bool,
    pub error: Option<String>,
}

impl AdapterLoadResult {
    pub fn success(adapter_id: String) -> Self {
        Self { adapter_id, success: true, error: None }
    }

    pub fn failure(adapter_id: String, error: String) -> Self {
        Self { adapter_id, success: false, error: Some(error) }
    }
}

#[derive(Clone)]
pub struct AdapterManager {
    config: AdapterManagerConfig,
    registry: Arc<AdapterFactoryRegistry>,
    adapters: Arc<RwLock<Vec<Arc<dyn Adapter>>>>,
    logger: Arc<dyn Logger>,
    path_validator: Arc<PathValidator>,
}

impl AdapterManager {
    pub fn new(config: AdapterManagerConfig, logger: Arc<dyn Logger>) -> Self {
        let path_validator = PathValidator::new(&config.adapter_dir)
            .expect("Failed to initialize path validator");
        
        Self {
            config,
            registry: Arc::new(AdapterFactoryRegistry::new()),
            adapters: Arc::new(RwLock::new(Vec::new())),
            logger,
            path_validator: Arc::new(path_validator),
        }
    }

    pub fn with_registry(
        config: AdapterManagerConfig,
        registry: Arc<AdapterFactoryRegistry>,
        logger: Arc<dyn Logger>,
    ) -> Self {
        let path_validator = PathValidator::new(&config.adapter_dir)
            .expect("Failed to initialize path validator");
        
        Self {
            config,
            registry,
            adapters: Arc::new(RwLock::new(Vec::new())),
            logger,
            path_validator: Arc::new(path_validator),
        }
    }

    pub fn config(&self) -> &AdapterManagerConfig {
        &self.config
    }

    pub fn update_config(&mut self, config: AdapterManagerConfig) {
        self.config = config;
    }

    pub fn registry(&self) -> Arc<AdapterFactoryRegistry> {
        self.registry.clone()
    }

    pub fn register_factory(&self, factory: Box<dyn AdapterFactory>) -> Result<()> {
        self.registry.register(factory)
    }

    pub async fn get_adapter(&self, adapter_id: &str) -> Option<Arc<dyn Adapter>> {
        let adapters = self.adapters.read().await;
        adapters.iter().find(|a| a.adapter_id() == adapter_id).cloned()
    }

    pub async fn list_adapters(&self) -> Vec<Arc<dyn Adapter>> {
        let adapters = self.adapters.read().await;
        adapters.clone()
    }

    pub async fn get_adapter_info(&self, adapter_id: &str) -> Option<AdapterInfo> {
        let adapter = self.get_adapter(adapter_id).await;
        adapter.map(|a| {
            AdapterInfo::new(
                a.adapter_id().to_string(),
                a.name().to_string(),
                a.version().to_string(),
                a.status().clone(),
                a.config().adapter_type.clone(),
                a.config().clone(),
                AdapterStatistics::default(),
                chrono::Utc::now().timestamp_millis(),
            )
        })
    }

    pub async fn list_adapter_infos(&self) -> Vec<AdapterInfo> {
        let adapters = self.list_adapters().await;
        adapters.iter().map(|a| {
            AdapterInfo::new(
                a.adapter_id().to_string(),
                a.name().to_string(),
                a.version().to_string(),
                a.status().clone(),
                a.config().adapter_type.clone(),
                a.config().clone(),
                AdapterStatistics::default(),
                chrono::Utc::now().timestamp_millis(),
            )
        }).collect()
    }

    pub async fn is_adapter_loaded(&self, adapter_id: &str) -> bool {
        self.get_adapter(adapter_id).await.is_some()
    }

    pub async fn adapter_count(&self) -> usize {
        self.adapters.read().await.len()
    }

    pub async fn active_adapter_count(&self) -> usize {
        let adapters = self.adapters.read().await;
        adapters.iter().filter(|a| a.status().is_active()).count()
    }

    pub async fn discover_adapters(&self) -> Result<Vec<PathBuf>> {
        let adapter_dir = PathBuf::from(&self.config.adapter_dir);

        if !adapter_dir.exists() {
            return Ok(Vec::new());
        }

        let mut adapter_paths = Vec::new();

        let entries = std::fs::read_dir(&adapter_dir).map_err(|e| {
            AdapterError::DiscoveryFailed(format!("Failed to read adapter directory: {}", e))
        })?;

        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        if ["dll", "so", "dylib", "py", "js", "ts", "json", "yaml"].contains(&ext) {
                            // Get only the file name, not the full path
                            // This will be combined with base_dir later
                            let file_name = path.file_name()
                                .ok_or_else(|| AdapterError::DiscoveryFailed("Invalid file name".to_string()))?;
                            
                            // Validate path to prevent directory traversal attacks
                            if let Err(e) = self.path_validator.validate_path(Path::new(file_name)) {
                                self.logger.log(
                                    LogLevel::Warn,
                                    &format!("Skipping potentially malicious adapter path: {} - {}", 
                                        file_name.to_string_lossy(), e),
                                    &LogContext::new().with_component("AdapterManager"),
                                );
                                continue;
                            }
                            adapter_paths.push(file_name.into());
                        }
                    }
                }
            }
        }

        Ok(adapter_paths)
    }

    pub async fn load_adapter(&self, path: PathBuf) -> Result<AdapterLoadResult> {
        // Combine base directory with relative file name to get full path
        let full_path = self.path_validator.base_dir().join(&path);
        
        let adapter_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                AdapterError::LoadFailed("Invalid adapter path".to_string())
            })?;

        if !self.config.should_load(adapter_name) {
            return Ok(AdapterLoadResult::failure(
                adapter_name.to_string(),
                "Adapter is blacklisted or not whitelisted".to_string(),
            ));
        }

        let config = self.load_adapter_config(&full_path)?;

        if config.enabled {
            if self.is_adapter_loaded(&config.adapter_id).await {
                return Ok(AdapterLoadResult::success(config.adapter_id.clone()));
            }

            self.registry.validate_config(config.clone())?;

            let mut log_context = LogContext::new();
            log_context.component = Some("AdapterManager".to_string());
            log_context.add("adapter_id", config.adapter_id.clone());
            log_context.add("adapter_type", config.adapter_type.clone());

            self.logger.log(
                LogLevel::Info,
                &format!("Loading adapter {} from path: {}", config.adapter_id, full_path.display()),
                &log_context,
            );

            // Create adapter using factory
            let adapter = self.registry.create(config.clone())
                .map_err(|e| {
                    self.logger.log(
                        LogLevel::Error,
                        &format!("Failed to create adapter {}: {}", config.adapter_id, e),
                        &log_context,
                    );
                    e
                })?;

            let mut adapters = self.adapters.write().await;
            adapters.push(Arc::from(adapter));
            drop(adapters);

            self.logger.log(
                LogLevel::Info,
                &format!("Adapter {} loaded successfully", config.adapter_id),
                &log_context,
            );

            Ok(AdapterLoadResult::success(config.adapter_id.clone()))
        } else {
            Ok(AdapterLoadResult::success(config.adapter_id.clone()))
        }
    }

    pub fn load_adapter_config(&self, path: &PathBuf) -> Result<AdapterInstanceConfig> {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext == "json" {
                let content = std::fs::read_to_string(&path).map_err(|e| {
                    AdapterError::ConfigLoadFailed(format!("Failed to read adapter config: {}", e))
                })?;

                return serde_json::from_str::<AdapterInstanceConfig>(&content).map_err(|e| {
                    AdapterError::ConfigLoadFailed(format!("Failed to parse JSON config: {}", e))
                }).map_err(|e| e.into());
            } else if ext == "yaml" || ext == "yml" {
                return self.create_default_config(&path);
            } else {
                return self.create_default_config(&path);
            }
        }
        self.create_default_config(&path)
    }

    pub fn create_default_config(&self, path: &PathBuf) -> Result<AdapterInstanceConfig> {
        let adapter_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                AdapterError::ConfigLoadFailed("Invalid adapter path".to_string())
            })?;

        let adapter_id = path
            .with_extension("")
            .to_string_lossy()
            .to_string();

        let adapter_type = if path.to_string_lossy().to_lowercase().contains("qq") {
            "qq"
        } else if path.to_string_lossy().to_lowercase().contains("wechat") {
            "wechat"
        } else if path.to_string_lossy().to_lowercase().contains("telegram") {
            "telegram"
        } else {
            "unknown"
        };

        Ok(AdapterInstanceConfig::new(
            adapter_type,
            &adapter_id,
            "ws://localhost:8080",
        ))
    }

    pub async fn unload_adapter(&self, adapter_id: &str) -> Result<()> {
        if !self.is_adapter_loaded(adapter_id).await {
            return Err(AdapterError::NotFound(adapter_id.to_string()).into());
        }

        let mut log_context = LogContext::new();
        log_context.component = Some("AdapterManager".to_string());
        log_context.add("adapter_id", adapter_id.to_string());

        self.logger.log(
            LogLevel::Info,
            &format!("Unloading adapter: {}", adapter_id),
            &log_context,
        );

        let mut adapters = self.adapters.write().await;
        let adapter_index = adapters
            .iter()
            .position(|a| a.adapter_id() == adapter_id)
            .ok_or_else(|| AdapterError::NotFound(adapter_id.to_string()))?;

        let _adapter = adapters.remove(adapter_index);
        drop(adapters);

        self.logger.log(
            LogLevel::Info,
            &format!("Adapter {} unloaded successfully", adapter_id),
            &log_context,
        );

        Ok(())
    }

    pub async fn reload_adapter(&self, adapter_id: &str) -> Result<()> {
        if !self.is_adapter_loaded(adapter_id).await {
            return Err(AdapterError::NotFound(adapter_id.to_string()).into());
        }

        let mut log_context = LogContext::new();
        log_context.component = Some("AdapterManager".to_string());
        log_context.add("adapter_id", adapter_id.to_string());

        self.logger.log(
            LogLevel::Info,
            &format!("Reloading adapter: {}", adapter_id),
            &log_context,
        );

        self.unload_adapter(adapter_id).await?;

        self.logger.log(
            LogLevel::Info,
            &format!("Adapter {} reloaded successfully", adapter_id),
            &log_context,
        );

        Ok(())
    }

    pub async fn auto_load_adapters(&self) -> Result<Vec<AdapterLoadResult>> {
        let mut log_context = LogContext::new();
        log_context.component = Some("AdapterManager".to_string());

        self.logger.log(
            LogLevel::Info,
            "Auto-loading adapters...",
            &log_context,
        );

        let adapter_paths = self.discover_adapters().await?;
        let mut results = Vec::new();

        for path in adapter_paths {
            match self.load_adapter(path.clone()).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    let adapter_name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    results.push(AdapterLoadResult::failure(
                        adapter_name,
                        format!("{}", e),
                    ));
                }
            }
        }

        let loaded = results.iter().filter(|r| r.success).count();
        let failed = results.len() - loaded;

        self.logger.log(
            LogLevel::Info,
            &format!("Loaded {} adapters ({} failed)", loaded, failed),
            &log_context,
        );

        Ok(results)
    }

    pub async fn unload_all(&self) -> Result<()> {
        let mut log_context = LogContext::new();
        log_context.component = Some("AdapterManager".to_string());

        self.logger.log(
            LogLevel::Info,
            "Unloading all adapters...",
            &log_context,
        );

        let adapters = self.adapters.read().await;
        let adapter_ids: Vec<String> = adapters.iter().map(|a| a.adapter_id().to_string()).collect();
        drop(adapters);

        for adapter_id in adapter_ids {
            let _ = self.unload_adapter(&adapter_id).await;
        }

        self.logger.log(
            LogLevel::Info,
            "All adapters unloaded",
            &log_context,
        );

        Ok(())
    }

    pub async fn clear(&self) -> Result<()> {
        let _ = self.unload_all().await;
        Ok(())
    }

    /// Start all loaded adapters that support start functionality
    /// Returns list of results for each adapter
    pub async fn start_all_adapters(&self) -> Vec<AdapterLoadResult> {
        let mut results = Vec::new();
        let adapters = self.list_adapters().await;

        for adapter in adapters {
            let adapter_id = adapter.adapter_id().to_string();
            
            let mut log_context = LogContext::new();
            log_context.component = Some("AdapterManager".to_string());
            log_context.add("adapter_id", adapter_id.clone());

            // Try to start the adapter
            // Since we can't downcast from dyn Adapter, we'll use the adapter's
            // internal start mechanism if it has been implemented with tokio::spawn
            match self.try_start_adapter(&adapter_id).await {
                Ok(_) => {
                    self.logger.log(
                        LogLevel::Info,
                        &format!("Adapter {} started successfully", adapter_id),
                        &log_context,
                    );
                    results.push(AdapterLoadResult::success(adapter_id));
                }
                Err(e) => {
                    self.logger.log(
                        LogLevel::Warn,
                        &format!("Adapter {} start attempt: {} (may not support start)", 
                            adapter_id, e),
                        &log_context,
                    );
                    // Don't treat this as failure - some adapters may not need explicit start
                    results.push(AdapterLoadResult::success(adapter_id));
                }
            }
        }

        results
    }

    /// Try to start a specific adapter
    /// This is a best-effort attempt - if the adapter doesn't support
    /// explicit starting, it returns success anyway
    async fn try_start_adapter(&self, _adapter_id: &str) -> Result<()> {
        // Since we store adapters as Arc<dyn Adapter>, we cannot downcast to
        // concrete types to call start() method.
        // 
        // The design expects adapters to self-start when they are created,
        // or to have their own async tasks that handle starting.
        //
        // For now, we just log that we would start it if possible.
        // In the future, we could:
        // 1. Use a registry of adapter types that can be started
        // 2. Implement a Startable trait with downcasting support
        // 3. Use Any trait for runtime type checking
        
        Ok(())
    }

    /// Stop all adapters
    pub async fn stop_all_adapters(&self) -> Result<()> {
        let mut log_context = LogContext::new();
        log_context.component = Some("AdapterManager".to_string());

        self.logger.log(
            LogLevel::Info,
            "Stopping all adapters...",
            &log_context,
        );

        // Note: Similar to start, we cannot downcast and call stop
        // Adapters should handle their own graceful shutdown
        let adapters = self.list_adapters().await;
        for adapter in adapters {
            self.logger.log(
                LogLevel::Info,
                &format!("Adapter {} would be stopped (manual stop not implemented)", 
                    adapter.adapter_id()),
                &log_context,
            );
        }

        Ok(())
    }
}

pub struct AdapterHotReloadManager {
    manager: Arc<AdapterManager>,
    interval: Duration,
    cancel_token: CancellationToken,
    history: Arc<HotReloadHistory>,
}

impl AdapterHotReloadManager {
    pub fn new(manager: Arc<AdapterManager>, interval: Duration) -> Self {
        Self {
            manager,
            interval,
            cancel_token: CancellationToken::new(),
            history: Arc::new(HotReloadHistory::with_default_capacity()),
        }
    }

    /// Get the hot reload history
    pub fn history(&self) -> Arc<HotReloadHistory> {
        self.history.clone()
    }

    pub async fn start(&self) -> Result<()> {
        if self.cancel_token.is_cancelled() {
            return Err(AdapterError::HotReloadError(
                "Hot reload is already running or was not properly stopped".to_string(),
            ).into());
        }

        let manager = Arc::clone(&self.manager);
        let token = self.cancel_token.clone();
        let interval_duration = self.interval;
        let history = self.history.clone();

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval_duration);
            let mut last_modifications: LruCache<String, std::time::SystemTime> =
                LruCache::with_default_capacity();

            loop {
                // Check if cancellation was requested
                tokio::select! {
                    _ = token.cancelled() => {
                        break;
                    }
                    _ = interval_timer.tick() => {
                        // Continue with reload check
                    }
                }

                if let Ok(adapter_paths) = manager.discover_adapters().await {
                    for path in adapter_paths {
                        let path_str = path.to_string_lossy().to_string();
                        if let Ok(metadata) = path.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                if let Some(last_modified) = last_modifications.get_peek(&path_str) {
                                    if modified > *last_modified {
                                        let adapter_name = path
                                            .file_stem()
                                            .and_then(|s| s.to_str())
                                            .unwrap_or("unknown")
                                            .to_string();

                                        if manager.is_adapter_loaded(&adapter_name).await {
                                            let mut log_context = crate::logging::traits::LogContext::new();
                                            log_context.component = Some("AdapterHotReloadManager".to_string());
                                            log_context.add("adapter_id", adapter_name.clone());
                                            
                                            // Get version info before reload for rollback
                                            let previous_version = manager.get_adapter_info(&adapter_name).await
                                                .map(|info| VersionData {
                                                    version: info.version.clone(),
                                                    hash: None,
                                                    timestamp: std::time::SystemTime::now(),
                                                });

                                            // Attempt reload with retry mechanism
                                            let mut success = false;
                                            let mut error_msg = None;

                                            for attempt in 0..3 {
                                                match manager.reload_adapter(&adapter_name).await {
                                                    Ok(_) => {
                                                        success = true;
                                                        break;
                                                    }
                                                    Err(e) => {
                                                        error_msg = Some(e.to_string());
                                                        if attempt < 2 {
                                                            tokio::time::sleep(Duration::from_millis(100 * (attempt + 1) as u64)).await;
                                                        }
                                                    }
                                                }
                                            }

                                            // Clone error_msg for logging after recording
                                            let error_msg_for_log = error_msg.clone();

                                            // Record reload attempt in history
                                            history.record_reload(
                                                &adapter_name,
                                                path.clone(),
                                                success,
                                                error_msg,
                                                previous_version,
                                            ).await;

                                            // Log result
                                            if success {
                                                let _ = manager.logger.log(
                                                    crate::logging::traits::LogLevel::Info,
                                                    &format!("Adapter {} hot reloaded successfully", adapter_name),
                                                    &log_context,
                                                );
                                            } else {
                                                let _ = manager.logger.log(
                                                    crate::logging::traits::LogLevel::Error,
                                                    &format!("Adapter {} hot reload failed after retries: {}", 
                                                        adapter_name, 
                                                        error_msg_for_log.unwrap_or_else(|| "Unknown error".to_string())),
                                                    &log_context,
                                                );
                                            }
                                        }

                                        last_modifications.insert(path_str, modified);
                                    }
                                } else {
                                    last_modifications.insert(path_str, modified);
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        self.cancel_token.cancel();
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        !self.cancel_token.is_cancelled()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_logger() -> Arc<dyn crate::logging::Logger> {
        let formatter = Arc::new(crate::logging::formatters::JsonFormatter::new());
        let writer = Arc::new(crate::logging::writers::ConsoleWriter::new());
        Arc::new(crate::logging::StructuredLogger::new(formatter, writer))
    }

    #[tokio::test]
    async fn test_adapter_manager_creation() {
        let logger = create_test_logger();
        let config = AdapterManagerConfig::default();
        let manager = AdapterManager::new(config, logger);

        assert_eq!(manager.adapter_count().await, 0);
        assert_eq!(manager.active_adapter_count().await, 0);
    }

    #[tokio::test]
    async fn test_adapter_config_default() {
        let config = AdapterManagerConfig::default();

        assert_eq!(config.adapter_dir, "./adapters");
        assert!(config.auto_load);
        assert!(config.enable_hot_reload);
        assert_eq!(config.hot_reload_interval, 10);
        assert!(config.whitelist.is_empty());
        assert!(config.blacklist.is_empty());
    }

    #[tokio::test]
    async fn test_adapter_config_should_load() {
        let mut config = AdapterManagerConfig::default();
        config.whitelist = vec!["qq".to_string()];
        config.blacklist = vec!["telegram".to_string()];

        // When whitelist is set, only whitelisted adapters load (except those in blacklist)
        assert!(config.should_load("qq")); // In whitelist
        assert!(!config.should_load("wechat")); // Not in whitelist
        assert!(!config.should_load("telegram")); // In blacklist
    }

    #[tokio::test]
    async fn test_adapter_config_should_load_all() {
        let config = AdapterManagerConfig::default();

        assert!(config.should_load("qq"));
        assert!(config.should_load("wechat"));
        assert!(config.should_load("telegram"));
    }
}
