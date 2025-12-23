//! Pool validator for detecting dead loops and other issues

use crate::events::Package;
use crate::logging::traits::Logger;

/// Pool validator - checks for dead loops and validation issues
#[derive(Debug, Clone)]
pub struct PoolValidator {
    /// Maximum iterations before detecting potential dead loop
    max_iterations: usize,
}

impl PoolValidator {
    /// Create a new pool validator
    pub fn new() -> Self {
        Self {
            max_iterations: 100,
        }
    }
    
    /// Create a new pool validator with custom max iterations
    pub fn with_max_iterations(max_iterations: usize) -> Self {
        Self { max_iterations }
    }
    
    /// Log a dead loop warning
    pub fn log_dead_loop_warning(&self, logger: &dyn Logger, worker_name: &str, package: &Package) {
        use crate::logging::{LogLevel, LogContext};
        logger.log(LogLevel::Warn, &format!(
            "Potential dead loop detected: Worker '{}' produced output that would match itself. Package ID: {}",
            worker_name, package.package_id
        ), &LogContext::new());
    }
    
    /// Check if package has exceeded processing iterations
    pub fn check_iterations(&self, current_iterations: usize) -> bool {
        current_iterations < self.max_iterations
    }
    
    /// Get max iterations
    pub fn max_iterations(&self) -> usize {
        self.max_iterations
    }
}

impl Default for PoolValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_validator_creation() {
        let validator = PoolValidator::new();
        assert_eq!(validator.max_iterations(), 100);
    }

    #[test]
    fn test_validator_with_max_iterations() {
        let validator = PoolValidator::with_max_iterations(50);
        assert_eq!(validator.max_iterations(), 50);
    }

    #[test]
    fn test_check_iterations() {
        let validator = PoolValidator::new();
        assert!(validator.check_iterations(99));
        assert!(!validator.check_iterations(100));
    }

    #[test]
    fn test_log_dead_loop_warning() {
        use crate::logging::formatters::JsonFormatter;
        use crate::logging::writers::ConsoleWriter;
        let formatter = Arc::new(JsonFormatter::new());
        let writer = Arc::new(ConsoleWriter::new());
        let logger = Arc::new(crate::logging::StructuredLogger::new(formatter, writer));
        let validator = PoolValidator::new();
        let package = Package::new();
        
        // This should log a warning
        validator.log_dead_loop_warning(logger.as_ref(), "test_worker", &package);
    }
}
