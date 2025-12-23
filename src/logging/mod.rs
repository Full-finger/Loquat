//! Logging module for Loquat framework
//!
//! Provides a flexible, structured logging system with multiple output formats
//! and destinations, following clean architecture principles.

pub mod formatters;
pub mod logger;
pub mod traits;
pub mod writers;

pub use formatters::*;
pub use logger::*;
pub use traits::*;
pub use writers::*;

use crate::errors::Result;
use std::sync::Arc;
use std::sync::OnceLock;

static GLOBAL_LOGGER: OnceLock<Arc<dyn Logger>> = OnceLock::new();

/// Convenience function to initialize the default logging system
pub fn init_default_logger() -> Result<Arc<dyn Logger>> {
    use crate::logging::formatters::JsonFormatter;
    use crate::logging::writers::ConsoleWriter;
    
    let formatter = Arc::new(JsonFormatter::new());
    let writer = Arc::new(ConsoleWriter::new());
    let logger = Arc::new(StructuredLogger::new(formatter, writer));
    
    logger.set_level(LogLevel::Info);
    logger.init()?;
    
    Ok(logger)
}

/// Initialize logging with custom configuration
pub fn init_with_config(
    formatter: Arc<dyn LogFormatter>,
    writer: Arc<dyn LogWriter>,
    level: LogLevel,
) -> Result<Arc<dyn Logger>> {
    let logger = Arc::new(StructuredLogger::new(formatter, writer));
    logger.set_level(level);
    logger.init()?;
    Ok(logger)
}

/// Macro for convenient logging
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logging::get_logger().log(
            $crate::logging::LogLevel::Info,
            &format!($($arg)*),
            &$crate::logging::LogContext::current(),
        );
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logging::get_logger().log(
            $crate::logging::LogLevel::Error,
            &format!($($arg)*),
            &$crate::logging::LogContext::current(),
        );
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::logging::get_logger().log(
            $crate::logging::LogLevel::Debug,
            &format!($($arg)*),
            &$crate::logging::LogContext::current(),
        );
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logging::get_logger().log(
            $crate::logging::LogLevel::Warn,
            &format!($($arg)*),
            &$crate::logging::LogContext::current(),
        );
    };
}

#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        $crate::logging::get_logger().log(
            $crate::logging::LogLevel::Trace,
            &format!($($arg)*),
            &$crate::logging::LogContext::current(),
        );
    };
}

/// Set of global logger instance
pub fn set_global_logger(logger: Arc<dyn Logger>) {
    let _ = GLOBAL_LOGGER.set(logger);
}

/// Get the global logger instance
pub fn get_logger() -> Arc<dyn Logger> {
    GLOBAL_LOGGER
        .get()
        .expect("Logger not initialized. Call init_default_logger() or init_with_config() first.")
        .clone()
}
