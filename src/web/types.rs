//! API response types for Web service

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Success status
    pub success: bool,
    /// Response data
    pub data: Option<T>,
    /// Error message if any
    pub error: Option<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    /// Create a successful response
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
        }
    }

    /// Create an error response
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: Utc::now(),
        }
    }
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Service status
    pub status: String,
    /// Version
    pub version: String,
    /// Environment
    pub environment: String,
    /// Uptime in seconds
    pub uptime: u64,
    /// Plugin system status
    pub plugins_enabled: bool,
    /// Adapter system status
    pub adapters_enabled: bool,
}

/// Plugin information for API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Plugin name
    pub name: String,
    /// Plugin type
    pub plugin_type: String,
    /// Plugin status
    pub status: String,
    /// Version if available
    pub version: Option<String>,
    /// Author if available
    pub author: Option<String>,
    /// Description if available
    pub description: Option<String>,
}

/// Adapter information for API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterInfo {
    /// Adapter name
    pub name: String,
    /// Adapter status
    pub status: String,
    /// Version if available
    pub version: Option<String>,
    /// Description if available
    pub description: Option<String>,
}

/// Reload request body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReloadRequest {
    /// Whether to reload plugins
    pub plugins: Option<bool>,
    /// Whether to reload adapters
    pub adapters: Option<bool>,
}

/// Reload response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReloadResponse {
    /// Message
    pub message: String,
    /// Plugins reloaded count
    pub plugins_reloaded: u32,
    /// Adapters reloaded count
    pub adapters_reloaded: u32,
}

/// Configuration response (sanitized)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigResponse {
    /// Environment
    pub environment: String,
    /// Framework name
    pub name: String,
    /// Log level
    pub log_level: String,
    /// Log format
    pub log_format: String,
    /// Log output
    pub log_output: String,
    /// Plugins enabled
    pub plugins_enabled: bool,
    /// Adapters enabled
    pub adapters_enabled: bool,
    /// Web enabled
    pub web_enabled: bool,
    /// Web host
    pub web_host: String,
    /// Web port
    pub web_port: u16,
}
