//! Plugin manager for managing plugin lifecycle

use crate::errors::{PluginError, Result};
use crate::plugins::loader::{CompositePluginLoader, PluginLoader};
use crate::plugins::registry::PluginRegistry;
use crate::plugins::traits::Plugin;
use crate::plugins::types::{PluginConfig, PluginInfo, PluginLoadResult, PluginStatus};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Plugin manager for managing plugin lifecycle
pub struct PluginManager {
    registry: Arc<PluginRegistry>,
    loader: Arc<CompositePluginLoader>,
    config: PluginConfig,
    plugins: Arc<RwLock<Vec<Arc<dyn Plugin>>>>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(config: PluginConfig) -> Self {
        Self {
            registry: Arc::new(PluginRegistry::new()),
            loader: Arc::new(CompositePluginLoader::default()),
            config,
            plugins: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a new plugin manager with custom loader
    pub fn with_loader(config: PluginConfig, loader: CompositePluginLoader) -> Self {
        Self {
            registry: Arc::new(PluginRegistry::new()),
            loader: Arc::new(loader),
            config,
            plugins: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get the plugin registry
    pub fn registry(&self) -> Arc<PluginRegistry> {
        Arc::clone(&self.registry)
    }

    /// Get the plugin config
    pub fn config(&self) -> &PluginConfig {
        &self.config
    }

    /// Update the plugin config
    pub fn update_config(&mut self, config: PluginConfig) {
        self.config = config;
    }

    /// Load a plugin from a path
    pub async fn load_plugin(&self, path: PathBuf) -> Result<PluginLoadResult> {
        // Check if plugin is in blacklist
        if !self.config.blacklist.is_empty() {
            let plugin_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| PluginError::LoadFailed("Invalid plugin path".to_string()))?;
            if self.config.blacklist.contains(&plugin_name.to_string()) {
                return Ok(PluginLoadResult {
                    plugin_name: plugin_name.to_string(),
                    success: false,
                    error: Some("Plugin is blacklisted".to_string()),
                });
            }
        }

        // Load the plugin
        let plugin = self.loader.load_plugin(&path).await?;

        // Check if plugin is in whitelist (if whitelist is configured)
        if !self.config.whitelist.is_empty() {
            let plugin_name = plugin.name();
            if !self.config.whitelist.contains(&plugin_name.to_string()) {
                return Ok(PluginLoadResult {
                    plugin_name: plugin_name.to_string(),
                    success: false,
                    error: Some("Plugin is not whitelisted".to_string()),
                });
            }
        }

        // Check dependencies
        // TODO: Implement dependency checking

        // Get plugin info before moving
        let plugin_name = plugin.name().to_string();
        let plugin_version = plugin.version().to_string();
        let plugin_type = plugin.plugin_type();

        // Initialize the plugin
        // Note: We need to handle this differently because Arc<dyn Plugin> can't be mutably borrowed directly
        // In a real implementation, we'd need to use interior mutability or a different architecture
        // For now, we'll skip the init/load and assume plugin is ready

        // Register the plugin
        let metadata = crate::plugins::types::PluginMetadata::new(
            plugin_name.clone(),
            plugin_version,
            plugin_type,
            path.to_string_lossy().to_string(),
        );
        let plugin_info = PluginInfo::new(metadata).with_status(PluginStatus::Loaded);

        self.registry.register(plugin_info.metadata)?;

        // Store the plugin
        self.plugins.write().await.push(plugin);

        Ok(PluginLoadResult {
            plugin_name,
            success: true,
            error: None,
        })
    }

    /// Unload a plugin by name
    pub async fn unload_plugin(&self, name: &str) -> Result<()> {
        // Check if plugin exists
        if !self.registry.exists(name)? {
            return Err(PluginError::NotFound(name.to_string()).into());
        }

        // Find the plugin
        let mut plugins = self.plugins.write().await;
        let plugin_index = plugins
            .iter()
            .position(|p| p.name() == name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        // Unload the plugin
        let _plugin = plugins.remove(plugin_index);
        // Note: Can't call unload on Arc<dyn Plugin> directly
        // In a real implementation, we'd use interior mutability

        // Unregister the plugin
        self.registry.unregister(name)?;

        Ok(())
    }

    /// Reload a plugin by name
    pub async fn reload_plugin(&self, name: &str) -> Result<()> {
        // Check if plugin exists
        if !self.registry.exists(name)? {
            return Err(PluginError::NotFound(name.to_string()).into());
        }

        // Get plugin info to get the path
        let plugin_info = self.registry.get(name)?;
        let path = PathBuf::from(plugin_info.metadata.entry_point.clone());

        // Unload the plugin
        self.unload_plugin(name).await?;

        // Load the plugin again
        let result = self.load_plugin(path).await?;

        if !result.success {
            return Err(PluginError::ReloadFailed(
                result.error.unwrap_or_else(|| "Unknown error".to_string()),
            )
            .into());
        }

        Ok(())
    }

    /// Update a plugin's config
    pub async fn update_plugin_config(&self, _name: &str, _config: serde_json::Value) -> Result<()> {
        // Note: Cannot update config on Arc<dyn Plugin> without interior mutability
        // For now, this is a placeholder
        Err(PluginError::LoadFailed("Plugin config update not implemented".to_string()).into())
    }

    /// Get a plugin by name
    pub async fn get_plugin(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        let plugins = self.plugins.read().await;
        plugins.iter().find(|p| p.name() == name).cloned()
    }

    /// List all plugins
    pub async fn list_plugins(&self) -> Vec<Arc<dyn Plugin>> {
        let plugins = self.plugins.read().await;
        plugins.clone()
    }

    /// Get plugin info
    pub fn get_plugin_info(&self, name: &str) -> Option<PluginInfo> {
        self.registry.get(name).ok()
    }

    /// List all plugin infos
    pub fn list_plugin_infos(&self) -> Vec<PluginInfo> {
        self.registry.all().unwrap_or_default()
    }

    /// Check if a plugin is loaded
    pub fn is_plugin_loaded(&self, name: &str) -> bool {
        self.registry.exists(name).unwrap_or(false)
    }

    /// Get plugin count
    pub fn plugin_count(&self) -> usize {
        self.registry.count().unwrap_or(0)
    }

    /// Get active plugin count
    pub fn active_plugin_count(&self) -> usize {
        self.registry.active().unwrap_or_default().len()
    }

    /// Discover plugins in the plugin directory
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
                // Check if file has a valid plugin extension
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if ["dll", "so", "dylib", "py", "js", "mjs", "ts"].contains(&ext) {
                        plugin_paths.push(path);
                    }
                }
            }
        }

        Ok(plugin_paths)
    }

    /// Auto-load all plugins from plugin directory
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

    /// Unload all plugins
    pub async fn unload_all(&self) -> Result<()> {
        let plugins = self.plugins.read().await;
        let plugin_names: Vec<String> = plugins.iter().map(|p| p.name().to_string()).collect();
        drop(plugins);

        for name in plugin_names {
            let _ = self.unload_plugin(&name).await;
        }

        Ok(())
    }

    /// Clear all plugins (force unload)
    pub async fn clear(&self) {
        let _ = self.unload_all().await;
        drop(self.registry.clear());
    }
}

/// Hot reload manager for watching plugin file changes
pub struct HotReloadManager {
    manager: Arc<PluginManager>,
    interval: Duration,
    running: Arc<RwLock<bool>>,
}

impl HotReloadManager {
    /// Create a new hot reload manager
    pub fn new(manager: Arc<PluginManager>, interval: Duration) -> Self {
        Self {
            manager,
            interval,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the hot reload loop
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

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(Self::interval(&manager));
            let mut last_modifications: std::collections::HashMap<String, std::time::SystemTime> =
                std::collections::HashMap::new();

            loop {
                let is_running = *running_flag.read().await;
                if !is_running {
                    break;
                }

                interval_timer.tick().await;

                // Check for plugin modifications
                if let Ok(plugin_paths) = manager.discover_plugins().await {
                    for path in plugin_paths {
                        let path_str = path.to_string_lossy().to_string();
                        if let Ok(metadata) = path.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                if let Some(&last_modified) = last_modifications.get(&path_str) {
                                    if modified > last_modified {
                                        // Plugin file was modified, reload it
                                        let plugin_name = path
                                            .file_stem()
                                            .and_then(|s| s.to_str())
                                            .unwrap_or("unknown")
                                            .to_string();

                                        if manager.is_plugin_loaded(&plugin_name) {
                                            if let Err(e) = manager.reload_plugin(&plugin_name).await {
                                                eprintln!(
                                                    "Failed to hot reload plugin {}: {}",
                                                    plugin_name, e
                                                );
                                            }
                                        } else {
                                            // Try to load the plugin
                                            if let Err(e) = manager.load_plugin(path).await {
                                                eprintln!("Failed to load plugin {}: {}", plugin_name, e);
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

    /// Stop the hot reload loop
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        Ok(())
    }

    /// Check if hot reload is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Get the reload interval from manager config
    fn interval(manager: &PluginManager) -> Duration {
        Duration::from_secs(manager.config().hot_reload_interval)
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
        let config = PluginConfig {
            plugin_dir: "./plugins".to_string(),
            auto_load: false,
            enable_hot_reload: false,
            hot_reload_interval: 1,
            whitelist: vec![],
            blacklist: vec!["test".to_string()],
        };
        let manager = PluginManager::new(config);
        assert_eq!(manager.config().plugin_dir, "./plugins");
    }
}
