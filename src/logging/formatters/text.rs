//! Text log formatter implementation

use crate::logging::traits::{LogEntry, LogFormatter};

/// Text formatter for human-readable logs
#[derive(Debug, Clone)]
pub struct TextFormatter {
    include_timestamp: bool,
    include_level: bool,
    include_component: bool,
    include_metadata: bool,
    format_type: TextFormatType,
}

/// Different text formatting styles
#[derive(Debug, Clone, Copy)]
pub enum TextFormatType {
    /// Simple format: [LEVEL] message
    Simple,
    /// Detailed format: timestamp [LEVEL] [component] message
    Detailed,
    /// Compact format: timestamp level message
    Compact,
    /// Custom format with all fields
    Full,
}

impl TextFormatter {
    /// Create a new text formatter with detailed format
    pub fn new() -> Self {
        Self {
            include_timestamp: true,
            include_level: true,
            include_component: true,
            include_metadata: false,
            format_type: TextFormatType::Detailed,
        }
    }

    /// Create a detailed text formatter
    pub fn detailed() -> Self {
        Self {
            include_timestamp: true,
            include_level: true,
            include_component: true,
            include_metadata: false,
            format_type: TextFormatType::Detailed,
        }
    }

    /// Create a simple text formatter
    pub fn simple() -> Self {
        Self {
            include_timestamp: false,
            include_level: true,
            include_component: false,
            include_metadata: false,
            format_type: TextFormatType::Simple,
        }
    }

    /// Create a compact text formatter
    pub fn compact() -> Self {
        Self {
            include_timestamp: true,
            include_level: true,
            include_component: false,
            include_metadata: false,
            format_type: TextFormatType::Compact,
        }
    }

    /// Create a full text formatter with all information
    pub fn full() -> Self {
        Self {
            include_timestamp: true,
            include_level: true,
            include_component: true,
            include_metadata: true,
            format_type: TextFormatType::Full,
        }
    }

    /// Configure timestamp inclusion
    pub fn with_timestamp(mut self, include: bool) -> Self {
        self.include_timestamp = include;
        self
    }

    /// Configure level inclusion
    pub fn with_level(mut self, include: bool) -> Self {
        self.include_level = include;
        self
    }

    /// Configure component inclusion
    pub fn with_component(mut self, include: bool) -> Self {
        self.include_component = include;
        self
    }

    /// Configure metadata inclusion
    pub fn with_metadata(mut self, include: bool) -> Self {
        self.include_metadata = include;
        self
    }

    /// Set format type
    pub fn with_format(mut self, format_type: TextFormatType) -> Self {
        self.format_type = format_type;
        self
    }

    /// Format timestamp for display
    fn format_timestamp(&self, timestamp: chrono::DateTime<chrono::Utc>) -> String {
        match self.format_type {
            TextFormatType::Simple => String::new(),
            TextFormatType::Compact => timestamp.format("%H:%M:%S%.3f").to_string(),
            TextFormatType::Detailed => timestamp.format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
            TextFormatType::Full => timestamp.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        }
    }

    /// Format level with appropriate colors (if supported)
    fn format_level(&self, level: crate::logging::traits::LogLevel) -> String {
        if !self.include_level {
            return String::new();
        }

        let level_str = match level {
            crate::logging::traits::LogLevel::Trace => "TRACE",
            crate::logging::traits::LogLevel::Debug => "DEBUG",
            crate::logging::traits::LogLevel::Info => "INFO",
            crate::logging::traits::LogLevel::Warn => "WARN",
            crate::logging::traits::LogLevel::Error => "ERROR",
        };

        match self.format_type {
            TextFormatType::Simple => format!("[{}]", level_str),
            TextFormatType::Compact => format!("{} ", level_str),
            TextFormatType::Detailed | TextFormatType::Full => {
                let colored = match level {
                    crate::logging::traits::LogLevel::Error => {
                        format!("\x1b[31m{}\x1b[0m", level_str) // Red
                    }
                    crate::logging::traits::LogLevel::Warn => {
                        format!("\x1b[33m{}\x1b[0m", level_str) // Yellow
                    }
                    crate::logging::traits::LogLevel::Info => {
                        format!("\x1b[32m{}\x1b[0m", level_str) // Green
                    }
                    crate::logging::traits::LogLevel::Debug => {
                        format!("\x1b[36m{}\x1b[0m", level_str) // Cyan
                    }
                    crate::logging::traits::LogLevel::Trace => {
                        format!("\x1b[37m{}\x1b[0m", level_str) // White
                    }
                };
                format!("[{}]", colored)
            }
        }
    }

    /// Format component information
    fn format_component(&self, component: Option<&str>) -> String {
        if !self.include_component {
            return String::new();
        }

        match component {
            Some(comp) => match self.format_type {
                TextFormatType::Detailed | TextFormatType::Full => format!("[{}] ", comp),
                _ => String::new(),
            },
            None => String::new(),
        }
    }

    /// Format metadata
    fn format_metadata(&self, entry: &LogEntry) -> String {
        if !self.include_metadata || entry.context.metadata.is_empty() {
            return String::new();
        }

        let metadata_str = entry
            .context
            .metadata
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(", ");

        match self.format_type {
            TextFormatType::Full => format!(" | {}", metadata_str),
            _ => String::new(),
        }
    }

    /// Format additional context information
    fn format_additional_context(&self, entry: &LogEntry) -> String {
        let mut parts = Vec::new();

        if let Some(correlation_id) = &entry.context.correlation_id {
            parts.push(format!("cid:{}", correlation_id));
        }

        if let Some(user_id) = &entry.context.user_id {
            parts.push(format!("user:{}", user_id));
        }

        if let Some(request_info) = &entry.context.request_info {
            parts.push(format!("{}{}", request_info.method, request_info.path));
        }

        if parts.is_empty() {
            String::new()
        } else {
            match self.format_type {
                TextFormatType::Full => format!(" ({})", parts.join(", ")),
                _ => String::new(),
            }
        }
    }
}

impl Default for TextFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl LogFormatter for TextFormatter {
    fn format(&self, entry: &LogEntry) -> String {
        let mut parts = Vec::new();

        // Add timestamp if enabled
        if self.include_timestamp {
            let timestamp = self.format_timestamp(entry.timestamp);
            if !timestamp.is_empty() {
                parts.push(timestamp);
            }
        }

        // Add level if enabled
        let level = self.format_level(entry.level);
        if !level.is_empty() {
            parts.push(level);
        }

        // Add component if enabled
        let component = self.format_component(entry.context.component.as_deref());
        if !component.is_empty() {
            parts.push(component);
        }

        // Add the main message
        parts.push(entry.message.clone());

        // Build the base message
        let mut message = parts.join(" ");

        // Add metadata if enabled
        let metadata = self.format_metadata(entry);
        if !metadata.is_empty() {
            message.push_str(&metadata);
        }

        // Add additional context
        let additional_context = self.format_additional_context(entry);
        if !additional_context.is_empty() {
            message.push_str(&additional_context);
        }

        message
    }

    fn format_batch(&self, entries: &[LogEntry]) -> Vec<String> {
        entries
            .iter()
            .map(|entry| self.format(entry))
            .collect()
    }
}

/// Colored text formatter for console output
#[derive(Debug, Clone)]
pub struct ColoredTextFormatter {
    base: TextFormatter,
    enable_colors: bool,
}

impl ColoredTextFormatter {
    /// Create a new colored text formatter
    pub fn new() -> Self {
        Self {
            base: TextFormatter::full(),
            enable_colors: true,
        }
    }

    /// Disable colors
    pub fn without_colors(mut self) -> Self {
        self.enable_colors = false;
        self
    }

    /// Check if colors should be enabled (based on terminal support)
    fn should_use_colors(&self) -> bool {
        self.enable_colors && atty::is(atty::Stream::Stdout)
    }
}

impl Default for ColoredTextFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl LogFormatter for ColoredTextFormatter {
    fn format(&self, entry: &LogEntry) -> String {
        let formatted = self.base.format(entry);
        
        if !self.should_use_colors() {
            return formatted;
        }

        // Add additional coloring for the entire line based on level
        match entry.level {
            crate::logging::traits::LogLevel::Error => {
                format!("\x1b[31m{}\x1b[0m", formatted) // Red background for errors
            }
            crate::logging::traits::LogLevel::Warn => {
                format!("\x1b[33m{}\x1b[0m", formatted) // Yellow for warnings
            }
            _ => formatted, // No additional coloring for other levels
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::traits::{LogContext, LogLevel};

    #[test]
    fn test_text_formatter_simple() {
        let formatter = TextFormatter::simple();
        let context = LogContext::new();
        let entry = LogEntry::new(LogLevel::Info, "Test message".to_string(), context);

        let formatted = formatter.format(&entry);
        assert_eq!(formatted, "[INFO] Test message");
    }

    #[test]
    fn test_text_formatter_detailed() {
        let formatter = TextFormatter::detailed();
        let context = LogContext::new().with_component("test");
        let entry = LogEntry::new(LogLevel::Info, "Test message".to_string(), context);

        let formatted = formatter.format(&entry);
        
        // Should contain INFO level (may have color codes), component, and message
        assert!(formatted.contains("INFO") || formatted.contains("INFO")); // Check for INFO text
        assert!(formatted.contains("[test]"));
        assert!(formatted.contains("Test message"));
    }

    #[test]
    fn test_text_formatter_with_correlation_id() {
        let formatter = TextFormatter::full();
        let context = LogContext::new()
            .with_component("api")
            .with_correlation_id("req-123".to_string());
        let entry = LogEntry::new(LogLevel::Info, "Request processed".to_string(), context);

        let formatted = formatter.format(&entry);
        
        assert!(formatted.contains("cid:req-123"));
        assert!(formatted.contains("[api]"));
    }

    #[test]
    fn test_colored_text_formatter() {
        let formatter = ColoredTextFormatter::new();
        let context = LogContext::new();
        let entry = LogEntry::new(LogLevel::Error, "Error message".to_string(), context);

        let formatted = formatter.format(&entry);
        
        // Should contain ANSI color codes (if colors are enabled)
        if atty::is(atty::Stream::Stdout) {
            assert!(formatted.contains("\x1b["));
        }
        
        assert!(formatted.contains("Error message"));
    }
}
