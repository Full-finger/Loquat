//! JSON log formatter implementation

use crate::logging::traits::{LogEntry, LogFormatter};
use serde_json::{json, Value};

/// JSON formatter for structured logging
#[derive(Debug, Clone)]
pub struct JsonFormatter {
    pretty_print: bool,
    include_metadata: bool,
}

impl JsonFormatter {
    /// Create a new JSON formatter
    pub fn new() -> Self {
        Self {
            pretty_print: false,
            include_metadata: true,
        }
    }

    /// Create a pretty-printed JSON formatter
    pub fn pretty() -> Self {
        Self {
            pretty_print: true,
            include_metadata: true,
        }
    }

    /// Create a compact JSON formatter (no metadata)
    pub fn compact() -> Self {
        Self {
            pretty_print: false,
            include_metadata: false,
        }
    }

    /// Configure whether to include metadata
    pub fn with_metadata(mut self, include: bool) -> Self {
        self.include_metadata = include;
        self
    }

    /// Configure pretty printing
    pub fn with_pretty_print(mut self, pretty: bool) -> Self {
        self.pretty_print = pretty;
        self
    }

    /// Convert log entry to JSON object
    fn entry_to_json(&self, entry: &LogEntry) -> Value {
        let mut json_obj = json!({
            "timestamp": entry.timestamp.to_rfc3339(),
            "level": entry.level.as_str(),
            "message": entry.message,
        });

        // Add context information
        if let Some(correlation_id) = &entry.context.correlation_id {
            json_obj["correlation_id"] = json!(correlation_id);
        }

        if let Some(user_id) = &entry.context.user_id {
            json_obj["user_id"] = json!(user_id);
        }

        if let Some(session_id) = &entry.context.session_id {
            json_obj["session_id"] = json!(session_id);
        }

        if let Some(component) = &entry.context.component {
            json_obj["component"] = json!(component);
        }

        // Add request info
        if let Some(request_info) = &entry.context.request_info {
            json_obj["request"] = json!({
                "method": request_info.method,
                "path": request_info.path,
                "user_agent": request_info.user_agent.as_ref().unwrap_or(&String::new()),
                "remote_addr": request_info.remote_addr.as_ref().unwrap_or(&String::new()),
            });
        }

        // Add metadata if enabled
        if self.include_metadata && !entry.context.metadata.is_empty() {
            json_obj["metadata"] = json!(entry.context.metadata);
        }

        // Add source location if available
        if let Some(module_path) = &entry.module_path {
            json_obj["module"] = json!(module_path);
        }

        if let Some(file) = &entry.file {
            json_obj["file"] = json!(file);
        }

        if let Some(line) = entry.line {
            json_obj["line"] = json!(line);
        }

        json_obj
    }

    /// Convert JSON to string with configured formatting
    fn json_to_string(&self, json: Value) -> String {
        if self.pretty_print {
            serde_json::to_string_pretty(&json).unwrap_or_else(|_| json.to_string())
        } else {
            json.to_string()
        }
    }
}

impl Default for JsonFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl LogFormatter for JsonFormatter {
    fn format(&self, entry: &LogEntry) -> String {
        let json = self.entry_to_json(entry);
        self.json_to_string(json)
    }

    fn format_batch(&self, entries: &[LogEntry]) -> Vec<String> {
        entries
            .iter()
            .map(|entry| {
                let json = self.entry_to_json(entry);
                self.json_to_string(json)
            })
            .collect()
    }
}

/// Enhanced JSON formatter with additional features
#[derive(Debug, Clone)]
pub struct EnhancedJsonFormatter {
    base: JsonFormatter,
    include_performance_metrics: bool,
    include_error_details: bool,
}

impl EnhancedJsonFormatter {
    /// Create a new enhanced JSON formatter
    pub fn new() -> Self {
        Self {
            base: JsonFormatter::new(),
            include_performance_metrics: true,
            include_error_details: true,
        }
    }

    /// Configure performance metrics inclusion
    pub fn with_performance_metrics(mut self, include: bool) -> Self {
        self.include_performance_metrics = include;
        self
    }

    /// Configure error details inclusion
    pub fn with_error_details(mut self, include: bool) -> Self {
        self.include_error_details = include;
        self
    }

    /// Add performance metrics to JSON
    fn add_performance_metrics(&self, entry: &LogEntry, mut json: Value) -> Value {
        if self.include_performance_metrics {
            // In a real implementation, these would be actual metrics
            json["performance"] = json!({
                "timestamp_ms": entry.timestamp.timestamp_millis(),
                "level_priority": entry.level as u8,
            });
        }
        json
    }

    /// Add error details for error level logs
    fn add_error_details(&self, entry: &LogEntry, mut json: Value) -> Value {
        if self.include_error_details && entry.level == crate::logging::traits::LogLevel::Error {
            json["error"] = json!({
                "severity": "high",
                "type": "application_error",
                "requires_action": true,
            });
        }
        json
    }
}

impl Default for EnhancedJsonFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl LogFormatter for EnhancedJsonFormatter {
    fn format(&self, entry: &LogEntry) -> String {
        let mut json = self.base.entry_to_json(entry);
        
        json = self.add_performance_metrics(entry, json);
        json = self.add_error_details(entry, json);

        if self.base.pretty_print {
            serde_json::to_string_pretty(&json).unwrap_or_else(|_| json.to_string())
        } else {
            json.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::traits::{LogContext, LogLevel};

    #[test]
    fn test_json_formatter_basic() {
        let formatter = JsonFormatter::new();
        let context = LogContext::new().with_component("test");
        let entry = LogEntry::new(
            LogLevel::Info,
            "Test message".to_string(),
            context,
        );

        let formatted = formatter.format(&entry);
        
        // Verify it's valid JSON
        let parsed: Value = serde_json::from_str(&formatted).unwrap();
        assert_eq!(parsed["message"], "Test message");
        assert_eq!(parsed["level"], "INFO");
        assert_eq!(parsed["component"], "test");
    }

    #[test]
    fn test_json_formatter_with_request_info() {
        let formatter = JsonFormatter::new();
        let request_info = crate::logging::traits::RequestInfo::new("GET", "/api/test");
        let context = LogContext::new()
            .with_component("api")
            .with_correlation_id("req-123".to_string());
        let entry = LogEntry::new(
            LogLevel::Info,
            "Request processed".to_string(),
            context,
        );

        let formatted = formatter.format(&entry);
        
        let parsed: Value = serde_json::from_str(&formatted).unwrap();
        assert_eq!(parsed["correlation_id"], "req-123");
        assert_eq!(parsed["component"], "api");
    }

    #[test]
    fn test_enhanced_json_formatter() {
        let formatter = EnhancedJsonFormatter::new();
        let context = LogContext::new();
        let entry = LogEntry::new(
            LogLevel::Error,
            "Error occurred".to_string(),
            context,
        );

        let formatted = formatter.format(&entry);
        
        let parsed: Value = serde_json::from_str(&formatted).unwrap();
        assert!(parsed.get("performance").is_some());
        assert!(parsed.get("error").is_some());
        assert_eq!(parsed["error"]["severity"], "high");
    }
}
