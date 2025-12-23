//! Plugin registry - maintains plugin information

use crate::errors::{PluginError, Result};
use crate::plugins::types::{PluginInfo, PluginMetadata, PluginStatus};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Plugin registry - stores plugin information
#[derive(Debug, Clone)]
pub struct PluginRegistry {
    /// Plugin info by name
    plugins: Arc<RwLock<HashMap<String, PluginInfo>>>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a plugin
    pub fn register(&self, metadata: PluginMetadata) -> Result<()> {
        let mut plugins = self.plugins.write().map_err(|e| {
            PluginError::RegistryError(format!("Failed to acquire write lock: {}", e))
        })?;

        if plugins.contains_key(&metadata.name) {
            return Err(PluginError::AlreadyExists(metadata.name.clone()).into());
        }

        plugins.insert(metadata.name.clone(), PluginInfo::new(metadata));
        Ok(())
    }

    /// Unregister a plugin
    pub fn unregister(&self, name: &str) -> Result<()> {
        let mut plugins = self.plugins.write().map_err(|e| {
            PluginError::RegistryError(format!("Failed to acquire write lock: {}", e))
        })?;

        if !plugins.contains_key(name) {
            return Err(PluginError::NotFound(name.to_string()).into());
        }

        plugins.remove(name);
        Ok(())
    }

    /// Get plugin info
    pub fn get(&self, name: &str) -> Result<PluginInfo> {
        let plugins = self.plugins.read().map_err(|e| {
            PluginError::RegistryError(format!("Failed to acquire read lock: {}", e))
        })?;

        plugins
            .get(name)
            .cloned()
            .ok_or_else(|| PluginError::NotFound(name.to_string()).into())
    }

    /// Get all plugin names
    pub fn list(&self) -> Result<Vec<String>> {
        let plugins = self.plugins.read().map_err(|e| {
            PluginError::RegistryError(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(plugins.keys().cloned().collect())
    }

    /// Get all plugin info
    pub fn all(&self) -> Result<Vec<PluginInfo>> {
        let plugins = self.plugins.read().map_err(|e| {
            PluginError::RegistryError(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(plugins.values().cloned().collect())
    }

    /// Update plugin status
    pub fn update_status(&self, name: &str, status: PluginStatus) -> Result<()> {
        let mut plugins = self.plugins.write().map_err(|e| {
            PluginError::RegistryError(format!("Failed to acquire write lock: {}", e))
        })?;

        if let Some(info) = plugins.get_mut(name) {
            *info = info.clone().with_status(status);
            Ok(())
        } else {
            Err(PluginError::NotFound(name.to_string()).into())
        }
    }

    /// Check if plugin exists
    pub fn exists(&self, name: &str) -> Result<bool> {
        let plugins = self.plugins.read().map_err(|e| {
            PluginError::RegistryError(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(plugins.contains_key(name))
    }

    /// Get plugin count
    pub fn count(&self) -> Result<usize> {
        let plugins = self.plugins.read().map_err(|e| {
            PluginError::RegistryError(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(plugins.len())
    }

    /// Get plugins by status
    pub fn by_status(&self, status: PluginStatus) -> Result<Vec<PluginInfo>> {
        let plugins = self.plugins.read().map_err(|e| {
            PluginError::RegistryError(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(plugins
            .values()
            .filter(|p| p.status == status)
            .cloned()
            .collect())
    }

    /// Get active (loaded) plugins
    pub fn active(&self) -> Result<Vec<PluginInfo>> {
        self.by_status(PluginStatus::Loaded)
    }

    /// Clear all plugins
    pub fn clear(&self) -> Result<()> {
        let mut plugins = self.plugins.write().map_err(|e| {
            PluginError::RegistryError(format!("Failed to acquire write lock: {}", e))
        })?;

        plugins.clear();
        Ok(())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::types::PluginType;

    fn create_test_metadata(name: &str) -> PluginMetadata {
        PluginMetadata::new(
            name.to_string(),
            "1.0.0".to_string(),
            PluginType::Native,
            format!("./{}.so", name),
        )
    }

    #[test]
    fn test_registry_creation() {
        let registry = PluginRegistry::new();
        assert_eq!(registry.count().unwrap(), 0);
    }

    #[test]
    fn test_register_plugin() {
        let registry = PluginRegistry::new();
        let metadata = create_test_metadata("test_plugin");

        assert!(registry.register(metadata).is_ok());
        assert_eq!(registry.count().unwrap(), 1);
    }

    #[test]
    fn test_register_duplicate() {
        let registry = PluginRegistry::new();
        let metadata1 = create_test_metadata("test_plugin");
        let metadata2 = create_test_metadata("test_plugin");

        assert!(registry.register(metadata1).is_ok());
        assert!(registry.register(metadata2).is_err());
    }

    #[test]
    fn test_unregister_plugin() {
        let registry = PluginRegistry::new();
        let metadata = create_test_metadata("test_plugin");

        assert!(registry.register(metadata).is_ok());
        assert!(registry.unregister("test_plugin").is_ok());
        assert_eq!(registry.count().unwrap(), 0);
    }

    #[test]
    fn test_get_plugin() {
        let registry = PluginRegistry::new();
        let metadata = create_test_metadata("test_plugin");

        assert!(registry.register(metadata).is_ok());
        let info = registry.get("test_plugin").unwrap();
        assert_eq!(info.metadata.name, "test_plugin");
    }

    #[test]
    fn test_get_nonexistent() {
        let registry = PluginRegistry::new();
        assert!(registry.get("nonexistent").is_err());
    }

    #[test]
    fn test_list_plugins() {
        let registry = PluginRegistry::new();

        registry.register(create_test_metadata("plugin1")).unwrap();
        registry.register(create_test_metadata("plugin2")).unwrap();
        registry.register(create_test_metadata("plugin3")).unwrap();

        let names = registry.list().unwrap();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"plugin1".to_string()));
    }

    #[test]
    fn test_update_status() {
        let registry = PluginRegistry::new();
        let metadata = create_test_metadata("test_plugin");

        registry.register(metadata).unwrap();
        registry
            .update_status("test_plugin", PluginStatus::Loaded)
            .unwrap();

        let info = registry.get("test_plugin").unwrap();
        assert!(info.status.is_active());
    }

    #[test]
    fn test_plugin_exists() {
        let registry = PluginRegistry::new();

        assert!(!registry.exists("test_plugin").unwrap());

        registry.register(create_test_metadata("test_plugin")).unwrap();
        assert!(registry.exists("test_plugin").unwrap());
    }

    #[test]
    fn test_by_status() {
        let registry = PluginRegistry::new();

        registry
            .register(create_test_metadata("plugin1"))
            .unwrap();
        registry
            .register(create_test_metadata("plugin2"))
            .unwrap();
        registry
            .register(create_test_metadata("plugin3"))
            .unwrap();

        registry
            .update_status("plugin1", PluginStatus::Loaded)
            .unwrap();
        registry
            .update_status("plugin2", PluginStatus::Loaded)
            .unwrap();

        let loaded = registry.by_status(PluginStatus::Loaded).unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn test_active_plugins() {
        let registry = PluginRegistry::new();

        registry
            .register(create_test_metadata("plugin1"))
            .unwrap();
        registry
            .register(create_test_metadata("plugin2"))
            .unwrap();
        registry
            .register(create_test_metadata("plugin3"))
            .unwrap();

        registry
            .update_status("plugin1", PluginStatus::Loaded)
            .unwrap();
        registry
            .update_status("plugin2", PluginStatus::Unloaded)
            .unwrap();

        let active = registry.active().unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].metadata.name, "plugin1");
    }

    #[test]
    fn test_clear() {
        let registry = PluginRegistry::new();

        registry
            .register(create_test_metadata("plugin1"))
            .unwrap();
        registry
            .register(create_test_metadata("plugin2"))
            .unwrap();

        assert_eq!(registry.count().unwrap(), 2);
        registry.clear().unwrap();
        assert_eq!(registry.count().unwrap(), 0);
    }
}
