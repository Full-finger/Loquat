//! Plugin manager for managing plugin lifecycle

use crate::config::loquat_config::PluginConfig;
use crate::errors::{PluginError, Result};
use crate::plugins::loader::{CompositePluginLoader, PluginLoader};
use crate::plugins::registry::PluginRegistry;
use crate::plugins::traits::Plugin;
use crate::plugins::types::{PluginInfo, PluginLoadResult, PluginStatus};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

pub struct PluginManager {
    registry: Arc<PluginRegistry>,
    loader: Arc<CompositePluginLoader>,
    config: PluginConfig,
    plugins: Arc<RwLock<Vec<Arc<dyn Plugin>>>>,
}

impl PluginManager {
    pub fn new(config: PluginConfig) -> Self {
        Self {
            registry: Arc::new(PluginRegistry::new()),
            loader: Arc::new(CompositePluginLoader::default()),
            config,
            plugins: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn with_loader(config: PluginConfig, loader: CompositePluginLoader) -> Self {
        Self {
            registry: Arc::new(PluginRegistry::new()),
            loader: Arc::new(loader),
            config,
            plugins: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn registry(&self) -> Arc<PluginRegistry> {
        Arc::clone(&self.registry)
    }

    pub fn config(&self) -> &PluginConfig {
        &self.config
    }

    pub fn update_config(&mut self, config: PluginConfig) {
        self.config = config;
    }

    pub async fn load_plugin(&self, path: PathBuf) -> Result<PluginLoadResult> {
        // Use should_load to check if plugin should be loaded
        let plugin_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| PluginError::LoadFailed("Invalid plugin path".to_string()))?;
        
        if !self.config.should_load(&plugin_name) {
            return Ok(PluginLoadResult {
                plugin_name: plugin_name.to_string(),
                success: false,
                error: Some("Plugin is not enabled or is filtered".to_string()),
            });
        }

        let plugin = self.loader.load_plugin(&path).await?;
        let plugin_name = plugin.name().to_string();
        let plugin_version = plugin.version().to_string();
        let plugin_type = plugin.plugin_type();

        let metadata = crate::plugins::types::PluginMetadata::new(
            plugin_name.clone(),
            plugin_version,
            plugin_type,
            path.to_string_lossy().to_string(),
        );
        let plugin_info = PluginInfo::new(metadata.clone()).with_status(PluginStatus::Loaded);

        self.registry.register(plugin_info.metadata.clone())?;
        self.plugins.write().await.push(plugin);

        Ok(PluginLoadResult {
            plugin_name,
            success: true,
            error: None,
        })
    }

    pub async fn unload_plugin(&self, name: &str) -> Result<()> {
        if !self.registry.exists(name)? {
            return Err(PluginError::NotFound(name.to_string()).into());
        }

        let mut plugins = self.plugins.write().await;
        let plugin_index = plugins
            .iter()
            .position(|p| p.name() == name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        let _plugin = plugins.remove(plugin_index);
        drop(plugins);

        self.registry.unregister(name)?;
        Ok(())
    }

    pub async fn reload_plugin(&self, name: &str) -> Result<()> {
        if !self.registry.exists(name)? {
            return Err(PluginError::NotFound(name.to_string()).into());
        }

        let plugin_info = self.registry.get(name)?;
        let path = PathBuf::from(plugin_info.metadata.entry_point.clone());

        self.unload_plugin(name).await?;

        let result = self.load_plugin(path).await?;

        if !result.success {
            return Err(PluginError::ReloadFailed(
                result.error.unwrap_or_else(|| "Unknown error".to_string()),
            )
            .into());
        }

        Ok(())
    }

    pub async fn get_plugin(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        let plugins = self.plugins.read().await;
        plugins.iter().find(|p| p.name() == name).cloned()
    }

    pub async fn list_plugins(&self) -> Vec<Arc<dyn Plugin>> {
        let plugins = self.plugins.read().await;
        plugins.clone()
    }

    pub fn get_plugin_info(&self, name: &str) -> Option<PluginInfo> {
        self.registry.get(name).ok()
    }

    pub fn list_plugin_infos(&self) -> Vec<PluginInfo> {
        self.registry.all().unwrap_or_default()
    }

    pub fn is_plugin_loaded(&self, name: &str) -> bool {
        self.registry.exists(name).unwrap_or(false)
    }

    pub fn plugin_count(&self) -> usize {
        self.registry.count().unwrap_or(0)
    }

    pub fn active_plugin_count(&self) -> usize {
        self.registry.active().unwrap_or_default().len()
    }

    pub async fn discover_plugins(&self) -> Result<Vec<PathBuf>> {
        let plugin_dir = PathBuf::from(&self.config.plugin_dir);

        if !plugin_dir.exists() {
            return Ok(Vec::new());
        }

        let mut plugin_paths = Vec::new();
        let mut entries = tokio::fs::read_dir(&plugin_dir).await.map_err(|e| {
            PluginError::LoadFailed(format!("Failed to read plugin directory: {}", e))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            PluginError::LoadFailed(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if ["dll", "so", "dylib", "py", "js", "mjs", "ts"].contains(&ext) {
                        plugin_paths.push(path);
                    }
                }
            }
        }

        Ok(plugin_paths)
    }

    pub async fn auto_load_plugins(&self) -> Result<Vec<PluginLoadResult>> {
        let plugin_paths = self.discover_plugins().await?;
        let mut results = Vec::new();

        for path in plugin_paths {
            match self.load_plugin(path.clone()).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    let plugin_name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    results.push(PluginLoadResult {
                        plugin_name,
                        success: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(results)
    }

    pub async fn unload_all(&self) -> Result<()> {
        let plugins = self.plugins.read().await;
        let plugin_names: Vec<String> = plugins.iter().map(|p| p.name().to_string()).collect();
        drop(plugins);

        for name in plugin_names {
            let _ = self.unload_plugin(&name).await;
        }

        Ok(())
    }

    pub async fn clear(&self) {
        let _ = self.unload_all().await;
    }
}

pub struct HotReloadManager {
    manager: Arc<PluginManager>,
    interval: Duration,
    running: Arc<RwLock<bool>>,
}

impl HotReloadManager {
    pub fn new(manager: Arc<PluginManager>, interval: Duration) -> Self {
        Self {
            manager,
            interval,
            running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(PluginError::HotReloadError(
                "Hot reload is already running".to_string(),
            )
            .into());
        }
        *running = true;
        drop(running);

        let manager = Arc::clone(&self.manager);
        let running_flag = Arc::clone(&self.running);
        let interval_duration = self.interval;

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval_duration);
            let mut last_modifications: std::collections::HashMap<String, std::time::SystemTime> =
                std::collections::HashMap::new();

            loop {
                let is_running = *running_flag.read().await;
                if !is_running {
                    break;
                }

                interval_timer.tick().await;

                if let Ok(plugin_paths) = manager.discover_plugins().await {
                    for path in plugin_paths {
                        let path_str = path.to_string_lossy().to_string();
                        if let Ok(metadata) = path.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                if let Some(last_modified) = last_modifications.get(&path_str).copied() {
                                    if modified > last_modified {
                                        let plugin_name = path
                                            .file_stem()
                                            .and_then(|s| s.to_str())
                                            .unwrap_or("unknown")
                                            .to_string();

                                        if manager.is_plugin_loaded(&plugin_name) {
                                            let _ = manager.reload_plugin(&plugin_name).await;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_manager_creation() {
        let config = PluginConfig::default();
        let manager = PluginManager::new(config);
        assert_eq!(manager.plugin_count(), 0);
    }

    #[tokio::test]
    async fn test_plugin_manager_with_config() {
        let mut config = PluginConfig::default();
        config.plugin_dir = "./plugins".to_string();
        let manager = PluginManager::new(config);
        assert_eq!(manager.config().plugin_dir, "./plugins");
    }
}
