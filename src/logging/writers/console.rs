//! Console writer implementation

use crate::errors::{LoggingError, Result};
use crate::logging::traits::LogWriter;
use async_trait::async_trait;
use std::io::{self, Write};
use tokio::sync::Mutex;

/// Console writer for output to stdout/stderr
#[derive(Debug)]
pub struct ConsoleWriter {
    use_stderr: bool,
    buffer: Mutex<String>,
}

impl ConsoleWriter {
    /// Create a new console writer that writes to stdout
    pub fn new() -> Self {
        Self {
            use_stderr: false,
            buffer: Mutex::new(String::new()),
        }
    }

    /// Create a console writer that writes to stderr
    pub fn stderr() -> Self {
        Self {
            use_stderr: true,
            buffer: Mutex::new(String::new()),
        }
    }

    /// Create a console writer with automatic error level routing
    /// ERROR and WARN go to stderr, others to stdout
    pub fn auto_routing() -> AutoRoutingConsoleWriter {
        AutoRoutingConsoleWriter::new()
    }

    /// Get to appropriate output stream
    fn get_output_stream(&self) -> Box<dyn Write + Send> {
        if self.use_stderr {
            Box::new(io::stderr())
        } else {
            Box::new(io::stdout())
        }
    }

    /// Write to console with buffering
    fn write_to_console(&self, message: &str) -> Result<()> {
        let mut stream = self.get_output_stream();
        
        // Write to message
        writeln!(stream, "{}", message)
            .map_err(|e| LoggingError::WriteError(e.to_string()))?;
        
        // Flush to ensure immediate output
        stream.flush()
            .map_err(|e| LoggingError::WriteError(e.to_string()))?;
        
        Ok(())
    }
}

impl Default for ConsoleWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LogWriter for ConsoleWriter {
    async fn write_async(&self, formatted: &str) -> Result<()> {
        // For console, async is just a wrapper around sync
        self.write(formatted)
    }

    fn write(&self, formatted: &str) -> Result<()> {
        self.write_to_console(formatted)
    }

    fn flush(&self) -> Result<()> {
        let mut stream = self.get_output_stream();
        stream.flush()
            .map_err(|e| LoggingError::WriteError(e.to_string()))?;
        Ok(())
    }

    async fn flush_async(&self) -> Result<()> {
        // Console flush is immediate, no async needed
        self.flush()
    }

    async fn close_async(&self) -> Result<()> {
        self.flush()?;
        Ok(())
    }
}

/// Console writer that automatically routes messages based on log level
#[derive(Debug)]
pub struct AutoRoutingConsoleWriter {
    stdout_writer: ConsoleWriter,
    stderr_writer: ConsoleWriter,
}

impl AutoRoutingConsoleWriter {
    /// Create a new auto-routing console writer
    pub fn new() -> Self {
        Self {
            stdout_writer: ConsoleWriter::new(),
            stderr_writer: ConsoleWriter::stderr(),
        }
    }

    /// Determine which writer to use based on log level
    fn extract_level_from_message(&self, formatted: &str) -> crate::logging::traits::LogLevel {
        // Try to extract level from common log formats
        if formatted.contains("\"level\":\"ERROR\"") || formatted.contains("[ERROR]") {
            crate::logging::traits::LogLevel::Error
        } else if formatted.contains("\"level\":\"WARN\"") || formatted.contains("[WARN]") {
            crate::logging::traits::LogLevel::Warn
        } else if formatted.contains("\"level\":\"INFO\"") || formatted.contains("[INFO]") {
            crate::logging::traits::LogLevel::Info
        } else if formatted.contains("\"level\":\"DEBUG\"") || formatted.contains("[DEBUG]") {
            crate::logging::traits::LogLevel::Debug
        } else if formatted.contains("\"level\":\"TRACE\"") || formatted.contains("[TRACE]") {
            crate::logging::traits::LogLevel::Trace
        } else {
            // Default to INFO if we can't determine the level
            crate::logging::traits::LogLevel::Info
        }
    }

    /// Check if a message should go to stderr
    fn should_use_stderr(&self, formatted: &str) -> bool {
        let level = self.extract_level_from_message(formatted);
        matches!(level, crate::logging::traits::LogLevel::Error | crate::logging::traits::LogLevel::Warn)
    }

    /// Write to appropriate output stream
    fn write_to_appropriate_stream(&self, formatted: &str) -> Result<()> {
        if self.should_use_stderr(formatted) {
            self.stderr_writer.write(formatted)
        } else {
            self.stdout_writer.write(formatted)
        }
    }
}

impl Default for AutoRoutingConsoleWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LogWriter for AutoRoutingConsoleWriter {
    async fn write_async(&self, formatted: &str) -> Result<()> {
        self.write(formatted)
    }

    fn write(&self, formatted: &str) -> Result<()> {
        self.write_to_appropriate_stream(formatted)
    }

    fn flush(&self) -> Result<()> {
        // Flush both streams
        self.stdout_writer.flush()?;
        self.stderr_writer.flush()?;
        Ok(())
    }

    async fn flush_async(&self) -> Result<()> {
        self.flush()
    }

    async fn close_async(&self) -> Result<()> {
        self.flush()?;
        Ok(())
    }
}

/// Enhanced console writer with additional features
#[derive(Debug)]
pub struct EnhancedConsoleWriter {
    base: AutoRoutingConsoleWriter,
    enable_colors: bool,
    timestamp_format: TimestampFormat,
}

/// Timestamp format options
#[derive(Debug, Clone, Copy)]
pub enum TimestampFormat {
    None,
    Short,   // HH:MM:SS
    Medium,  // YYYY-MM-DD HH:MM:SS
    Full,    // ISO 8601
}

impl EnhancedConsoleWriter {
    /// Create a new enhanced console writer
    pub fn new() -> Self {
        Self {
            base: AutoRoutingConsoleWriter::new(),
            enable_colors: true,
            timestamp_format: TimestampFormat::Medium,
        }
    }

    /// Configure color support
    pub fn with_colors(mut self, enable: bool) -> Self {
        self.enable_colors = enable;
        self
    }

    /// Configure timestamp format
    pub fn with_timestamp_format(mut self, format: TimestampFormat) -> Self {
        self.timestamp_format = format;
        self
    }

    /// Check if colors should be enabled (based on terminal support)
    fn should_use_colors(&self) -> bool {
        self.enable_colors && atty::is(atty::Stream::Stdout)
    }

    /// Add timestamp if configured
    fn add_timestamp(&self, message: &str) -> String {
        match self.timestamp_format {
            TimestampFormat::None => message.to_string(),
            TimestampFormat::Short => {
                let now = chrono::Utc::now();
                format!("{} {}", now.format("%H:%M:%S"), message)
            }
            TimestampFormat::Medium => {
                let now = chrono::Utc::now();
                format!("{} {}", now.format("%Y-%m-%d %H:%M:%S"), message)
            }
            TimestampFormat::Full => {
                let now = chrono::Utc::now();
                format!("{} {}", now.to_rfc3339(), message)
            }
        }
    }
}

impl Default for EnhancedConsoleWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LogWriter for EnhancedConsoleWriter {
    async fn write_async(&self, formatted: &str) -> Result<()> {
        self.write(formatted)
    }

    fn write(&self, formatted: &str) -> Result<()> {
        let message = self.add_timestamp(formatted);
        self.base.write(&message)
    }

    fn flush(&self) -> Result<()> {
        self.base.flush()
    }

    async fn flush_async(&self) -> Result<()> {
        self.base.flush_async().await?;
        Ok(())
    }

    async fn close_async(&self) -> Result<()> {
        self.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_writer_creation() {
        let writer = ConsoleWriter::new();
        assert!(!writer.use_stderr);

        let writer = ConsoleWriter::stderr();
        assert!(writer.use_stderr);
    }

    #[test]
    fn test_auto_routing_console_writer() {
        let writer = AutoRoutingConsoleWriter::new();
        
        // Test error message routing
        let error_msg = r#"{"timestamp":"2023-01-01T00:00:00Z","level":"ERROR","message":"Test error"}"#;
        assert!(writer.should_use_stderr(error_msg));

        // Test info message routing
        let info_msg = r#"{"timestamp":"2023-01-01T00:00:00Z","level":"INFO","message":"Test info"}"#;
        assert!(!writer.should_use_stderr(info_msg));

        // Test text format routing
        let error_text = "[ERROR] Test error message";
        assert!(writer.should_use_stderr(error_text));

        let info_text = "[INFO] Test info message";
        assert!(!writer.should_use_stderr(info_text));
    }

    #[test]
    fn test_enhanced_console_writer() {
        let writer = EnhancedConsoleWriter::new();
        
        let message = "Test message";
        let enhanced = writer.add_timestamp(message);
        
        // Should contain timestamp
        assert!(enhanced.len() > message.len());
        assert!(enhanced.contains(message));
    }

    #[test]
    fn test_timestamp_formats() {
        let writer = EnhancedConsoleWriter::new()
            .with_timestamp_format(TimestampFormat::Short);
        
        let message = "Test";
        let result = writer.add_timestamp(message);
        
        // Should contain timestamp-like pattern and message
        assert!(result.len() > message.len() && result.contains("Test"));
    }

    #[tokio::test]
    async fn test_async_write() {
        let writer = ConsoleWriter::new();
        let result = writer.write_async("Test async message").await;
        assert!(result.is_ok());
    }
}
