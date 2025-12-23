//! Adapter configuration definitions

use serde::{Deserialize, Serialize};

/// Adapter connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Connection type (ws, http, tcp, etc.)
    pub conn_type: String,
    
    /// Connection URL or address
    pub url: String,
    
    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    
    /// Whether to use TLS/SSL
    #[serde(default)]
    pub use_tls: bool,
    
    /// Keep-alive interval in seconds
    #[serde(default)]
    pub keep_alive: Option<u64>,
    
    /// Maximum reconnect attempts
    #[serde(default = "default_max_reconnect")]
    pub max_reconnect: u32,
    
    /// Additional connection parameters
    #[serde(default)]
    pub params: serde_json::Value,
}

fn default_timeout() -> u64 {
    30
}

fn default_max_reconnect() -> u32 {
    5
}

/// Heartbeat configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    /// Heartbeat interval in seconds
    pub interval: u64,
    
    /// Heartbeat timeout in seconds
    #[serde(default)]
    pub timeout: Option<u64>,
    
    /// Whether to enable heartbeat
    #[serde(default = "default_heartbeat_enabled")]
    pub enabled: bool,
}

fn default_heartbeat_enabled() -> bool {
    true
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    #[serde(default = "default_max_retry")]
    pub max_attempts: u32,
    
    /// Initial retry delay in milliseconds
    #[serde(default = "default_initial_delay")]
    pub initial_delay: u64,
    
    /// Maximum retry delay in milliseconds
    #[serde(default = "default_max_delay")]
    pub max_delay: u64,
    
    /// Backoff multiplier
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
}

fn default_max_retry() -> u32 {
    3
}

fn default_initial_delay() -> u64 {
    1000
}

fn default_max_delay() -> u64 {
    30000
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

/// Adapter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    /// Adapter type (qq, wechat, telegram, etc.)
    pub adapter_type: String,
    
    /// Adapter unique identifier
    pub adapter_id: String,
    
    /// Whether the adapter is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    
    /// Adapter name (display name)
    #[serde(default)]
    pub name: Option<String>,
    
    /// Connection configuration
    pub connection: ConnectionConfig,
    
    /// Heartbeat configuration
    #[serde(default)]
    pub heartbeat: Option<HeartbeatConfig>,
    
    /// Retry configuration
    #[serde(default)]
    pub retry: Option<RetryConfig>,
    
    /// Platform-specific configuration
    #[serde(default)]
    pub platform: serde_json::Value,
    
    /// Additional metadata
    #[serde(default)]
    pub extra: serde_json::Value,
}

fn default_enabled() -> bool {
    true
}

impl AdapterConfig {
    /// Create a new adapter configuration
    pub fn new(adapter_type: &str, adapter_id: &str, url: &str) -> Self {
        Self {
            adapter_type: adapter_type.to_string(),
            adapter_id: adapter_id.to_string(),
            enabled: true,
            name: None,
            connection: ConnectionConfig {
                conn_type: "ws".to_string(),
                url: url.to_string(),
                timeout: 30,
                use_tls: false,
                keep_alive: None,
                max_reconnect: 5,
                params: serde_json::json!({}),
            },
            heartbeat: None,
            retry: None,
            platform: serde_json::json!({}),
            extra: serde_json::json!({}),
        }
    }
    
    /// Set adapter name
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }
    
    /// Set whether adapter is enabled
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    
    /// Set heartbeat configuration
    pub fn with_heartbeat(mut self, heartbeat: HeartbeatConfig) -> Self {
        self.heartbeat = Some(heartbeat);
        self
    }
    
    /// Set retry configuration
    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.retry = Some(retry);
        self
    }
    
    /// Add platform-specific configuration
    pub fn with_platform_config<K: Into<String>, V: Serialize>(
        mut self,
        key: K,
        value: V,
    ) -> Result<Self, serde_json::Error> {
        let platform_config = if self.platform.is_null() {
            serde_json::Value::Object(serde_json::Map::new())
        } else {
            self.platform.clone()
        };
        
        let mut obj = if let serde_json::Value::Object(o) = platform_config {
            o
        } else {
            serde_json::Map::new()
        };
        
        obj.insert(key.into(), serde_json::to_value(value)?);
        self.platform = serde_json::Value::Object(obj);
        Ok(self)
    }
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self::new("unknown", "default", "ws://localhost")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_config_creation() {
        let config = AdapterConfig::new("qq", "qq-001", "ws://localhost:8080");
        
        assert_eq!(config.adapter_type, "qq");
        assert_eq!(config.adapter_id, "qq-001");
        assert_eq!(config.connection.url, "ws://localhost:8080");
        assert!(config.enabled);
    }

    #[test]
    fn test_adapter_config_builder() {
        let config = AdapterConfig::new("telegram", "tg-001", "ws://api.telegram.org")
            .with_name("Telegram Adapter")
            .with_enabled(true);
        
        assert_eq!(config.name, Some("Telegram Adapter".to_string()));
        assert!(config.enabled);
    }

    #[test]
    fn test_heartbeat_config() {
        let heartbeat = HeartbeatConfig {
            interval: 30,
            timeout: Some(10),
            enabled: true,
        };
        
        let config = AdapterConfig::new("qq", "qq-001", "ws://localhost")
            .with_heartbeat(heartbeat);
        
        assert!(config.heartbeat.is_some());
        assert_eq!(config.heartbeat.unwrap().interval, 30);
    }

    #[test]
    fn test_retry_config() {
        let retry = RetryConfig {
            max_attempts: 5,
            initial_delay: 1000,
            max_delay: 60000,
            backoff_multiplier: 2.0,
        };
        
        let config = AdapterConfig::new("wechat", "wx-001", "ws://localhost")
            .with_retry(retry);
        
        assert!(config.retry.is_some());
        assert_eq!(config.retry.unwrap().max_attempts, 5);
    }

    #[test]
    fn test_platform_config() {
        let config = AdapterConfig::new("qq", "qq-001", "ws://localhost")
            .with_platform_config("app_id", "123456")
            .unwrap()
            .with_platform_config("app_secret", "abcdef")
            .unwrap();
        
        assert_eq!(config.platform["app_id"], "123456");
        assert_eq!(config.platform["app_secret"], "abcdef");
    }

    #[test]
    fn test_connection_config_defaults() {
        let config = AdapterConfig::new("test", "test-001", "ws://localhost");
        
        assert_eq!(config.connection.timeout, 30);
        assert_eq!(config.connection.max_reconnect, 5);
        assert!(!config.connection.use_tls);
    }

    #[test]
    fn test_retry_config_defaults() {
        let retry = RetryConfig {
            max_attempts: 3,
            initial_delay: 1000,
            max_delay: 30000,
            backoff_multiplier: 2.0,
        };
        
        assert_eq!(retry.max_attempts, 3);
        assert_eq!(retry.initial_delay, 1000);
        assert_eq!(retry.max_delay, 30000);
        assert_eq!(retry.backoff_multiplier, 2.0);
    }
}
