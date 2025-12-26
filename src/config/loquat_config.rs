//! Loquat Framework Configuration
//! 
//! Manages configuration loading from TOML files with support for multiple environments

use crate::errors::{ConfigError, LoquatError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::net::SocketAddr;
use std::net::AddrParseError;

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

/// Validation trait for configuration structures
pub trait Validate {
    /// Validate the configuration
    /// Returns Ok(()) if valid, Err(ConfigError) if invalid
    fn validate(&self) -> Result<()>;
}

/// General configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Environment name (dev, test, prod)
    pub environment: String,
    /// Framework name
    pub name: String,
}

impl Validate for GeneralConfig {
    fn validate(&self) -> Result<()> {
        // Check environment is not empty
        if self.environment.trim().is_empty() {
            return Err(ConfigError::ValidationError(
                "GeneralConfig: environment cannot be empty".to_string()
            ).into());
        }
        
        // Validate environment is one of allowed values
        let valid_envs = ["dev", "test", "prod"];
        if !valid_envs.contains(&self.environment.as_str()) {
            return Err(ConfigError::ValidationError(
                format!("GeneralConfig: environment must be one of: {}", valid_envs.join(", "))
            ).into());
        }
        
        Ok(())
    }
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

impl Validate for LoggingConfig {
    fn validate(&self) -> Result<()> {
        // Validate log level
        let valid_levels = ["Trace", "Debug", "Info", "Warn", "Error"];
        let level_lower = self.level.to_lowercase();
        if !valid_levels.iter().any(|l| l.to_lowercase() == level_lower) {
            return Err(ConfigError::ValidationError(
                format!("LoggingConfig: level must be one of: {}", valid_levels.join(", "))
            ).into());
        }
        
        // Validate log format
        let valid_formats = ["text", "json"];
        let format_lower = self.format.to_lowercase();
        if !valid_formats.contains(&format_lower.as_str()) {
            return Err(ConfigError::ValidationError(
                format!("LoggingConfig: format must be one of: {}", valid_formats.join(", "))
            ).into());
        }
        
        // Validate log output
        let valid_outputs = ["console", "file", "combined"];
        let output_lower = self.output.to_lowercase();
        if !valid_outputs.contains(&output_lower.as_str()) {
            return Err(ConfigError::ValidationError(
                format!("LoggingConfig: output must be one of: {}", valid_outputs.join(", "))
            ).into());
        }
        
        // Validate file_path if output is file or combined
        if output_lower == "file" || output_lower == "combined" {
            if self.file_path.trim().is_empty() {
                return Err(ConfigError::ValidationError(
                    "LoggingConfig: file_path cannot be empty when output is 'file' or 'combined'".to_string()
                ).into());
            }
            
            // Check if parent directory exists or can be created
            if let Some(parent) = PathBuf::from(&self.file_path).parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    return Err(ConfigError::ValidationError(
                        format!("LoggingConfig: log directory '{}' does not exist", parent.display())
                    ).into());
                }
            }
        }
        
        Ok(())
    }
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

impl Validate for PluginConfig {
    fn validate(&self) -> Result<()> {
        // Validate plugin_dir if plugins are enabled
        if self.enabled {
            if self.plugin_dir.trim().is_empty() {
                return Err(ConfigError::ValidationError(
                    "PluginConfig: plugin_dir cannot be empty when enabled".to_string()
                ).into());
            }
            
            // Check if plugin_dir exists
            let plugin_path = PathBuf::from(&self.plugin_dir);
            if !plugin_path.exists() {
                return Err(ConfigError::ValidationError(
                    format!("PluginConfig: plugin_dir '{}' does not exist", plugin_path.display())
                ).into());
            }
            
            // Validate hot_reload_interval is at least 1 second
            if self.enable_hot_reload && self.hot_reload_interval == 0 {
                return Err(ConfigError::ValidationError(
                    "PluginConfig: hot_reload_interval must be at least 1 second when hot reload is enabled".to_string()
                ).into());
            }
            
            // Validate whitelist and blacklist strategy
            if !self.whitelist.is_empty() && !self.blacklist.is_empty() {
                // Log a warning but don't fail
                // Both can be used together, but blacklist takes precedence
            }
        }
        
        Ok(())
    }
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

impl PluginConfig {
    /// Check if plugin should be loaded based on whitelist/blacklist
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

impl Validate for AdapterConfig {
    fn validate(&self) -> Result<()> {
        // Validate adapter_dir if adapters are enabled
        if self.enabled {
            if self.adapter_dir.trim().is_empty() {
                return Err(ConfigError::ValidationError(
                    "AdapterConfig: adapter_dir cannot be empty when enabled".to_string()
                ).into());
            }
            
            // Check if adapter_dir exists
            let adapter_path = PathBuf::from(&self.adapter_dir);
            if !adapter_path.exists() {
                return Err(ConfigError::ValidationError(
                    format!("AdapterConfig: adapter_dir '{}' does not exist", adapter_path.display())
                ).into());
            }
            
            // Validate hot_reload_interval is at least 1 second
            if self.enable_hot_reload && self.hot_reload_interval == 0 {
                return Err(ConfigError::ValidationError(
                    "AdapterConfig: hot_reload_interval must be at least 1 second when hot reload is enabled".to_string()
                ).into());
            }
            
            // Validate whitelist and blacklist strategy
            if !self.whitelist.is_empty() && !self.blacklist.is_empty() {
                // Log a warning but don't fail
                // Both can be used together, but blacklist takes precedence
            }
        }
        
        Ok(())
    }
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            adapter_dir: "./adapters".to_string(),
            auto_load: true,
            enable_hot_reload: true,
            hot_reload_interval: 10,
            whitelist: Vec::new(),
            blacklist: Vec::new(),
        }
    }
}

impl AdapterConfig {
    /// Check if adapter should be loaded based on whitelist/blacklist
    /// Blacklist takes priority over whitelist
    /// If whitelist is not empty, only whitelisted adapters are loaded (except those in blacklist)
    /// If whitelist is empty, all adapters are loaded except those in blacklist
    pub fn should_load(&self, adapter_name: &str) -> bool {
        // Check blacklist first - highest priority
        if self.blacklist.contains(&adapter_name.to_string()) {
            return false;
        }
        
        // If whitelist is empty, load all remaining (not blacklisted)
        if self.whitelist.is_empty() {
            return true;
        }
        
        // If whitelist is not empty, only load whitelisted adapters
        // Note: This means adapters not in whitelist won't load even if not blacklisted
        self.whitelist.contains(&adapter_name.to_string())
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

impl Validate for EngineConfig {
    fn validate(&self) -> Result<()> {
        // EngineConfig doesn't have strict validation requirements
        // All fields are boolean flags that can be any combination
        Ok(())
    }
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
    /// Enable CORS
    pub enable_cors: bool,
}

impl Validate for WebConfig {
    fn validate(&self) -> Result<()> {
        // Validate host and port if web server is enabled
        if self.enabled {
            // Validate host format
            if self.host.trim().is_empty() {
                return Err(ConfigError::ValidationError(
                    "WebConfig: host cannot be empty when enabled".to_string()
                ).into());
            }
            
            // Try to parse as socket address to validate format
            let addr_str = format!("{}:{}", self.host, self.port);
            if let Err(e) = addr_str.parse::<SocketAddr>() {
                // If parsing as SocketAddr fails, try to parse host as IP or hostname
                // Port validation is separate
                return Err(ConfigError::ValidationError(
                    format!("WebConfig: invalid host format '{}': {}", self.host, e)
                ).into());
            }
            
            // Validate port range (1-65535)
            if self.port == 0 {
                return Err(ConfigError::ValidationError(
                    "WebConfig: port must be between 1 and 65535".to_string()
                ).into());
            }
            
            // Check for common reserved ports
            if self.port < 1024 {
                // Warning for privileged ports, but don't fail
                // Could be intentional in production
            }
        }
        
        Ok(())
    }
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 8080,
            enable_cors: true,
        }
    }
}

impl Validate for LoquatConfig {
    fn validate(&self) -> Result<()> {
        // Validate all sub-configurations
        self.general.validate()
            .map_err(|e| LoquatError::from(ConfigError::ValidationError(
                format!("Failed to validate general config: {}", e)
            )))?;
        
        self.logging.validate()
            .map_err(|e| LoquatError::from(ConfigError::ValidationError(
                format!("Failed to validate logging config: {}", e)
            )))?;
        
        self.plugins.validate()
            .map_err(|e| LoquatError::from(ConfigError::ValidationError(
                format!("Failed to validate plugins config: {}", e)
            )))?;
        
        self.adapters.validate()
            .map_err(|e| LoquatError::from(ConfigError::ValidationError(
                format!("Failed to validate adapters config: {}", e)
            )))?;
        
        self.engine.validate()
            .map_err(|e| LoquatError::from(ConfigError::ValidationError(
                format!("Failed to validate engine config: {}", e)
            )))?;
        
        self.web.validate()
            .map_err(|e| LoquatError::from(ConfigError::ValidationError(
                format!("Failed to validate web config: {}", e)
            )))?;
        
        Ok(())
    }
}

impl LoquatConfig {
    /// Load configuration from a TOML file
    pub fn from_toml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| ConfigError::LoadError(format!("Failed to read config file '{}': {}", path.as_ref().display(), e)))?;
        
        let config: Self = toml::from_str(&content)
            .map_err(|e| LoquatError::from(ConfigError::InvalidFormat(format!("Invalid TOML format in '{}': {}", path.as_ref().display(), e))))?;
        
        // Validate configuration after loading
        config.validate()?;
        
        Ok(config)
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
        
        // Validate final merged configuration
        config.validate()?;
        
        Ok(config)
    }
    
    /// Merge with another configuration (environment overrides default)
    /// Non-default values in other override values in self
    pub fn merge(&mut self, other: &LoquatConfig) -> Result<()> {
        // Helper function to merge string values
        fn merge_string(target: &mut String, source: &str, default: &str) {
            if source != default {
                *target = source.to_string();
            }
        }
        
        // Helper function to merge bool values
        fn merge_bool(target: &mut bool, source: bool, default: bool) {
            if source != default {
                *target = source;
            }
        }
        
        // Helper function to merge u64 values
        fn merge_u64(target: &mut u64, source: u64, default: u64) {
            if source != default {
                *target = source;
            }
        }
        
        // Helper function to merge u16 values
        fn merge_u16(target: &mut u16, source: u16, default: u16) {
            if source != default {
                *target = source;
            }
        }
        
        // Helper function to merge Vec values
        fn merge_vec<T: Clone>(target: &mut Vec<T>, source: &[T]) {
            if !source.is_empty() {
                *target = source.to_vec();
            }
        }
        
        // Merge general config
        let general_default = GeneralConfig::default();
        merge_string(&mut self.general.environment, &other.general.environment, &general_default.environment);
        merge_string(&mut self.general.name, &other.general.name, &general_default.name);
        
        // Merge logging config
        let logging_default = LoggingConfig::default();
        merge_string(&mut self.logging.level, &other.logging.level, &logging_default.level);
        merge_string(&mut self.logging.format, &other.logging.format, &logging_default.format);
        merge_string(&mut self.logging.output, &other.logging.output, &logging_default.output);
        merge_string(&mut self.logging.file_path, &other.logging.file_path, &logging_default.file_path);
        merge_bool(&mut self.logging.enable_colors, other.logging.enable_colors, logging_default.enable_colors);
        
        // Merge plugin config
        let plugin_default = PluginConfig::default();
        merge_bool(&mut self.plugins.enabled, other.plugins.enabled, plugin_default.enabled);
        merge_string(&mut self.plugins.plugin_dir, &other.plugins.plugin_dir, &plugin_default.plugin_dir);
        merge_bool(&mut self.plugins.auto_load, other.plugins.auto_load, plugin_default.auto_load);
        merge_bool(&mut self.plugins.enable_hot_reload, other.plugins.enable_hot_reload, plugin_default.enable_hot_reload);
        merge_u64(&mut self.plugins.hot_reload_interval, other.plugins.hot_reload_interval, plugin_default.hot_reload_interval);
        merge_vec(&mut self.plugins.whitelist, &other.plugins.whitelist);
        merge_vec(&mut self.plugins.blacklist, &other.plugins.blacklist);
        
        // Merge adapter config
        let adapter_default = AdapterConfig::default();
        merge_bool(&mut self.adapters.enabled, other.adapters.enabled, adapter_default.enabled);
        merge_string(&mut self.adapters.adapter_dir, &other.adapters.adapter_dir, &adapter_default.adapter_dir);
        merge_bool(&mut self.adapters.auto_load, other.adapters.auto_load, adapter_default.auto_load);
        merge_bool(&mut self.adapters.enable_hot_reload, other.adapters.enable_hot_reload, adapter_default.enable_hot_reload);
        merge_u64(&mut self.adapters.hot_reload_interval, other.adapters.hot_reload_interval, adapter_default.hot_reload_interval);
        merge_vec(&mut self.adapters.whitelist, &other.adapters.whitelist);
        merge_vec(&mut self.adapters.blacklist, &other.adapters.blacklist);
        
        // Merge engine config
        let engine_default = EngineConfig::default();
        merge_bool(&mut self.engine.auto_route, other.engine.auto_route, engine_default.auto_route);
        merge_bool(&mut self.engine.auto_create_channels, other.engine.auto_create_channels, engine_default.auto_create_channels);
        merge_bool(&mut self.engine.auto_initialize, other.engine.auto_initialize, engine_default.auto_initialize);
        
        // Merge web config
        let web_default = WebConfig::default();
        merge_bool(&mut self.web.enabled, other.web.enabled, web_default.enabled);
        merge_string(&mut self.web.host, &other.web.host, &web_default.host);
        merge_u16(&mut self.web.port, other.web.port, web_default.port);
        
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
    
    #[test]
    fn test_validate_general_config() {
        let mut config = GeneralConfig::default();
        assert!(config.validate().is_ok());
        
        // Test empty environment
        config.environment = "".to_string();
        assert!(config.validate().is_err());
        
        // Test invalid environment
        config.environment = "invalid_env".to_string();
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_validate_logging_config() {
        let mut config = LoggingConfig::default();
        assert!(config.validate().is_ok());
        
        // Test invalid log level
        config.level = "InvalidLevel".to_string();
        assert!(config.validate().is_err());
        
        // Test invalid format
        config.level = "Info".to_string();
        config.format = "xml".to_string();
        assert!(config.validate().is_err());
        
        // Test invalid output
        config.format = "text".to_string();
        config.output = "stdout".to_string();
        assert!(config.validate().is_err());
        
        // Test file output with empty path
        config.output = "file".to_string();
        config.file_path = "".to_string();
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_validate_plugin_config() {
        let mut config = PluginConfig::default();
        assert!(config.validate().is_ok());
        
        // Test empty plugin_dir when enabled
        config.enabled = true;
        config.plugin_dir = "".to_string();
        assert!(config.validate().is_err());
        
        // Test hot_reload_interval must be at least 1
        config.plugin_dir = "./plugins".to_string();
        config.enable_hot_reload = true;
        config.hot_reload_interval = 0;
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_validate_adapter_config() {
        let mut config = AdapterConfig::default();
        assert!(config.validate().is_ok());
        
        // Test empty adapter_dir when enabled
        config.enabled = true;
        config.adapter_dir = "".to_string();
        assert!(config.validate().is_err());
        
        // Test hot_reload_interval must be at least 1
        config.adapter_dir = "./adapters".to_string();
        config.enable_hot_reload = true;
        config.hot_reload_interval = 0;
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_validate_web_config() {
        let mut config = WebConfig::default();
        assert!(config.validate().is_ok());
        
        // Test empty host when enabled
        config.enabled = true;
        config.host = "".to_string();
        assert!(config.validate().is_err());
        
        // Test invalid host format
        config.host = "invalid host".to_string();
        assert!(config.validate().is_err());
        
        // Test port 0 is invalid
        config.host = "127.0.0.1".to_string();
        config.port = 0;
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_validate_full_config() {
        let config = LoquatConfig::default();
        assert!(config.validate().is_ok());
    }
}
