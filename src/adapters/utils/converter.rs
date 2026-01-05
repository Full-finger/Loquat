//! Event converter for adapting platform events to internal event format

use crate::events::EventEnum;
use crate::errors::{LoquatError, Result};

/// Event converter trait - converts platform-specific events to EventEnum
pub trait EventConverter<T>: Send + Sync {
    /// Convert platform event to EventEnum
    fn convert(&self, event: T) -> Result<EventEnum>;
    
    /// Get supported event types
    fn supported_types(&self) -> Vec<String>;
    
    /// Check if event type is supported
    fn supports(&self, event_type: &str) -> bool {
        self.supported_types().contains(&event_type.to_string())
    }
}

/// Message converter trait - specialized for message events
pub trait MessageConverter<T>: Send + Sync {
    /// Convert platform message to MessageEvent
    fn convert_message(&self, message: T) -> Result<crate::events::MessageEvent>;
    
    /// Check if message type is supported
    fn supports_message(&self, message_type: &str) -> bool;
}

/// Notice converter trait - specialized for notice events
pub trait NoticeConverter<T>: Send + Sync {
    /// Convert platform notice to NoticeEvent
    fn convert_notice(&self, notice: T) -> Result<crate::events::NoticeEvent>;
    
    /// Check if notice type is supported
    fn supports_notice(&self, notice_type: &str) -> bool;
}

/// Request converter trait - specialized for request events
pub trait RequestConverter<T>: Send + Sync {
    /// Convert platform request to RequestEvent
    fn convert_request(&self, request: T) -> Result<crate::events::RequestEvent>;
    
    /// Check if request type is supported
    fn supports_request(&self, request_type: &str) -> bool;
}

/// Meta event converter trait - specialized for meta events
pub trait MetaConverter<T>: Send + Sync {
    /// Convert platform meta event to MetaEvent
    fn convert_meta(&self, meta: T) -> Result<crate::events::MetaEvent>;
    
    /// Check if meta event type is supported
    fn supports_meta(&self, meta_type: &str) -> bool;
}

/// Conversion context - provides additional information for conversion
#[derive(Debug, Clone)]
pub struct ConversionContext {
    /// Adapter ID
    pub adapter_id: String,
    
    /// Platform type
    pub platform_type: String,
    
    /// Self ID (bot's own ID)
    pub self_id: String,
    
    /// Conversion options
    pub options: ConversionOptions,
}

impl ConversionContext {
    /// Create a new conversion context
    pub fn new(adapter_id: &str, platform_type: &str, self_id: &str) -> Self {
        Self {
            adapter_id: adapter_id.to_string(),
            platform_type: platform_type.to_string(),
            self_id: self_id.to_string(),
            options: ConversionOptions::default(),
        }
    }
    
    /// Set conversion options
    pub fn with_options(mut self, options: ConversionOptions) -> Self {
        self.options = options;
        self
    }
}

/// Conversion options
#[derive(Debug, Clone)]
pub struct ConversionOptions {
    /// Whether to include raw event data
    pub include_raw: bool,
    
    /// Whether to validate event structure
    pub validate: bool,
    
    /// Maximum event size in bytes (None for unlimited)
    pub max_size: Option<usize>,
    
    /// Timeout for conversion in milliseconds (None for no timeout)
    pub timeout: Option<u64>,
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            include_raw: false,
            validate: true,
            max_size: None,
            timeout: None,
        }
    }
}

impl ConversionOptions {
    /// Create new conversion options
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set whether to include raw event data
    pub fn with_include_raw(mut self, include_raw: bool) -> Self {
        self.include_raw = include_raw;
        self
    }
    
    /// Set whether to validate event structure
    pub fn with_validate(mut self, validate: bool) -> Self {
        self.validate = validate;
        self
    }
    
    /// Set maximum event size
    pub fn with_max_size(mut self, max_size: usize) -> Self {
        self.max_size = Some(max_size);
        self
    }
    
    /// Set conversion timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout = Some(timeout_ms);
        self
    }
}

/// Conversion result with metadata
#[derive(Debug)]
pub struct ConversionResult {
    /// Converted event
    pub event: EventEnum,
    
    /// Original event type
    pub original_type: String,
    
    /// Whether conversion was successful
    pub success: bool,
    
    /// Conversion errors (if any)
    pub errors: Vec<String>,
    
    /// Warnings generated during conversion
    pub warnings: Vec<String>,
}

impl ConversionResult {
    /// Create a successful conversion result
    pub fn success(event: EventEnum, original_type: &str) -> Self {
        Self {
            event,
            original_type: original_type.to_string(),
            success: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
    
    /// Create a failed conversion result
    pub fn failure(original_type: &str, error: String) -> Self {
        Self {
            // Create a placeholder event for failed conversions
            event: EventEnum::Meta(crate::events::MetaEvent::System {
                event_type: crate::events::SystemEventType::Other("conversion_failed".to_string()),
                description: error.clone(),
                data: std::collections::HashMap::new(),
                metadata: crate::events::EventMetadata::new("conversion.failed"),
            }),
            original_type: original_type.to_string(),
            success: false,
            errors: vec![error],
            warnings: Vec::new(),
        }
    }
    
    /// Add a warning to the result
    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversion_context_creation() {
        let ctx = ConversionContext::new("qq-001", "qq", "bot123");
        
        assert_eq!(ctx.adapter_id, "qq-001");
        assert_eq!(ctx.platform_type, "qq");
        assert_eq!(ctx.self_id, "bot123");
    }

    #[test]
    fn test_conversion_options_default() {
        let options = ConversionOptions::default();
        
        assert!(!options.include_raw);
        assert!(options.validate);
        assert!(options.max_size.is_none());
        assert!(options.timeout.is_none());
    }

    #[test]
    fn test_conversion_options_builder() {
        let options = ConversionOptions::new()
            .with_include_raw(true)
            .with_validate(false)
            .with_max_size(1024 * 1024)
            .with_timeout(5000);
        
        assert!(options.include_raw);
        assert!(!options.validate);
        assert_eq!(options.max_size, Some(1024 * 1024));
        assert_eq!(options.timeout, Some(5000));
    }

    #[test]
    fn test_conversion_result_success() {
        let event = EventEnum::Meta(crate::events::MetaEvent::System {
            event_type: crate::events::SystemEventType::ConfigUpdated,
            description: "Test".to_string(),
            data: std::collections::HashMap::new(),
            metadata: crate::events::EventMetadata::default(),
        });
        
        let result = ConversionResult::success(event, "test_event");
        
        assert!(result.success);
        assert_eq!(result.original_type, "test_event");
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_conversion_result_failure() {
        let result = ConversionResult::failure("test_event", "Conversion failed".to_string());
        
        assert!(!result.success);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0], "Conversion failed");
    }

    #[test]
    fn test_conversion_result_with_warning() {
        let event = EventEnum::Meta(crate::events::MetaEvent::System {
            event_type: crate::events::SystemEventType::ConfigUpdated,
            description: "Test".to_string(),
            data: std::collections::HashMap::new(),
            metadata: crate::events::EventMetadata::default(),
        });
        
        let result = ConversionResult::success(event, "test_event")
            .with_warning("Unknown field ignored".to_string());
        
        assert!(result.success);
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0], "Unknown field ignored");
    }
}
