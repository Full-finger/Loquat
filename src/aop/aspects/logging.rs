//! Logging aspect implementation

use crate::aop::traits::{Aspect, AspectContext, AspectResult};
use crate::logging::traits::{Logger, LogLevel, LogContext};
use crate::errors::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// Logging aspect that automatically logs method calls
pub struct LoggingAspect {
    logger: Arc<dyn Logger>,
    log_level: LogLevel,
    include_args: bool,
    include_result: bool,
}

impl LoggingAspect {
    /// Create a new logging aspect
    pub fn new(logger: Arc<dyn Logger>) -> Self {
        Self {
            logger,
            log_level: LogLevel::Info,
            include_args: false,
            include_result: false,
        }
    }

    /// Set the log level for this aspect
    pub fn with_log_level(mut self, level: LogLevel) -> Self {
        self.log_level = level;
        self
    }

    /// Set whether to include method arguments in logs
    pub fn with_args(mut self, include: bool) -> Self {
        self.include_args = include;
        self
    }

    /// Set whether to include method results in logs
    pub fn with_result(mut self, include: bool) -> Self {
        self.include_result = include;
        self
    }

    /// Create a logger with default logging aspect
    pub fn default_logger(logger: Arc<dyn Logger>) -> Self {
        Self {
            logger,
            log_level: LogLevel::Info,
            include_args: false,
            include_result: false,
        }
    }

    /// Create a logger with detailed logging
    pub fn detailed_logger(logger: Arc<dyn Logger>) -> Self {
        Self {
            logger,
            log_level: LogLevel::Debug,
            include_args: true,
            include_result: true,
        }
    }

    /// Create a request logger for web requests
    pub fn request_logger(logger: Arc<dyn Logger>) -> Self {
        Self {
            logger,
            log_level: LogLevel::Info,
            include_args: true,
            include_result: false,
        }
    }

    /// Create a function logger for function calls
    pub fn function_logger(logger: Arc<dyn Logger>) -> Self {
        Self {
            logger,
            log_level: LogLevel::Debug,
            include_args: false,
            include_result: false,
        }
    }
}

#[async_trait]
impl Aspect for LoggingAspect {
    async fn before(&self, operation: &str) -> crate::errors::Result<()> {
        let log_context = LogContext::new()
            .with_metadata("operation", operation);

        self.logger.log_with_context(
            self.log_level,
            &format!("Calling {}", operation),
            &log_context,
        );

        Ok(())
    }

    async fn after(&self, operation: &str, _result: &crate::errors::Result<()>) -> crate::errors::Result<()> {
        let log_context = LogContext::new()
            .with_metadata("operation", operation);

        self.logger.log_with_context(
            self.log_level,
            &format!("Completed {}", operation),
            &log_context,
        );

        Ok(())
    }

    async fn on_error(&self, operation: &str, error: &crate::errors::AopError) -> crate::errors::Result<()> {
        let log_context = LogContext::new()
            .with_metadata("operation", operation)
            .with_metadata("error_type", std::any::type_name_of_val(error))
            .with_metadata("error_message", error.to_string());

        self.logger.log_with_context(
            LogLevel::Error,
            &format!("Error in {}", operation),
            &log_context,
        );

        Ok(())
    }
}

/// Builder for creating logging aspects
pub struct LoggingAspectBuilder {
    logger: Option<Arc<dyn Logger>>,
    log_level: LogLevel,
    include_args: bool,
    include_result: bool,
}

impl LoggingAspectBuilder {
    /// Create a new logging aspect builder
    pub fn new() -> Self {
        Self {
            logger: None,
            log_level: LogLevel::Info,
            include_args: false,
            include_result: false,
        }
    }

    /// Set the logger
    pub fn logger(mut self, logger: Arc<dyn Logger>) -> Self {
        self.logger = Some(logger);
        self
    }

    /// Set the log level
    pub fn log_level(mut self, level: LogLevel) -> Self {
        self.log_level = level;
        self
    }

    /// Set whether to include arguments
    pub fn include_args(mut self, include: bool) -> Self {
        self.include_args = include;
        self
    }

    /// Set whether to include results
    pub fn include_result(mut self, include: bool) -> Self {
        self.include_result = include;
        self
    }

    /// Build the logging aspect
    pub fn build(self) -> Result<LoggingAspect> {
        let logger = self.logger.ok_or_else(|| {
            crate::errors::LoquatError::Config(crate::errors::ConfigError::MissingRequired(
                "Logger is required for logging aspect".to_string()
            ))
        })?;

        Ok(LoggingAspect {
            logger,
            log_level: self.log_level,
            include_args: self.include_args,
            include_result: self.include_result,
        })
    }
}

impl std::fmt::Debug for LoggingAspect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoggingAspect")
            .field("log_level", &self.log_level)
            .field("include_args", &self.include_args)
            .field("include_result", &self.include_result)
            .finish()
    }
}

impl Default for LoggingAspectBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::writers::ConsoleWriter;
    use crate::logging::formatters::TextFormatter;

    #[tokio::test]
    async fn test_logging_aspect_creation() {
        let writer = Arc::new(ConsoleWriter::new());
        let formatter = Arc::new(TextFormatter::detailed());
        let logger: Arc<dyn Logger> = Arc::new(crate::logging::logger::StructuredLogger::new(formatter, writer));

        let aspect = LoggingAspect::new(Arc::clone(&logger));
        assert_eq!(aspect.log_level, LogLevel::Info);
        assert!(!aspect.include_args);
        assert!(!aspect.include_result);
    }

    #[tokio::test]
    async fn test_logging_aspect_builder() {
        let writer = Arc::new(ConsoleWriter::new());
        let formatter = Arc::new(TextFormatter::detailed());
        let logger: Arc<dyn Logger> = Arc::new(crate::logging::logger::StructuredLogger::new(formatter, writer));

        let aspect = LoggingAspectBuilder::new()
            .logger(logger)
            .log_level(LogLevel::Debug)
            .include_args(true)
            .include_result(true)
            .build()
            .unwrap();

        assert_eq!(aspect.log_level, LogLevel::Debug);
        assert!(aspect.include_args);
        assert!(aspect.include_result);
    }

    #[tokio::test]
    async fn test_logging_aspect_builder_no_logger() {
        let result = LoggingAspectBuilder::new().build();
        assert!(result.is_err());
    }
}
