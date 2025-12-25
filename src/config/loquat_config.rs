//! Loquat Framework Configuration
//! 
//! Manages configuration loading from TOML files with support for multiple environments

use crate::errors::{ConfigError, LoquatError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Main configuration structure for the Loquat framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoquatConfig {
    /// General configuration
    pub general: GeneralConfig,
    
    /// Logging configuration
    pub logging: LoggingConfig,
    
    /// Plugin configuration
    pub plugins: PluginConfig,
    
    /// Adapter configuration
    pub adapters: AdapterConfig,
    
    /// Engine configuration
    pub engine: EngineConfig,
    
    /// Web configuration
    pub web: WebConfig,
}

impl Default for LoquatConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            logging: LoggingConfig::default(),
            plugins: PluginConfig::default(),
            adapters: AdapterConfig::default(),
            engine: EngineConfig::default(),
            web: WebConfig::default(),
        }
    }
}

/// General configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Environment name (dev, test, prod)
    pub environment: String,
    /// Framework name
    pub name: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            environment: "dev".to_string(),
            name: "Loquat Framework".to_string(),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (Trace, Debug, Info, Warn, Error)
    pub level: String,
    /// Log format (text, json)
    pub format: String,
    /// Log output (console, file, combined)
    pub output: String,
    /// Log file path
    pub file_path: String,
    /// Enable colored output
    pub enable_colors: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "Info".to_string(),
            format: "text".to_string(),
            output: "console".to_string(),
            file_path: "./logs/loquat.log".to_string(),
            enable_colors: true,
        }
    }
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Enable plugin system
    pub enabled: bool,
    /// Plugin directory
    pub plugin_dir: String,
    /// Auto-load plugins on startup
    pub auto_load: bool,
    /// Enable hot reload
    pub enable_hot_reload: bool,
    /// Hot reload interval in seconds
    pub hot_reload_interval: u64,
    /// Whitelist of plugin names to load
    pub whitelist: Vec<String>,
    /// Blacklist of plugin names to skip
    pub blacklist: Vec<String>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            plugin_dir: "./plugins".to_string(),
            auto_load: false,
            enable_hot_reload: false,
            hot_reload_interval: 5,
            whitelist: Vec::new(),
            blacklist: Vec::new(),
        }
    }
}

/// Adapter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    /// Enable adapter system
    pub enabled: bool,
    /// Adapter directory
    pub adapter_dir: String,
    /// Auto-load adapters on startup
    pub auto_load: bool,
    /// Enable hot reload
    pub enable_hot_reload: bool,
    /// Hot reload interval in seconds
    pub hot_reload_interval: u64,
    /// Whitelist of adapter names to load
    pub whitelist: Vec<String>,
    /// Blacklist of adapter names to skip
    pub blacklist: Vec<String>,
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            adapter_dir: "./adapters".to_string(),
            auto_load: false,
            enable_hot_reload: false,
            hot_reload_interval: 10,
            whitelist: Vec::new(),
            blacklist: Vec::new(),
        }
    }
}

/// Engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    /// Enable auto-routing
    pub auto_route: bool,
    /// Enable auto-creation of channels
    pub auto_create_channels: bool,
    /// Enable auto-initialization
    pub auto_initialize: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            auto_route: true,
            auto_create_channels: true,
            auto_initialize: true,
        }
    }
}

/// Web server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    /// Enable web server
    pub enabled: bool,
    /// Server host
    pub host: String,
    /// Server port
    pub port: u16,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}

impl LoquatConfig {
    /// Load configuration from a TOML file
    pub fn from_toml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| ConfigError::LoadError(format!("Failed to read config file '{}': {}", path.as_ref().display(), e)))?;
        
        toml::from_str(&content)
            .map_err(|e| LoquatError::from(ConfigError::InvalidFormat(format!("Invalid TOML format in '{}': {}", path.as_ref().display(), e))))
    }
    
    /// Load configuration with environment override
    /// 
    /// Loads default config first, then overrides with environment-specific config
    pub fn from_environment<P: AsRef<Path>>(config_dir: P, environment: &str) -> Result<Self> {
        let config_dir = config_dir.as_ref();
        
        // Load default config
        let default_path = config_dir.join("default.toml");
        let mut config = if default_path.exists() {
            Self::from_toml_file(&default_path)?
        } else {
            return Err(LoquatError::from(ConfigError::LoadError(
                format!("Default configuration file not found: {}", default_path.display())
            )));
        };
        
        // Load environment-specific config and merge
        let env_path = config_dir.join(format!("{}.toml", environment));
        if env_path.exists() {
            let env_config = Self::from_toml_file(&env_path)?;
            config.merge(&env_config)?;
        }
        
        Ok(config)
    }
    
    /// Merge with another configuration (environment overrides default)
    pub fn merge(&mut self, other: &LoquatConfig) -> Result<()> {
        // Merge general config
        if other.general.environment != GeneralConfig::default().environment {
            self.general.environment = other.general.environment.clone();
        }
        if other.general.name != GeneralConfig::default().name {
            self.general.name = other.general.name.clone();
        }
        
        // Merge logging config
        if other.logging.level != LoggingConfig::default().level {
            self.logging.level = other.logging.level.clone();
        }
        if other.logging.format != LoggingConfig::default().format {
            self.logging.format = other.logging.format.clone();
        }
        if other.logging.output != LoggingConfig::default().output {
            self.logging.output = other.logging.output.clone();
        }
        if other.logging.file_path != LoggingConfig::default().file_path {
            self.logging.file_path = other.logging.file_path.clone();
        }
        if other.logging.enable_colors != LoggingConfig::default().enable_colors {
            self.logging.enable_colors = other.logging.enable_colors;
        }
        
        // Merge plugin config
        if other.plugins.enabled != PluginConfig::default().enabled {
            self.plugins.enabled = other.plugins.enabled;
        }
        if other.plugins.plugin_dir != PluginConfig::default().plugin_dir {
            self.plugins.plugin_dir = other.plugins.plugin_dir.clone();
        }
        if other.plugins.auto_load != PluginConfig::default().auto_load {
            self.plugins.auto_load = other.plugins.auto_load;
        }
        if other.plugins.enable_hot_reload != PluginConfig::default().enable_hot_reload {
            self.plugins.enable_hot_reload = other.plugins.enable_hot_reload;
        }
        if other.plugins.hot_reload_interval != PluginConfig::default().hot_reload_interval {
            self.plugins.hot_reload_interval = other.plugins.hot_reload_interval;
        }
        if !other.plugins.whitelist.is_empty() {
            self.plugins.whitelist = other.plugins.whitelist.clone();
        }
        if !other.plugins.blacklist.is_empty() {
            self.plugins.blacklist = other.plugins.blacklist.clone();
        }
        
        // Merge adapter config
        if other.adapters.enabled != AdapterConfig::default().enabled {
            self.adapters.enabled = other.adapters.enabled;
        }
        if other.adapters.adapter_dir != AdapterConfig::default().adapter_dir {
            self.adapters.adapter_dir = other.adapters.adapter_dir.clone();
        }
        if other.adapters.auto_load != AdapterConfig::default().auto_load {
            self.adapters.auto_load = other.adapters.auto_load;
        }
        if other.adapters.enable_hot_reload != AdapterConfig::default().enable_hot_reload {
            self.adapters.enable_hot_reload = other.adapters.enable_hot_reload;
        }
        if other.adapters.hot_reload_interval != AdapterConfig::default().hot_reload_interval {
            self.adapters.hot_reload_interval = other.adapters.hot_reload_interval;
        }
        if !other.adapters.whitelist.is_empty() {
            self.adapters.whitelist = other.adapters.whitelist.clone();
        }
        if !other.adapters.blacklist.is_empty() {
            self.adapters.blacklist = other.adapters.blacklist.clone();
        }
        
        // Merge engine config
        if other.engine.auto_route != EngineConfig::default().auto_route {
            self.engine.auto_route = other.engine.auto_route;
        }
        if other.engine.auto_create_channels != EngineConfig::default().auto_create_channels {
            self.engine.auto_create_channels = other.engine.auto_create_channels;
        }
        if other.engine.auto_initialize != EngineConfig::default().auto_initialize {
            self.engine.auto_initialize = other.engine.auto_initialize;
        }
        
        // Merge web config
        if other.web.enabled != WebConfig::default().enabled {
            self.web.enabled = other.web.enabled;
        }
        if other.web.host != WebConfig::default().host {
            self.web.host = other.web.host.clone();
        }
        if other.web.port != WebConfig::default().port {
            self.web.port = other.web.port;
        }
        
        Ok(())
    }
    
    /// Save configuration to a TOML file
    pub fn to_toml_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| LoquatError::from(ConfigError::InvalidFormat(format!("Failed to serialize TOML config: {}", e))))?;
        
        std::fs::write(path, content)
            .map_err(|e| LoquatError::from(ConfigError::LoadError(format!("Failed to write TOML config file: {}", e))))
    }
    
    /// Get config directory path
    pub fn get_config_dir() -> Result<PathBuf> {
        let current_dir = std::env::current_dir()
            .map_err(|e| ConfigError::LoadError(format!("Failed to get current directory: {}", e)))?;
        Ok(current_dir.join("config"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LoquatConfig::default();
        assert_eq!(config.general.environment, "dev");
        assert_eq!(config.logging.level, "Info");
        assert_eq!(config.plugins.plugin_dir, "./plugins");
        assert_eq!(config.adapters.adapter_dir, "./adapters");
    }

    #[test]
    fn test_config_merge() {
        let mut base = LoquatConfig::default();
        let mut env = LoquatConfig::default();
        
        env.general.environment = "prod".to_string();
        env.logging.level = "Warn".to_string();
        env.plugins.auto_load = true;
        env.adapters.auto_load = true;
        
        base.merge(&env).unwrap();
        
        assert_eq!(base.general.environment, "prod");
        assert_eq!(base.logging.level, "Warn");
        assert!(base.plugins.auto_load);
        assert!(base.adapters.auto_load);
    }
}
