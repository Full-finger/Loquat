//! Plugin type definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plugin type - supported plugin languages
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    /// Native Rust plugin (dylib)
    Native,
    /// Python plugin
    Python,
    /// JavaScript plugin
    JavaScript,
}

impl PluginType {
    /// Get file extension for this plugin type
    pub fn extension(&self) -> &str {
        match self {
            PluginType::Native => "so",
            PluginType::Python => "py",
            PluginType::JavaScript => "js",
        }
    }
}

impl std::fmt::Display for PluginType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginType::Native => write!(f, "native"),
            PluginType::Python => write!(f, "python"),
            PluginType::JavaScript => write!(f, "javascript"),
        }
    }
}

/// Plugin status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginStatus {
    /// Plugin not loaded
    Unloaded,
    /// Plugin is loading
    Loading,
    /// Plugin loaded successfully
    Loaded,
    /// Plugin encountered error
    Error { message: String },
    /// Plugin is disabled
    Disabled,
}

impl PluginStatus {
    /// Check if plugin is active
    pub fn is_active(&self) -> bool {
        matches!(self, PluginStatus::Loaded)
    }

    /// Get error message if any
    pub fn error_message(&self) -> Option<&str> {
        match self {
            PluginStatus::Error { message } => Some(message.as_str()),
            _ => None,
        }
    }
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin name
    pub name: String,
    
    /// Plugin version
    pub version: String,
    
    /// Plugin description
    pub description: Option<String>,
    
    /// Plugin author
    pub author: Option<String>,
    
    /// Plugin type
    pub plugin_type: PluginType,
    
    /// Entry point file path
    pub entry_point: String,
    
    /// Dependencies (other plugins)
    pub dependencies: Vec<String>,
    
    /// Additional metadata
    pub extra: HashMap<String, serde_json::Value>,
}

impl PluginMetadata {
    /// Create a new plugin metadata
    pub fn new(name: String, version: String, plugin_type: PluginType, entry_point: String) -> Self {
        Self {
            name,
            version,
            description: None,
            author: None,
            plugin_type,
            entry_point,
            dependencies: Vec::new(),
            extra: HashMap::new(),
        }
    }
    
    /// Set description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }
    
    /// Set author
    pub fn with_author(mut self, author: &str) -> Self {
        self.author = Some(author.to_string());
        self
    }
    
    /// Set dependencies
    pub fn with_dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }
    
    /// Add extra metadata
    pub fn with_extra<K: Into<String>, V: Serialize>(mut self, key: K, value: V) -> Self {
        if let Ok(val) = serde_json::to_value(value) {
            self.extra.insert(key.into(), val);
        }
        self
    }
}

/// Plugin information (metadata + runtime status)
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Plugin metadata
    pub metadata: PluginMetadata,
    
    /// Current status
    pub status: PluginStatus,
    
    /// Load timestamp
    pub loaded_at: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Error count
    pub error_count: usize,
}

impl PluginInfo {
    /// Create a new plugin info
    pub fn new(metadata: PluginMetadata) -> Self {
        Self {
            metadata,
            status: PluginStatus::Unloaded,
            loaded_at: None,
            error_count: 0,
        }
    }
    
    /// Update status
    pub fn with_status(mut self, status: PluginStatus) -> Self {
        self.status = status.clone();
        if status.is_active() {
            self.loaded_at = Some(chrono::Utc::now());
        }
        self
    }
    
    /// Increment error count
    pub fn with_error(mut self, message: &str) -> Self {
        self.error_count += 1;
        self.status = PluginStatus::Error {
            message: message.to_string(),
        };
        self
    }
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Plugin directory
    pub plugin_dir: String,
    
    /// Auto-load plugins on startup
    pub auto_load: bool,
    
    /// Enable hot-reload
    pub enable_hot_reload: bool,
    
    /// Hot-reload interval in seconds
    pub hot_reload_interval: u64,
    
    /// Whitelist of plugins to load (empty = all)
    pub whitelist: Vec<String>,
    
    /// Blacklist of plugins to skip
    pub blacklist: Vec<String>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            plugin_dir: "./plugins".to_string(),
            auto_load: true,
            enable_hot_reload: true,
            hot_reload_interval: 5,
            whitelist: Vec::new(),
            blacklist: Vec::new(),
        }
    }
}

impl PluginConfig {
    /// Create a new plugin config
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set plugin directory
    pub fn with_plugin_dir(mut self, dir: &str) -> Self {
        self.plugin_dir = dir.to_string();
        self
    }
    
    /// Enable or disable auto-load
    pub fn with_auto_load(mut self, enabled: bool) -> Self {
        self.auto_load = enabled;
        self
    }
    
    /// Enable or disable hot-reload
    pub fn with_hot_reload(mut self, enabled: bool) -> Self {
        self.enable_hot_reload = enabled;
        self
    }
    
    /// Set hot-reload interval
    pub fn with_hot_reload_interval(mut self, seconds: u64) -> Self {
        self.hot_reload_interval = seconds;
        self
    }
    
    /// Add to whitelist
    pub fn with_whitelist(mut self, plugins: Vec<String>) -> Self {
        self.whitelist = plugins;
        self
    }
    
    /// Add to blacklist
    pub fn with_blacklist(mut self, plugins: Vec<String>) -> Self {
        self.blacklist = plugins;
        self
    }
    
    /// Check if plugin should be loaded
    pub fn should_load(&self, plugin_name: &str) -> bool {
        // Check blacklist first
        if self.blacklist.contains(&plugin_name.to_string()) {
            return false;
        }
        
        // If whitelist is empty, load all
        if self.whitelist.is_empty() {
            return true;
        }
        
        // Check whitelist
        self.whitelist.contains(&plugin_name.to_string())
    }
}

/// Plugin load result
#[derive(Debug, Clone)]
pub struct PluginLoadResult {
    /// Plugin name
    pub plugin_name: String,
    
    /// Whether load was successful
    pub success: bool,
    
    /// Error message if any
    pub error: Option<String>,
}

impl PluginLoadResult {
    /// Create a successful result
    pub fn success(plugin_name: String) -> Self {
        Self {
            plugin_name,
            success: true,
            error: None,
        }
    }
    
    /// Create a failure result
    pub fn failure(plugin_name: String, error: String) -> Self {
        Self {
            plugin_name,
            success: false,
            error: Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_type_extensions() {
        assert_eq!(PluginType::Native.extension(), "so");
        assert_eq!(PluginType::Python.extension(), "py");
        assert_eq!(PluginType::JavaScript.extension(), "js");
    }

    #[test]
    fn test_plugin_type_display() {
        assert_eq!(PluginType::Native.to_string(), "native");
        assert_eq!(PluginType::Python.to_string(), "python");
        assert_eq!(PluginType::JavaScript.to_string(), "javascript");
    }

    #[test]
    fn test_plugin_status_is_active() {
        assert!(PluginStatus::Loaded.is_active());
        assert!(!PluginStatus::Unloaded.is_active());
        assert!(!PluginStatus::Error { message: "error".to_string() }.is_active());
    }

    #[test]
    fn test_plugin_status_error_message() {
        assert_eq!(
            PluginStatus::Error { message: "test error".to_string() }.error_message(),
            Some("test error")
        );
        assert!(PluginStatus::Loaded.error_message().is_none());
    }

    #[test]
    fn test_plugin_metadata_builder() {
        let metadata = PluginMetadata::new(
            "test_plugin".to_string(),
            "1.0.0".to_string(),
            PluginType::Native,
            "./test_plugin.so".to_string(),
        )
        .with_description("Test plugin")
        .with_author("Test Author")
        .with_dependencies(vec!["dep1".to_string()]);

        assert_eq!(metadata.name, "test_plugin");
        assert_eq!(metadata.description, Some("Test plugin".to_string()));
        assert_eq!(metadata.author, Some("Test Author".to_string()));
        assert!(metadata.dependencies.contains(&"dep1".to_string()));
    }

    #[test]
    fn test_plugin_config_default() {
        let config = PluginConfig::default();
        assert_eq!(config.plugin_dir, "./plugins");
        assert!(config.auto_load);
        assert!(config.enable_hot_reload);
        assert_eq!(config.hot_reload_interval, 5);
    }

    #[test]
    fn test_plugin_config_should_load() {
        let config = PluginConfig::new()
            .with_whitelist(vec!["plugin1".to_string()])
            .with_blacklist(vec!["plugin2".to_string()]);

        assert!(config.should_load("plugin1")); // In whitelist
        assert!(!config.should_load("plugin2")); // In blacklist
        assert!(!config.should_load("plugin3")); // Not in whitelist
    }

    #[test]
    fn test_plugin_config_all_whitelist() {
        let config = PluginConfig::new();
        assert!(config.should_load("any_plugin")); // Empty whitelist = all
    }

    #[test]
    fn test_plugin_info() {
        let metadata = PluginMetadata::new(
            "test".to_string(),
            "1.0.0".to_string(),
            PluginType::Native,
            "./test.so".to_string(),
        );
        let info = PluginInfo::new(metadata)
            .with_status(PluginStatus::Loaded);

        assert!(info.status.is_active());
        assert!(info.loaded_at.is_some());
    }

    #[test]
    fn test_plugin_load_result() {
        let success = PluginLoadResult::success("test_plugin".to_string());
        assert!(success.success);
        assert!(success.error.is_none());

        let failure = PluginLoadResult::failure("test_plugin".to_string(), "error".to_string());
        assert!(!failure.success);
        assert_eq!(failure.error, Some("error".to_string()));
    }
}
