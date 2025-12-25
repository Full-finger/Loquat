//! Logger implementation

use crate::errors::{LoggingError, Result};
use crate::logging::traits::{LogEntry, LogLevel, LogContext, LogFormatter, LogWriter, Logger};
use std::sync::Arc;

/// Structured logger implementation
pub struct StructuredLogger {
    formatter: Arc<dyn LogFormatter>,
    writer: Arc<dyn LogWriter>,
    min_level: std::sync::RwLock<LogLevel>,
    initialized: std::sync::atomic::AtomicBool,
}

impl StructuredLogger {
    /// Create a new structured logger
    pub fn new(formatter: Arc<dyn LogFormatter>, writer: Arc<dyn LogWriter>) -> Self {
        Self {
            formatter,
            writer,
            min_level: std::sync::RwLock::new(LogLevel::Info),
            initialized: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Create logger with custom initial level
    pub fn with_level(
        formatter: Arc<dyn LogFormatter>,
        writer: Arc<dyn LogWriter>,
        level: LogLevel,
    ) -> Self {
        Self {
            formatter,
            writer,
            min_level: std::sync::RwLock::new(level),
            initialized: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Initialize logger (marks as ready for use)
    pub fn initialize(&self) {
        self.initialized.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Check if logger is initialized
    fn is_initialized(&self) -> bool {
        self.initialized.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Ensure logger is initialized before use
    fn ensure_initialized(&self) -> Result<()> {
        if !self.is_initialized() {
            return Err(LoggingError::Initialization(
                "Logger not initialized. Call init() or initialize() first.".to_string()
            ).into());
        }
        Ok(())
    }

    /// Create a log entry with location information
    fn create_log_entry(
        &self,
        level: LogLevel,
        message: &str,
        context: &LogContext,
        module_path: Option<&str>,
        file: Option<&str>,
        line: Option<u32>,
    ) -> LogEntry {
        let mut entry = LogEntry::new(level, message.to_string(), context.clone());

        if let (Some(module), Some(src_file), Some(line_num)) = (module_path, file, line) {
            entry = entry.with_location(module, src_file, line_num);
        }

        entry
    }

    /// Internal log method
    fn log_internal(
        &self,
        level: LogLevel,
        message: &str,
        context: &LogContext,
        module_path: Option<&str>,
        file: Option<&str>,
        line: Option<u32>,
    ) -> Result<()> {
        self.ensure_initialized()?;

        // Check log level
        let min_level = *self.min_level.read().unwrap();
        if !level.should_log(min_level) {
            return Ok(());
        }

        // Create log entry
        let entry = self.create_log_entry(level, message, context, module_path, file, line);

        // Format and write
        let formatted = self.formatter.format(&entry);
        self.writer.write(&formatted)?;

        Ok(())
    }

    /// Log with location information (from macros)
    pub fn log_with_location(
        &self,
        level: LogLevel,
        message: &str,
        context: &LogContext,
        module_path: &str,
        file: &str,
        line: u32,
    ) -> Result<()> {
        self.log_internal(level, message, context, Some(module_path), Some(file), Some(line))
    }

    /// Batch log multiple entries
    pub fn log_batch(&self, entries: &[LogEntry]) -> Result<()> {
        self.ensure_initialized()?;

        if entries.is_empty() {
            return Ok(());
        }

        // Filter entries by level
        let min_level = *self.min_level.read().unwrap();
        let filtered_entries: Vec<LogEntry> = entries
            .iter()
            .filter(|entry| entry.level.should_log(min_level))
            .cloned()
            .collect();

        if filtered_entries.is_empty() {
            return Ok(());
        }

        // Format batch
        let formatted_entries = self.formatter.format_batch(&filtered_entries);

        // Write batch if supported, otherwise write individually
        for formatted in formatted_entries {
            self.writer.write(&formatted)?;
        }

        Ok(())
    }

    /// Get current log level
    pub fn current_level(&self) -> LogLevel {
        *self.min_level.read().unwrap()
    }

    /// Set log level atomically
    pub fn set_level_atomic(&self, level: LogLevel) {
        let mut min_level = self.min_level.write().unwrap();
        *min_level = level;
    }

    /// Check if a level is enabled
    pub fn is_level_enabled(&self, level: LogLevel) -> bool {
        let min_level = *self.min_level.read().unwrap();
        level.should_log(min_level)
    }
}

impl Logger for StructuredLogger {
    fn log(&self, level: LogLevel, message: &str, context: &LogContext) {
        // Log silently on error to avoid infinite recursion
        let _ = self.log_internal(level, message, context, None, None, None);
    }

    fn log_entry(&self, entry: &LogEntry) {
        // Log silently on error to avoid infinite recursion
        let _ = self.log_batch(&[entry.clone()]);
    }

    fn set_level(&self, level: LogLevel) {
        self.set_level_atomic(level);
    }

    fn get_level(&self) -> LogLevel {
        self.current_level()
    }

    fn is_enabled(&self, level: LogLevel) -> bool {
        self.is_level_enabled(level)
    }

    fn init(&self) -> Result<()> {
        self.initialize();
        Ok(())
    }

    fn flush(&self) -> Result<()> {
        self.writer.flush().map_err(|e| e.into())
    }
}

/// Logger builder for convenient configuration
pub struct LoggerBuilder {
    formatter: Option<Arc<dyn LogFormatter>>,
    writer: Option<Arc<dyn LogWriter>>,
    level: LogLevel,
}

impl LoggerBuilder {
    /// Create a new logger builder
    pub fn new() -> Self {
        Self {
            formatter: None,
            writer: None,
            level: LogLevel::Info,
        }
    }

    /// Set's formatter
    pub fn formatter(mut self, formatter: Arc<dyn LogFormatter>) -> Self {
        self.formatter = Some(formatter);
        self
    }

    /// Set's writer
    pub fn writer(mut self, writer: Arc<dyn LogWriter>) -> Self {
        self.writer = Some(writer);
        self
    }

    /// Set log level
    pub fn level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }

    /// Build logger
    pub fn build(self) -> Result<Arc<dyn Logger>> {
        let formatter = self.formatter
            .ok_or_else(|| LoggingError::Configuration("No formatter specified".to_string()))?;
        let writer = self.writer
            .ok_or_else(|| LoggingError::Configuration("No writer specified".to_string()))?;

        let logger = Arc::new(StructuredLogger::with_level(formatter, writer, self.level));
        logger.init()?;
        Ok(logger)
    }
}

impl Default for LoggerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience functions for creating common logger configurations
pub struct LoggerFactory;

impl LoggerFactory {
    /// Create a console logger with JSON formatting
    pub async fn console_json() -> Result<Arc<dyn Logger>> {
        use crate::logging::formatters::JsonFormatter;
        use crate::logging::writers::ConsoleWriter;

        let formatter = Arc::new(JsonFormatter::new());
        let writer: Arc<dyn LogWriter> = Arc::new(ConsoleWriter::auto_routing());
        let logger = Arc::new(StructuredLogger::new(formatter, writer));
        logger.init()?;
        Ok(logger)
    }

    /// Create a console logger with text formatting
    pub async fn console_text() -> Result<Arc<dyn Logger>> {
        use crate::logging::formatters::TextFormatter;
        use crate::logging::writers::ConsoleWriter;

        let formatter = Arc::new(TextFormatter::detailed());
        let writer: Arc<dyn LogWriter> = Arc::new(ConsoleWriter::auto_routing());
        let logger = Arc::new(StructuredLogger::new(formatter, writer));
        logger.init()?;
        Ok(logger)
    }

    /// Create a file logger with JSON formatting
    pub async fn file_json<P: AsRef<std::path::Path>>(path: P) -> Result<Arc<dyn Logger>> {
        use crate::logging::formatters::JsonFormatter;
        use crate::logging::writers::FileWriter;

        let formatter = Arc::new(JsonFormatter::new());
        let file_writer = FileWriter::with_rotation(path).await
            .map_err(|e| LoggingError::Configuration(e.to_string()))?;
        let writer: Arc<dyn LogWriter> = Arc::new(file_writer);
        let logger = Arc::new(StructuredLogger::new(formatter, writer));
        logger.init()?;
        Ok(logger)
    }

    /// Create a file logger with text formatting
    pub async fn file_text<P: AsRef<std::path::Path>>(path: P) -> Result<Arc<dyn Logger>> {
        use crate::logging::formatters::TextFormatter;
        use crate::logging::writers::FileWriter;

        let formatter = Arc::new(TextFormatter::detailed());
        let file_writer = FileWriter::with_rotation(path).await
            .map_err(|e| LoggingError::Configuration(e.to_string()))?;
        let writer: Arc<dyn LogWriter> = Arc::new(file_writer);
        let logger = Arc::new(StructuredLogger::new(formatter, writer));
        logger.init()?;
        Ok(logger)
    }

    /// Create a combined logger (console + file) with JSON formatting
    pub async fn combined_json<P: AsRef<std::path::Path>>(path: P) -> Result<Arc<dyn Logger>> {
        use crate::logging::formatters::JsonFormatter;
        use crate::logging::writers::{ConsoleWriter, FileWriter, CombinedWriter};

        let formatter = Arc::new(JsonFormatter::new());
        let console_writer: Arc<dyn LogWriter> = Arc::new(ConsoleWriter::auto_routing());
        let file_writer = FileWriter::with_rotation(path).await
            .map_err(|e| LoggingError::Configuration(e.to_string()))?;
        let file_writer: Arc<dyn LogWriter> = Arc::new(file_writer);
        let combined_writer: Arc<dyn LogWriter> = Arc::new(CombinedWriter::new(vec![console_writer, file_writer]));
        let logger = Arc::new(StructuredLogger::new(formatter, combined_writer));
        logger.init()?;
        Ok(logger)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::formatters::JsonFormatter;
    use crate::logging::writers::ConsoleWriter;
    use tempfile::tempdir;

    #[test]
    fn test_structured_logger_creation() {
        let formatter = Arc::new(JsonFormatter::new());
        let writer = Arc::new(ConsoleWriter::new());
        let logger = StructuredLogger::new(formatter, writer);

        assert!(!logger.is_initialized());
        assert_eq!(logger.current_level(), LogLevel::Info);
    }

    #[test]
    fn test_logger_level_management() {
        let formatter = Arc::new(JsonFormatter::new());
        let writer = Arc::new(ConsoleWriter::new());
        let logger = StructuredLogger::new(formatter, writer);

        logger.set_level(LogLevel::Debug);
        assert_eq!(logger.get_level(), LogLevel::Debug);
        assert!(logger.is_enabled(LogLevel::Debug));
        assert!(!logger.is_enabled(LogLevel::Trace));
    }

    #[test]
    fn test_logger_builder() {
        let formatter = Arc::new(JsonFormatter::new());
        let writer = Arc::new(ConsoleWriter::new());

        let logger = LoggerBuilder::new()
            .formatter(formatter)
            .writer(writer)
            .level(LogLevel::Debug)
            .build();

        assert!(logger.is_ok());
    }

    #[tokio::test]
    async fn test_logger_factory_console() {
        let logger = LoggerFactory::console_json().await;
        assert!(logger.is_ok());
    }

    #[tokio::test]
    async fn test_logger_factory_file() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.log");

        let logger = LoggerFactory::file_json(&file_path).await;
        assert!(logger.is_ok());
        assert!(file_path.exists());
    }

    #[test]
    fn test_log_with_location() {
        let formatter = Arc::new(JsonFormatter::new());
        let writer = Arc::new(ConsoleWriter::new());
        let logger = StructuredLogger::new(formatter, writer);
        logger.init().unwrap();

        let context = LogContext::new();
        let result = logger.log_with_location(
            LogLevel::Info,
            "Test message",
            &context,
            "test_module",
            "test_file.rs",
            42,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_batch_logging() {
        let formatter = Arc::new(JsonFormatter::new());
        let writer = Arc::new(ConsoleWriter::new());
        let logger = StructuredLogger::new(formatter, writer);
        logger.init().unwrap();

        let entries = vec![
            LogEntry::new(LogLevel::Info, "Message 1".to_string(), LogContext::new()),
            LogEntry::new(LogLevel::Debug, "Message 2".to_string(), LogContext::new()),
        ];

        let result = logger.log_batch(&entries);
        assert!(result.is_ok());
    }
}
