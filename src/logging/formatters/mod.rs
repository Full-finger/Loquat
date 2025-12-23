//! Log formatter implementations

pub mod json;
pub mod text;

pub use json::*;
pub use text::*;

use crate::logging::traits::{LogEntry, LogFormatter};
use std::sync::Arc;

/// Create a formatter based on configuration
pub fn create_formatter(format_type: &str) -> Arc<dyn LogFormatter> {
    match format_type.to_lowercase().as_str() {
        "json" => Arc::new(JsonFormatter::new()),
        "text" => Arc::new(TextFormatter::new()),
        "compact" => Arc::new(TextFormatter::compact()),
        _ => Arc::new(JsonFormatter::new()), // Default to JSON
    }
}
