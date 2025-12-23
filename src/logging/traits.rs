//! Core traits for the logging system

use crate::errors::{LoggingError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Log levels in order of severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

impl LogLevel {
    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "TRACE" => Ok(LogLevel::Trace),
            "DEBUG" => Ok(LogLevel::Debug),
            "INFO" => Ok(LogLevel::Info),
            "WARN" => Ok(LogLevel::Warn),
            "ERROR" => Ok(LogLevel::Error),
            _ => Err(LoggingError::InvalidLevel(s.to_string()).into()),
        }
    }

    /// Check if this level should be logged given the minimum level
    pub fn should_log(&self, min_level: LogLevel) -> bool {
        *self >= min_level
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Log context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogContext {
    /// Unique request/correlation ID
    pub correlation_id: Option<String>,
    
    /// User ID if available
    pub user_id: Option<String>,
    
    /// Session ID
    pub session_id: Option<String>,
    
    /// Request path/method for web requests
    pub request_info: Option<RequestInfo>,
    
    /// Component/module name
    pub component: Option<String>,
    
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl LogContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self {
            correlation_id: None,
            user_id: None,
            session_id: None,
            request_info: None,
            component: None,
            metadata: HashMap::new(),
        }
    }

    /// Get the current context (in a real implementation, this would use thread-local or async context)
    pub fn current() -> Self {
        // Simplified implementation - in production, use proper context propagation
        Self::new()
    }

    /// Create context with correlation ID
    pub fn with_correlation_id(correlation_id: String) -> Self {
        let mut ctx = Self::new();
        ctx.correlation_id = Some(correlation_id);
        ctx
    }

    /// Add metadata
    pub fn with_metadata<K: Into<String>, V: Serialize>(mut self, key: K, value: V) -> Self {
        if let Ok(val) = serde_json::to_value(value) {
            self.metadata.insert(key.into(), val);
        }
        self
    }

    /// Add metadata to context (chaining version)
    pub fn add<K: Into<String>, V: Serialize>(&mut self, key: K, value: V) -> &mut Self {
        if let Ok(val) = serde_json::to_value(value) {
            self.metadata.insert(key.into(), val);
        }
        self
    }

    /// Set component
    pub fn with_component(mut self, component: &str) -> Self {
        self.component = Some(component.to_string());
        self
    }
}

impl Default for LogContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Request information for web requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestInfo {
    pub method: String,
    pub path: String,
    pub user_agent: Option<String>,
    pub remote_addr: Option<String>,
}

impl RequestInfo {
    pub fn new(method: &str, path: &str) -> Self {
        Self {
            method: method.to_string(),
            path: path.to_string(),
            user_agent: None,
            remote_addr: None,
        }
    }
}

/// A single log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: LogLevel,
    pub message: String,
    pub context: LogContext,
    pub module_path: Option<String>,
    pub file: Option<String>,
    pub line: Option<u32>,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(level: LogLevel, message: String, context: LogContext) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            level,
            message,
            context,
            module_path: None,
            file: None,
            line: None,
        }
    }

    /// Add source location information
    pub fn with_location(mut self, module_path: &str, file: &str, line: u32) -> Self {
        self.module_path = Some(module_path.to_string());
        self.file = Some(file.to_string());
        self.line = Some(line);
        self
    }
}

/// Core logger trait
#[async_trait]
pub trait Logger: Send + Sync {
    /// Log a message with the given level and context
    fn log(&self, level: LogLevel, message: &str, context: &LogContext);

    /// Log a structured entry
    fn log_entry(&self, entry: LogEntry);

    /// Set the minimum log level
    fn set_level(&self, level: LogLevel);

    /// Get the current log level
    fn get_level(&self) -> LogLevel;

    /// Check if a level should be logged
    fn is_enabled(&self, level: LogLevel) -> bool {
        level.should_log(self.get_level())
    }

    /// Initialize the logger (async for potentially expensive setup)
    async fn init_async(&self) -> Result<()> {
        Ok(())
    }

    /// Synchronous initialization (calls async internally)
    fn init(&self) -> Result<()> {
        // Simple blocking implementation - in production, use proper async runtime
        std::thread::sleep(std::time::Duration::from_millis(1));
        Ok(())
    }

    /// Flush any pending logs
    fn flush(&self);

    /// Shutdown the logger gracefully
    async fn shutdown_async(&self) -> Result<()> {
        self.flush();
        Ok(())
    }

    /// Convenience method to log info level message
    fn info(&self, message: &str) {
        self.log(LogLevel::Info, message, &LogContext::current());
    }

    /// Convenience method to log with context
    fn log_with_context(&self, level: LogLevel, message: &str, context: &LogContext) {
        self.log(level, message, context);
    }
}

/// Log formatter trait
pub trait LogFormatter: Send + Sync {
    /// Format a log entry into a string
    fn format(&self, entry: &LogEntry) -> String;

    /// Format multiple entries (for batch processing)
    fn format_batch(&self, entries: &[LogEntry]) -> Vec<String> {
        entries.iter().map(|e| self.format(e)).collect()
    }
}

/// Log writer trait for output destinations
#[async_trait]
pub trait LogWriter: Send + Sync {
    /// Write a formatted log entry
    async fn write_async(&self, formatted: &str) -> Result<()>;

    /// Write multiple entries (for batch processing)
    async fn write_batch_async(&self, formatted_entries: &[String]) -> Result<()> {
        for entry in formatted_entries {
            self.write_async(entry).await?;
        }
        Ok(())
    }

    /// Synchronous write
    fn write(&self, formatted: &str) -> Result<()>;

    /// Flush pending writes
    fn flush(&self) -> Result<()>;

    /// Close the writer
    async fn close_async(&self) -> Result<()> {
        self.flush()?;
        Ok(())
    }
}

/// Logger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggerConfig {
    pub level: LogLevel,
    pub format: LogFormat,
    pub output: LogOutput,
    pub enable_async: bool,
    pub buffer_size: Option<usize>,
    pub max_file_size: Option<u64>,
    pub max_files: Option<usize>,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Json,
            output: LogOutput::Console,
            enable_async: true,
            buffer_size: Some(1000),
            max_file_size: Some(10 * 1024 * 1024), // 10MB
            max_files: Some(5),
        }
    }
}

/// Log format types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    Json,
    Text,
    Compact,
    Custom(String),
}

/// Log output destinations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogOutput {
    Console,
    File { path: String },
    Both { path: String },
    Custom(String),
}

/// Factory trait for creating loggers
pub trait LoggerFactory: Send + Sync {
    fn create_logger(&self, config: &LoggerConfig) -> Result<Arc<dyn Logger>>;
}
