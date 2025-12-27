//! Error handling utilities for the Loquat framework
//!
//! Provides functions and helpers for consistent error handling, logging,
//! and recovery strategies.

use crate::errors::{LoquatError, Result};
use crate::logging::traits::{LogContext, LogLevel, Logger};
use std::sync::Arc;

/// Error handling context with retry configuration
#[derive(Debug, Clone)]
pub struct ErrorHandlingConfig {
    /// Whether to log errors
    pub log_errors: bool,
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Delay between retries in milliseconds
    pub retry_delay_ms: u64,
    /// Whether to continue execution after error
    pub continue_on_error: bool,
}

impl Default for ErrorHandlingConfig {
    fn default() -> Self {
        Self {
            log_errors: true,
            max_retries: 3,
            retry_delay_ms: 100,
            continue_on_error: false,
        }
    }
}

/// Log an error and optionally return it
///
/// This function provides a consistent way to log errors before returning them.
///
/// # Arguments
/// * `logger` - The logger to use
/// * `error` - The error to log
/// * `context` - Additional context for the log entry
///
/// # Returns
/// The original error wrapped in `Err`
pub fn log_and_return_error(
    logger: &Arc<dyn Logger>,
    error: LoquatError,
    context: &str,
) -> Result<()> {
    let mut log_context = LogContext::new();
    log_context.component = Some("ErrorHandling".to_string());
    log_context.add("context", context.to_string());
    log_context.add("error", error.to_string());

    logger.log(LogLevel::Error, context, &log_context);
    Err(error)
}

/// Log an error but continue execution
///
/// This function logs an error but returns `Ok(())`, allowing execution to continue.
///
/// # Arguments
/// * `logger` - The logger to use
/// * `error` - The error to log
/// * `context` - Additional context for the log entry
///
/// # Returns
/// `Ok(())` regardless of the error
pub fn log_and_continue(
    logger: &Arc<dyn Logger>,
    error: impl std::fmt::Display,
    context: &str,
) -> Result<()> {
    let mut log_context = LogContext::new();
    log_context.component = Some("ErrorHandling".to_string());
    log_context.add("context", context.to_string());
    log_context.add("error", error.to_string());

    logger.log(LogLevel::Error, context, &log_context);
    Ok(())
}

/// Execute a function with retry logic
///
/// This function will retry the given operation up to `max_retries` times if it fails.
///
/// # Arguments
/// * `operation` - The operation to execute (returns `Result`)
/// * `config` - Retry configuration
/// * `logger` - Optional logger for logging retry attempts
/// * `context` - Context description for logging
///
/// # Returns
/// The result of the operation if it succeeds, or the last error if all retries fail
pub async fn retry_with_backoff<F, Fut, T>(
    mut operation: F,
    config: &ErrorHandlingConfig,
    logger: Option<&Arc<dyn Logger>>,
    context: &str,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut last_error = None;
    let mut attempt = 0;

    loop {
        attempt += 1;
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e.clone());
                
                if attempt >= config.max_retries {
                    if let Some(log) = logger {
                        log_and_continue(
                            log,
                            format!("Operation failed after {} attempts: {}", attempt, e),
                            context,
                        )?;
                    }
                    return Err(e);
                }

                if let Some(log) = logger {
                    let mut log_context = LogContext::new();
                    log_context.component = Some("ErrorHandling".to_string());
                    log_context.add("attempt", attempt.to_string());
                    log_context.add("max_retries", config.max_retries.to_string());
                    log_context.add("context", context.to_string());
                    log_context.add("error", e.to_string());

                    log.log(
                        LogLevel::Warn,
                        &format!("Retry attempt {}/{} for: {}", attempt, config.max_retries, context),
                        &log_context,
                    );
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(config.retry_delay_ms)).await;
            }
        }
    }
}

/// Execute a function with error handling based on configuration
///
/// This function provides a flexible error handling strategy based on the provided config.
///
/// # Arguments
/// * `operation` - The operation to execute (returns `Result`)
/// * `config` - Error handling configuration
/// * `logger` - Logger for logging errors
/// * `context` - Context description for logging
///
/// # Returns
/// `Ok(())` if configured to continue on error, otherwise the error result
pub async fn execute_with_error_handling<F, Fut>(
    operation: F,
    config: &ErrorHandlingConfig,
    logger: &Arc<dyn Logger>,
    context: &str,
) -> Result<()>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    match operation().await {
        Ok(()) => Ok(()),
        Err(e) => {
            if config.continue_on_error {
                log_and_continue(logger, &e, context)
            } else {
                log_and_return_error(logger, e, context)
            }
        }
    }
}

/// Error statistics tracker
#[derive(Debug, Default, Clone)]
pub struct ErrorStats {
    pub total_errors: u64,
    pub retryable_errors: u64,
    pub fatal_errors: u64,
    pub errors_by_type: std::collections::HashMap<String, u64>,
}

impl ErrorStats {
    /// Create a new error stats tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an error
    pub fn record_error(&mut self, error: &LoquatError, retryable: bool) {
        self.total_errors += 1;
        
        if retryable {
            self.retryable_errors += 1;
        } else {
            self.fatal_errors += 1;
        }

        let error_type = std::any::type_name_of_val(error).to_string();
        *self.errors_by_type.entry(error_type).or_insert(0) += 1;
    }

    /// Get the error rate as a percentage
    pub fn error_rate(&self, total_operations: u64) -> f64 {
        if total_operations == 0 {
            0.0
        } else {
            (self.total_errors as f64 / total_operations as f64) * 100.0
        }
    }

    /// Get statistics summary
    pub fn summary(&self) -> String {
        format!(
            "Total Errors: {}, Retryable: {}, Fatal: {}, Types: {}",
            self.total_errors,
            self.retryable_errors,
            self.fatal_errors,
            self.errors_by_type.len()
        )
    }

    /// Reset all statistics
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_logger() -> Arc<dyn crate::logging::Logger> {
        let formatter = Arc::new(crate::logging::formatters::JsonFormatter::new());
        let writer = Arc::new(crate::logging::writers::ConsoleWriter::new());
        Arc::new(crate::logging::StructuredLogger::new(formatter, writer))
    }

    #[test]
    fn test_error_handling_config_default() {
        let config = ErrorHandlingConfig::default();
        assert!(config.log_errors);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay_ms, 100);
        assert!(!config.continue_on_error);
    }

    #[test]
    fn test_error_stats_record() {
        let mut stats = ErrorStats::new();
        
        let error = LoquatError::Unknown("Test error".to_string());
        stats.record_error(&error, true);
        
        assert_eq!(stats.total_errors, 1);
        assert_eq!(stats.retryable_errors, 1);
        assert_eq!(stats.fatal_errors, 0);
    }

    #[test]
    fn test_error_stats_rate() {
        let mut stats = ErrorStats::new();
        
        let error = LoquatError::Unknown("Test error".to_string());
        stats.record_error(&error, false);
        stats.record_error(&error, false);
        stats.record_error(&error, false);
        
        let rate = stats.error_rate(10);
        assert_eq!(rate, 30.0);
    }

    #[test]
    fn test_error_stats_summary() {
        let mut stats = ErrorStats::new();
        
        let error = LoquatError::Unknown("Test error".to_string());
        stats.record_error(&error, true);
        stats.record_error(&error, false);
        
        let summary = stats.summary();
        assert!(summary.contains("Total Errors: 2"));
        assert!(summary.contains("Retryable: 1"));
        assert!(summary.contains("Fatal: 1"));
    }

    #[test]
    fn test_error_stats_reset() {
        let mut stats = ErrorStats::new();
        
        let error = LoquatError::Unknown("Test error".to_string());
        stats.record_error(&error, true);
        assert_eq!(stats.total_errors, 1);
        
        stats.reset();
        assert_eq!(stats.total_errors, 0);
        assert_eq!(stats.retryable_errors, 0);
        assert_eq!(stats.fatal_errors, 0);
    }

    #[tokio::test]
    async fn test_log_and_continue() {
        let logger = create_test_logger();
        let error = LoquatError::Unknown("Test error".to_string());
        
        let result = log_and_continue(&logger, &error, "Test context");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_retry_with_backoff_success() {
        let logger = create_test_logger();
        let config = ErrorHandlingConfig {
            max_retries: 3,
            retry_delay_ms: 10,
            ..Default::default()
        };
        
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let result = retry_with_backoff(
            || {
                let attempts = attempts.clone();
                async move {
                    let count = attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    if count < 3 {
                        Err(LoquatError::Unknown("Temporary error".to_string()))
                    } else {
                        Ok(42)
                    }
                }
            },
            &config,
            Some(&logger),
            "Test operation",
        ).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_with_backoff_failure() {
        let logger = create_test_logger();
        let config = ErrorHandlingConfig {
            max_retries: 2,
            retry_delay_ms: 10,
            ..Default::default()
        };
        
        let result: Result<i32> = retry_with_backoff(
            || async {
                Err(LoquatError::Unknown("Persistent error".to_string()))
            },
            &config,
            Some(&logger),
            "Test operation",
        ).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_with_error_handling_continue() {
        let logger = create_test_logger();
        let config = ErrorHandlingConfig {
            continue_on_error: true,
            ..Default::default()
        };
        
        let result = execute_with_error_handling(
            || async {
                Err(LoquatError::Unknown("Test error".to_string()))
            },
            &config,
            &logger,
            "Test context",
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_error_handling_fail() {
        let logger = create_test_logger();
        let config = ErrorHandlingConfig {
            continue_on_error: false,
            ..Default::default()
        };
        
        let result = execute_with_error_handling(
            || async {
                Err(LoquatError::Unknown("Test error".to_string()))
            },
            &config,
            &logger,
            "Test context",
        ).await;
        
        assert!(result.is_err());
    }
}
