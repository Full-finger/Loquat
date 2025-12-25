//! Error tracking aspect implementation

use crate::aop::traits::Aspect;
use crate::logging::traits::{Logger, LogLevel, LogContext};
use crate::errors::{AopError, Result};
use async_trait::async_trait;
use std::sync::Arc;

/// Error tracking aspect that automatically tracks and logs errors
pub struct ErrorTrackingAspect {
    logger: Arc<dyn Logger>,
    track_panics: bool,
    collect_stack_traces: bool,
    error_threshold: Option<usize>,
    error_count: std::sync::atomic::AtomicUsize,
}

impl ErrorTrackingAspect {
    /// Create a new error tracking aspect
    pub fn new(logger: Arc<dyn Logger>) -> Self {
        Self {
            logger,
            track_panics: true,
            collect_stack_traces: true,
            error_threshold: None,
            error_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Set whether to track panics
    pub fn with_panics(mut self, track: bool) -> Self {
        self.track_panics = track;
        self
    }

    /// Set whether to collect stack traces
    pub fn with_stack_traces(mut self, collect: bool) -> Self {
        self.collect_stack_traces = collect;
        self
    }

    /// Set error threshold for alerting
    pub fn with_error_threshold(mut self, threshold: usize) -> Self {
        self.error_threshold = Some(threshold);
        self
    }

    /// Get current error count
    pub fn error_count(&self) -> usize {
        self.error_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Reset error count
    pub fn reset_error_count(&self) {
        self.error_count.store(0, std::sync::atomic::Ordering::Relaxed);
    }

    /// Create an error tracker for production
    pub fn production_tracker(logger: Arc<dyn Logger>) -> Self {
        Self {
            logger,
            track_panics: true,
            collect_stack_traces: false, // Don't collect in production for performance
            error_threshold: Some(100), // Alert after 100 errors
            error_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Create an error tracker for development
    pub fn development_tracker(logger: Arc<dyn Logger>) -> Self {
        Self {
            logger,
            track_panics: true,
            collect_stack_traces: true, // Collect in development for debugging
            error_threshold: None, // No threshold in development
            error_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Aspect for ErrorTrackingAspect {
    async fn before(&self, _operation: &str) -> crate::aop::traits::AopResult<()> {
        // No action needed before operation
        Ok(())
    }

    async fn after(&self, operation: &str, result: &crate::aop::traits::AopResult<()>) -> crate::aop::traits::AopResult<()> {
        // Check if the operation failed
        if let Err(error) = result {
            let current_count = self.error_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            
            let mut log_context = LogContext::new();
            log_context.add("operation", operation);
            log_context.add("error_type", std::any::type_name_of_val(error));
            log_context.add("error_message", error.to_string());
            log_context.add("error_count", current_count);

            // Add stack trace if enabled
            if self.collect_stack_traces {
                if let Some(stack_trace) = self.get_stack_trace() {
                    log_context.add("stack_trace", stack_trace);
                }
            }

            // Check if threshold is exceeded
            if let Some(threshold) = self.error_threshold {
                if current_count >= threshold {
                    log_context.add("threshold_exceeded", true);
                    
                    self.logger.log_with_context(
                        LogLevel::Error,
                        &format!("Error threshold exceeded in {}: {}/{}", operation, current_count, threshold),
                        &log_context,
                    );
                } else {
                    self.logger.log_with_context(
                        LogLevel::Error,
                        &format!("Error in operation {}: {}", operation, error),
                        &log_context,
                    );
                }
            } else {
                self.logger.log_with_context(
                    LogLevel::Error,
                    &format!("Error in operation {}: {}", operation, error),
                    &log_context,
                );
            }
        }

        Ok(())
    }

    async fn on_error(&self, operation: &str, error: &AopError) -> crate::aop::traits::AopResult<()> {
        let current_count = self.error_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        
        let mut log_context = LogContext::new();
        log_context.add("operation", operation);
        log_context.add("error_type", std::any::type_name_of_val(error));
        log_context.add("error_message", error.to_string());
        log_context.add("error_count", current_count);

        // Add stack trace if enabled
        if self.collect_stack_traces {
            if let Some(stack_trace) = self.get_stack_trace() {
                log_context.add("stack_trace", stack_trace);
            }
        }

        // Check if threshold is exceeded
        if let Some(threshold) = self.error_threshold {
            if current_count >= threshold {
                log_context.add("threshold_exceeded", true);
                
                self.logger.log_with_context(
                    LogLevel::Error,
                    &format!("Error threshold exceeded in {}: {}/{}", operation, current_count, threshold),
                    &log_context,
                );
            } else {
                self.logger.log_with_context(
                    LogLevel::Error,
                    &format!("AOP error in operation {}: {}", operation, error),
                    &log_context,
                );
            }
        } else {
            self.logger.log_with_context(
                LogLevel::Error,
                &format!("AOP error in operation {}: {}", operation, error),
                &log_context,
            );
        }

        Ok(())
    }
}

impl std::fmt::Debug for ErrorTrackingAspect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ErrorTrackingAspect")
            .field("track_panics", &self.track_panics)
            .field("collect_stack_traces", &self.collect_stack_traces)
            .field("error_threshold", &self.error_threshold)
            .field("error_count", &self.error_count)
            .finish()
    }
}

impl ErrorTrackingAspect {
    /// Get current stack trace
    fn get_stack_trace(&self) -> Option<String> {
        std::backtrace::Backtrace::capture().to_string().into()
    }
}

/// Builder for creating error tracking aspects
pub struct ErrorTrackingAspectBuilder {
    logger: Option<Arc<dyn Logger>>,
    track_panics: bool,
    collect_stack_traces: bool,
    error_threshold: Option<usize>,
}

impl ErrorTrackingAspectBuilder {
    /// Create a new error tracking aspect builder
    pub fn new() -> Self {
        Self {
            logger: None,
            track_panics: true,
            collect_stack_traces: true,
            error_threshold: None,
        }
    }

    /// Set the logger
    pub fn logger(mut self, logger: Arc<dyn Logger>) -> Self {
        self.logger = Some(logger);
        self
    }

    /// Set whether to track panics
    pub fn track_panics(mut self, track: bool) -> Self {
        self.track_panics = track;
        self
    }

    /// Set whether to collect stack traces
    pub fn collect_stack_traces(mut self, collect: bool) -> Self {
        self.collect_stack_traces = collect;
        self
    }

    /// Set error threshold
    pub fn error_threshold(mut self, threshold: usize) -> Self {
        self.error_threshold = Some(threshold);
        self
    }

    /// Build the error tracking aspect
    pub fn build(self) -> Result<ErrorTrackingAspect> {
        let logger = self.logger.ok_or_else(|| {
            crate::errors::LoquatError::Config(crate::errors::ConfigError::MissingRequired(
                "Logger is required for error tracking aspect".to_string()
            ))
        })?;

        Ok(ErrorTrackingAspect {
            logger,
            track_panics: self.track_panics,
            collect_stack_traces: self.collect_stack_traces,
            error_threshold: self.error_threshold,
            error_count: std::sync::atomic::AtomicUsize::new(0),
        })
    }
}

impl Default for ErrorTrackingAspectBuilder {
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
    async fn test_error_tracking_aspect_creation() {
        let writer = Arc::new(ConsoleWriter::new());
        let formatter = Arc::new(TextFormatter::detailed());
        let logger: Arc<dyn Logger> = Arc::new(crate::logging::logger::StructuredLogger::new(formatter, writer));

        let aspect = ErrorTrackingAspect::new(Arc::clone(&logger));
        assert!(aspect.track_panics);
        assert!(aspect.collect_stack_traces);
        assert!(aspect.error_threshold.is_none());
        assert_eq!(aspect.error_count(), 0);
    }

    #[tokio::test]
    async fn test_error_tracking_aspect_builder() {
        let writer = Arc::new(ConsoleWriter::new());
        let formatter = Arc::new(TextFormatter::detailed());
        let logger: Arc<dyn Logger> = Arc::new(crate::logging::logger::StructuredLogger::new(formatter, writer));

        let aspect = ErrorTrackingAspectBuilder::new()
            .logger(logger)
            .track_panics(false)
            .collect_stack_traces(false)
            .error_threshold(50)
            .build()
            .unwrap();

        assert!(!aspect.track_panics);
        assert!(!aspect.collect_stack_traces);
        assert_eq!(aspect.error_threshold, Some(50));
    }

    #[tokio::test]
    async fn test_error_tracking_aspect_builder_no_logger() {
        let result = ErrorTrackingAspectBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_error_count_tracking() {
        let writer = Arc::new(ConsoleWriter::new());
        let formatter = Arc::new(TextFormatter::detailed());
        let logger: Arc<dyn Logger> = Arc::new(crate::logging::logger::StructuredLogger::new(formatter, writer));

        let aspect = ErrorTrackingAspect::new(logger);
        assert_eq!(aspect.error_count(), 0);
        
        aspect.reset_error_count();
        assert_eq!(aspect.error_count(), 0);
    }
}
