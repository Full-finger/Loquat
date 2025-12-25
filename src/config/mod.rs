//! Configuration management for Loquat framework

pub mod loquat_config;

pub use loquat_config::*;
pub use crate::plugins::types::PluginConfig;
pub use crate::adapters::config::AdapterConfig;

// Re-export legacy config structures for backward compatibility
use crate::errors::{ConfigError, LoquatError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Legacy main configuration structure for backward compatibility
#[deprecated(note = "Use loquat_config::LoquatConfig instead")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoquatConfig {
    /// Logging configuration
    pub logging: LoggingConfig,
    
    /// AOP configuration
    pub aop: AopConfig,
    
    /// Web service configuration
    pub web: WebConfig,
    
    /// Custom configuration values
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for LoquatConfig {
    fn default() -> Self {
        Self {
            logging: LoggingConfig::default(),
            aop: AopConfig::default(),
            web: WebConfig::default(),
            custom: HashMap::new(),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Default log level
    pub level: String,
    
    /// Output format (json, text)
    pub format: String,
    
    /// Whether to enable colors in console output
    pub colored: bool,
    
    /// File output configuration
    pub file: Option<FileLoggingConfig>,
    
    /// Console output configuration
    pub console: Option<ConsoleLoggingConfig>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "text".to_string(),
            colored: true,
            file: None,
            console: Some(ConsoleLoggingConfig::default()),
        }
    }
}

/// File logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLoggingConfig {
    /// Path to log file
    pub path: String,
    
    /// Whether to rotate logs
    pub rotate: bool,
    
    /// Maximum file size before rotation (bytes)
    pub max_size: Option<usize>,
    
    /// Maximum number of rotated files to keep
    pub max_files: Option<usize>,
    
    /// Whether to compress rotated files
    pub compress: bool,
}

impl Default for FileLoggingConfig {
    fn default() -> Self {
        Self {
            path: "logs/loquat.log".to_string(),
            rotate: true,
            max_size: Some(10 * 1024 * 1024), // 10MB
            max_files: Some(5),
            compress: true,
        }
    }
}

/// Console logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleLoggingConfig {
    /// Whether to enable console output
    pub enabled: bool,
    
    /// Whether to use colors
    pub colored: bool,
    
    /// Whether to route stderr to error level
    pub route_stderr: bool,
}

impl Default for ConsoleLoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            colored: true,
            route_stderr: true,
        }
    }
}

/// AOP configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AopConfig {
    /// Whether AOP is enabled
    pub enabled: bool,
    
    /// Default aspects to apply
    pub default_aspects: Vec<String>,
    
    /// Aspect-specific configurations
    pub aspects: HashMap<String, AspectConfig>,
}

impl Default for AopConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_aspects: vec![
                "logging".to_string(),
                "error_tracking".to_string(),
                "performance".to_string(),
            ],
            aspects: HashMap::new(),
        }
    }
}

/// Individual aspect configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AspectConfig {
    /// Whether this aspect is enabled
    pub enabled: bool,
    
    /// Aspect-specific settings
    pub settings: HashMap<String, serde_json::Value>,
}

impl Default for AspectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            settings: HashMap::new(),
        }
    }
}

/// Web service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    /// Server host
    pub host: String,
    
    /// Server port
    pub port: u16,
    
    /// Whether to enable HTTPS
    pub https: bool,
    
    /// TLS certificate path (if HTTPS enabled)
    pub cert_path: Option<String>,
    
    /// TLS private key path (if HTTPS enabled)
    pub key_path: Option<String>,
    
    /// Request timeout in seconds
    pub request_timeout: u64,
    
    /// Maximum request body size (bytes)
    pub max_request_size: Option<usize>,
    
    /// CORS configuration
    pub cors: Option<CorsConfig>,
    
    /// Middleware configuration
    pub middleware: Vec<String>,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            https: false,
            cert_path: None,
            key_path: None,
            request_timeout: 30,
            max_request_size: Some(10 * 1024 * 1024), // 10MB
            cors: Some(CorsConfig::default()),
            middleware: vec![],
        }
    }
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Allowed origins
    pub allowed_origins: Vec<String>,
    
    /// Allowed methods
    pub allowed_methods: Vec<String>,
    
    /// Allowed headers
    pub allowed_headers: Vec<String>,
    
    /// Whether to allow credentials
    pub allow_credentials: bool,
    
    /// Max age for preflight requests (seconds)
    pub max_age: Option<u64>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec!["*".to_string()],
            allow_credentials: false,
            max_age: Some(3600),
        }
    }
}

impl LoquatConfig {
    /// Load configuration from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::LoadError(format!("Failed to read config file: {}", e)))?;
        
        serde_json::from_str(&content)
            .map_err(|e| LoquatError::from(ConfigError::InvalidFormat(format!("Invalid JSON format: {}", e))))
    }
    
    /// Load configuration from a TOML file
    pub fn from_toml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::LoadError(format!("Failed to read TOML config file: {}", e)))?;
        
        toml::from_str(&content)
            .map_err(|e| LoquatError::from(ConfigError::InvalidFormat(format!("Invalid TOML format: {}", e))))
    }
    
    /// Save configuration to a file as JSON
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| LoquatError::from(ConfigError::InvalidFormat(format!("Failed to serialize config: {}", e))))?;
        
        std::fs::write(path, content)
            .map_err(|e| LoquatError::from(ConfigError::LoadError(format!("Failed to write config file: {}", e))))
    }
    
    /// Save configuration to a file as TOML
    pub fn to_toml_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| LoquatError::from(ConfigError::InvalidFormat(format!("Failed to serialize TOML config: {}", e))))?;
        
        std::fs::write(path, content)
            .map_err(|e| LoquatError::from(ConfigError::LoadError(format!("Failed to write TOML config file: {}", e))))
    }
    
    /// Get a custom configuration value
    pub fn get_custom<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        match self.custom.get(key) {
            Some(value) => {
                serde_json::from_value(value.clone())
                    .map(Some)
                    .map_err(|e| LoquatError::from(ConfigError::InvalidFormat(format!("Failed to deserialize custom value '{}': {}", key, e))))
            },
            None => Ok(None),
        }
    }
    
    /// Set a custom configuration value
    pub fn set_custom<T: Serialize>(&mut self, key: &str, value: T) -> Result<()> {
        let json_value = serde_json::to_value(value)
            .map_err(|e| LoquatError::from(ConfigError::InvalidFormat(format!("Failed to serialize custom value '{}': {}", key, e))))?;
        
        self.custom.insert(key.to_string(), json_value);
        Ok(())
    }
    
    /// Merge with another configuration
    pub fn merge(&mut self, other: &LoquatConfig) {
        // For simplicity, we just replace fields with non-default values from other
        // In a real implementation, you might want more sophisticated merging logic
        
        if other.logging.level != LoggingConfig::default().level {
            self.logging.level = other.logging.level.clone();
        }
        if other.logging.format != LoggingConfig::default().format {
            self.logging.format = other.logging.format.clone();
        }
        if other.logging.colored != LoggingConfig::default().colored {
            self.logging.colored = other.logging.colored;
        }
        
        // Merge file logging config
        if other.logging.file.is_some() {
            self.logging.file = other.logging.file.clone();
        }
        
        // Merge console logging config
        if other.logging.console.is_some() {
            self.logging.console = other.logging.console.clone();
        }
        
        // Merge AOP config
        if other.aop.enabled != AopConfig::default().enabled {
            self.aop.enabled = other.aop.enabled;
        }
        if !other.aop.default_aspects.is_empty() {
            self.aop.default_aspects = other.aop.default_aspects.clone();
        }
        
        // Merge web config
        if other.web.host != WebConfig::default().host {
            self.web.host = other.web.host.clone();
        }
        if other.web.port != WebConfig::default().port {
            self.web.port = other.web.port;
        }
        if other.web.https != WebConfig::default().https {
            self.web.https = other.web.https;
        }
        
        // Merge custom config
        for (key, value) in &other.custom {
            self.custom.insert(key.clone(), value.clone());
        }
    }
}

/// Configuration builder for programmatic configuration
pub struct ConfigBuilder {
    config: LoquatConfig,
}

impl ConfigBuilder {
    /// Create a new configuration builder
    pub fn new() -> Self {
        Self {
            config: LoquatConfig::default(),
        }
    }
    
    /// Set logging configuration
    pub fn logging(mut self, config: LoggingConfig) -> Self {
        self.config.logging = config;
        self
    }
    
    /// Set AOP configuration
    pub fn aop(mut self, config: AopConfig) -> Self {
        self.config.aop = config;
        self
    }
    
    /// Set web configuration
    pub fn web(mut self, config: WebConfig) -> Self {
        self.config.web = config;
        self
    }
    
    /// Set a custom configuration value
    pub fn custom<T: Serialize>(mut self, key: &str, value: T) -> Result<Self> {
        self.config.set_custom(key, value)?;
        Ok(self)
    }
    
    /// Build the configuration
    pub fn build(self) -> LoquatConfig {
        self.config
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LoquatConfig::default();
        
        assert_eq!(config.logging.level, "info");
        assert_eq!(config.logging.format, "text");
        assert!(config.logging.colored);
        assert!(config.aop.enabled);
        assert_eq!(config.web.host, "127.0.0.1");
        assert_eq!(config.web.port, 8080);
    }

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .custom("test_key", "test_value")
            .unwrap()
            .custom("number", 42)
            .unwrap()
            .build();
        
        assert_eq!(config.get_custom::<String>("test_key").unwrap().unwrap(), "test_value");
        assert_eq!(config.get_custom::<i64>("number").unwrap().unwrap(), 42);
    }

    #[test]
    fn test_config_merge() {
        let mut base = LoquatConfig::default();
        let mut other = LoquatConfig::default();
        
        other.logging.level = "debug".to_string();
        other.web.port = 9000;
        other.custom.insert("merged".to_string(), serde_json::Value::Bool(true));
        
        base.merge(&other);
        
        assert_eq!(base.logging.level, "debug");
        assert_eq!(base.web.port, 9000);
        assert!(base.custom.contains_key("merged"));
    }

    #[test]
    fn test_json_serialization() {
        let config = LoquatConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        
        let deserialized: LoquatConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.logging.level, config.logging.level);
        assert_eq!(deserialized.web.port, config.web.port);
    }
}
