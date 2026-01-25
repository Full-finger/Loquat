//! Integration test helpers

use loquat::config::LoquatConfig;
use loquat::engine::StandardEngine;
use loquat::logging::formatters::TextFormatter;
use loquat::logging::writers::ConsoleWriter;
use loquat::logging::traits::Logger;
use loquat::logging::StructuredLogger;

/// Create a test engine with minimal configuration
pub async fn create_test_engine() -> StandardEngine {
    let logger = create_test_logger();
    StandardEngine::new(logger)
}

/// Create a test logger
pub fn create_test_logger() -> std::sync::Arc<dyn Logger> {
    let formatter = std::sync::Arc::new(TextFormatter::simple());
    let writer = std::sync::Arc::new(ConsoleWriter::new());
    std::sync::Arc::new(StructuredLogger::new(formatter, writer))
}

/// Create a test configuration
pub fn create_test_config() -> LoquatConfig {
    LoquatConfig::default()
}

/// Wait for async operation with timeout
pub async fn wait_with_timeout<F>(future: F, duration: std::time::Duration) -> F::Output
where
    F: std::future::Future + std::panic::UnwindSafe,
{
    tokio::time::timeout(duration, future)
        .await
        .expect("Operation timed out")
}

/// Create a test directory for temporary files
pub fn create_test_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

/// Create a test config file in a temporary directory
pub fn create_test_config_file(dir: &std::path::Path, config: &str) -> std::path::PathBuf {
    let config_path = dir.join("test.toml");
    std::fs::write(&config_path, config).expect("Failed to write config file");
    config_path
}
