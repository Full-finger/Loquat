//! TUI Log Writer
//!
//! Implements LogWriter trait to send logs to TUI via mpsc channel

use crate::errors::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;

/// TUI-specific log message
#[derive(Debug, Clone)]
pub struct TuiLogMessage {
    /// Log level
    pub level: crate::logging::LogLevel,
    /// Message content
    pub message: String,
    /// Timestamp
    pub timestamp: String,
}

/// TUI Log Writer
/// Sends logs to TUI via channel
pub struct TuiLogWriter {
    /// Channel sender
    sender: mpsc::UnboundedSender<TuiLogMessage>,
}

impl TuiLogWriter {
    /// Create a new TUI log writer
    pub fn new(sender: mpsc::UnboundedSender<TuiLogMessage>) -> Self {
        Self { sender }
    }

    /// Create a new TUI log writer with Arc sender
    pub fn new_arc(sender: Arc<mpsc::UnboundedSender<TuiLogMessage>>) -> Self {
        Self { sender: (*sender).clone() }
    }
}

#[async_trait]
impl crate::logging::LogWriter for TuiLogWriter {
    async fn write_async(&self, formatted: &str) -> Result<()> {
        // Parse the formatted log entry
        // For simplicity, we'll extract the basic info
        // In production, this would parse the JSON or text format properly
        
        // Try to parse as JSON (from JsonFormatter)
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(formatted) {
            if let (Some(level), Some(message), Some(timestamp)) = (
                json_value.get("level").and_then(|v| v.as_str()),
                json_value.get("message").and_then(|v| v.as_str()),
                json_value.get("timestamp").and_then(|v| v.as_str()),
            ) {
                // Parse level
                let log_level = match level {
                    "TRACE" => crate::logging::LogLevel::Trace,
                    "DEBUG" => crate::logging::LogLevel::Debug,
                    "INFO" => crate::logging::LogLevel::Info,
                    "WARN" => crate::logging::LogLevel::Warn,
                    "ERROR" => crate::logging::LogLevel::Error,
                    _ => crate::logging::LogLevel::Info,
                };

                // Convert timestamp (from ISO 8601 to HH:MM:SS)
                let short_timestamp = timestamp
                    .split('T')
                    .nth(1)
                    .and_then(|t| t.split('.').next())
                    .unwrap_or(timestamp)
                    .to_string();

                let log_msg = TuiLogMessage {
                    level: log_level,
                    message: message.to_string(),
                    timestamp: short_timestamp,
                };

                // Send to channel (ignore if receiver is dropped)
                let _ = self.sender.send(log_msg);
            }
        } else {
            // Fallback: treat entire formatted string as message
            let log_msg = TuiLogMessage {
                level: crate::logging::LogLevel::Info,
                message: formatted.to_string(),
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            };
            let _ = self.sender.send(log_msg);
        }

        Ok(())
    }

    fn write(&self, formatted: &str) -> Result<()> {
        // Synchronous write - we'll spawn an async task
        let sender = self.sender.clone();
        let formatted = formatted.to_string();
        
        tokio::spawn(async move {
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&formatted) {
                if let (Some(level), Some(message), Some(timestamp)) = (
                    json_value.get("level").and_then(|v| v.as_str()),
                    json_value.get("message").and_then(|v| v.as_str()),
                    json_value.get("timestamp").and_then(|v| v.as_str()),
                ) {
                    let log_level = match level {
                        "TRACE" => crate::logging::LogLevel::Trace,
                        "DEBUG" => crate::logging::LogLevel::Debug,
                        "INFO" => crate::logging::LogLevel::Info,
                        "WARN" => crate::logging::LogLevel::Warn,
                        "ERROR" => crate::logging::LogLevel::Error,
                        _ => crate::logging::LogLevel::Info,
                    };

                    let short_timestamp = timestamp
                        .split('T')
                        .nth(1)
                        .and_then(|t| t.split('.').next())
                        .unwrap_or(timestamp)
                        .to_string();

                    let log_msg = TuiLogMessage {
                        level: log_level,
                        message: message.to_string(),
                        timestamp: short_timestamp,
                    };

                    let _ = sender.send(log_msg);
                }
            } else {
                let log_msg = TuiLogMessage {
                    level: crate::logging::LogLevel::Info,
                    message: formatted,
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                };
                let _ = sender.send(log_msg);
            }
        });

        Ok(())
    }

    fn flush(&self) -> Result<()> {
        // Nothing to flush for channel-based writer
        Ok(())
    }

    async fn flush_async(&self) -> Result<()> {
        Ok(())
    }

    async fn close_async(&self) -> Result<()> {
        Ok(())
    }
}
