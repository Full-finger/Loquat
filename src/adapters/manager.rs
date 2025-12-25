//! Adapter manager for managing adapter lifecycle

use crate::adapters::factory::{AdapterFactoryRegistry, AdapterFactory};
use crate::adapters::config::AdapterConfig;
use crate::adapters::status::AdapterStatus;
use crate::adapters::{Adapter};
use crate::adapters::types::{AdapterInfo, AdapterStatistics};
use crate::logging::traits::{LogContext, LogLevel, Logger};
use crate::errors::{AdapterError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Adapter configuration for AdapterManager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterManagerConfig {
    pub adapter_dir: String,
    pub auto_load: bool,
    pub enable_hot_reload: bool,
    pub hot_reload_interval: u64,
    pub whitelist: Vec<String>,
    pub blacklist: Vec<String>,
    pub adapters: HashMap<String, AdapterConfig>,
}

impl Default for AdapterManagerConfig {
    fn default() -> Self {
        Self {
            adapter_dir: "./adapters".to_string(),
            auto_load: true,
            enable_hot_reload: true,
            hot_reload_interval: 10,
            whitelist: Vec::new(),
            blacklist: Vec::new(),
            adapters: HashMap::new(),
        }
    }
}

impl AdapterManagerConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_adapter_dir(mut self, dir: &str) -> Self {
        self.adapter_dir = dir.to_string();
        self
    }

    pub fn with_auto_load(mut self, enabled: bool) -> Self {
        self.auto_load = enabled;
        self
    }

    pub fn with_hot_reload(mut self, enabled: bool) -> Self {
        self.enable_hot_reload = enabled;
        self
    }

    pub fn with_hot_reload_interval(mut self, seconds: u64) -> Self {
        self.hot_reload_interval = seconds;
        self
    }

    pub fn with_whitelist(mut self, list: Vec<String>) -> Self {
        self.whitelist = list;
        self
    }

    pub fn with_blacklist(mut self, list: Vec<String>) -> Self {
        self.blacklist = list;
        self
    }

    pub fn with_adapter(mut self, adapter_id: &str, config: AdapterConfig) -> Self {
        self.adapters.insert(adapter_id.to_string(), config);
        self
    }

    pub fn should_load(&self, adapter_id: &str) -> bool {
        if self.blacklist.contains(&adapter_id.to_string()) {
            return false;
        }
        if self.whitelist.is_empty() {
            return true;
        }
        self.whitelist.contains(&adapter_id.to_string())
    }
}

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

pub struct AdapterManager {
    config: AdapterManagerConfig,
    registry: Arc<AdapterFactoryRegistry>,
    adapters: Arc<RwLock<Vec<Arc<dyn Adapter>>>>,
    logger: Arc<dyn Logger>,
}

impl AdapterManager {
    pub fn new(config: AdapterManagerConfig, logger: Arc<dyn Logger>) -> Self {
        Self {
            config,
            registry: Arc::new(AdapterFactoryRegistry::new()),
            adapters: Arc::new(RwLock::new(Vec::new())),
            logger,
        }
    }

    pub fn with_registry(
        config: AdapterManagerConfig,
        registry: Arc<AdapterFactoryRegistry>,
        logger: Arc<dyn Logger>,
    ) -> Self {
        Self {
            config,
            registry,
            adapters: Arc::new(RwLock::new(Vec::new())),
            logger,
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
                            adapter_paths.push(path);
                        }
                    }
                }
            }
        }

        Ok(adapter_paths)
    }

    pub async fn load_adapter(&self, path: PathBuf) -> Result<AdapterLoadResult> {
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

        let config = self.load_adapter_config(&path)?;

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
                &format!("Loading adapter {} from path: {}", config.adapter_id, path.display()),
                &log_context,
            );

            let mut adapters = self.adapters.write().await;
            let adapter = Arc::new(MockAdapter::new(config.clone()));
            adapters.push(adapter);
            drop(adapters);

            Ok(AdapterLoadResult::success(config.adapter_id.clone()))
        } else {
            Ok(AdapterLoadResult::success(config.adapter_id.clone()))
        }
    }

    pub fn load_adapter_config(&self, path: &PathBuf) -> Result<AdapterConfig> {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext == "json" {
                let content = std::fs::read_to_string(&path).map_err(|e| {
                    AdapterError::ConfigLoadFailed(format!("Failed to read adapter config: {}", e))
                })?;

                return serde_json::from_str::<AdapterConfig>(&content).map_err(|e| {
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

    pub fn create_default_config(&self, path: &PathBuf) -> Result<AdapterConfig> {
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

        Ok(AdapterConfig::new(
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
}

pub struct AdapterHotReloadManager {
    manager: Arc<AdapterManager>,
    interval: Duration,
    running: Arc<RwLock<bool>>,
}

impl AdapterHotReloadManager {
    pub fn new(manager: Arc<AdapterManager>, interval: Duration) -> Self {
        Self {
            manager,
            interval,
            running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(AdapterError::HotReloadError(
                "Hot reload is already running".to_string(),
            ).into());
        }
        *running = true;
        drop(running);

        let manager = Arc::clone(&self.manager);
        let running_flag = Arc::clone(&self.running);
        let interval_duration = self.interval;

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval_duration);
            let mut last_modifications: HashMap<String, std::time::SystemTime> =
                HashMap::new();

            loop {
                let is_running = *running_flag.read().await;
                if !is_running {
                    break;
                }

                interval_timer.tick().await;

                if let Ok(adapter_paths) = manager.discover_adapters().await {
                    for path in adapter_paths {
                        let path_str = path.to_string_lossy().to_string();
                        if let Ok(metadata) = path.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                if let Some(last_modified) = last_modifications.get(&path_str) {
                                    if modified > *last_modified {
                                        let adapter_name = path
                                            .file_stem()
                                            .and_then(|s| s.to_str())
                                            .unwrap_or("unknown")
                                            .to_string();

                                        if manager.is_adapter_loaded(&adapter_name).await {
                                            if let Err(e) = manager.reload_adapter(&adapter_name).await {
                                                eprintln!("Failed to hot reload adapter {}: {}", adapter_name, e);
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
        let mut running = self.running.write().await;
        *running = false;
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

#[derive(Debug)]
pub struct MockAdapter {
    config: AdapterConfig,
}

impl MockAdapter {
    pub fn new(config: AdapterConfig) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
impl Adapter for MockAdapter {
    fn name(&self) -> &str {
        "MockAdapter"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn adapter_id(&self) -> &str {
        &self.config.adapter_id
    }

    fn config(&self) -> AdapterConfig {
        self.config.clone()
    }

    fn status(&self) -> AdapterStatus {
        AdapterStatus::Running
    }

    fn is_running(&self) -> bool {
        true
    }

    fn is_connected(&self) -> bool {
        true
    }

    fn statistics(&self) -> AdapterStatistics {
        AdapterStatistics::default()
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
        let config = AdapterManagerConfig::new();
        let manager = AdapterManager::new(config, logger);

        assert_eq!(manager.adapter_count().await, 0);
        assert_eq!(manager.active_adapter_count().await, 0);
    }

    #[tokio::test]
    async fn test_adapter_config_default() {
        let config = AdapterManagerConfig::new();

        assert_eq!(config.adapter_dir, "./adapters");
        assert!(config.auto_load);
        assert!(config.enable_hot_reload);
        assert_eq!(config.hot_reload_interval, 10);
        assert!(config.whitelist.is_empty());
        assert!(config.blacklist.is_empty());
    }

    #[tokio::test]
    async fn test_adapter_config_builder() {
        let config = AdapterManagerConfig::new()
            .with_adapter_dir("/custom/adapters")
            .with_auto_load(false)
            .with_hot_reload(true)
            .with_hot_reload_interval(30)
                .with_whitelist(vec!["qq".to_string()])
                .with_blacklist(vec!["telegram".to_string()]);

        assert_eq!(config.adapter_dir, "/custom/adapters");
        assert!(!config.auto_load);
        assert!(config.enable_hot_reload);
        assert_eq!(config.hot_reload_interval, 30);
        assert_eq!(config.whitelist, vec!["qq"]);
        assert_eq!(config.blacklist, vec!["telegram"]);
    }

    #[tokio::test]
    async fn test_adapter_config_should_load() {
        let config = AdapterManagerConfig::new()
            .with_whitelist(vec!["qq".to_string()])
            .with_blacklist(vec!["telegram".to_string()]);

        assert!(config.should_load("qq"));
        assert!(config.should_load("wechat"));
        assert!(!config.should_load("telegram"));
    }

    #[tokio::test]
    async fn test_adapter_config_should_load_all() {
        let config = AdapterManagerConfig::new();

        assert!(config.should_load("qq"));
        assert!(config.should_load("wechat"));
        assert!(config.should_load("telegram"));
    }
}
