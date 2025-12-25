//! Error type definitions and utilities

use std::collections::HashMap;
use uuid::Uuid;

/// Error context information for debugging and monitoring
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorContext {
    /// Unique error identifier
    pub error_id: String,
    
    /// Timestamp when error occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Error severity level
    pub severity: ErrorSeverity,
    
    /// Component where error occurred
    pub component: String,
    
    /// Operation being performed
    pub operation: Option<String>,
    
    /// Additional context data
    pub metadata: HashMap<String, serde_json::Value>,
    
    /// Stack trace information (in debug mode)
    #[serde(skip_serializing)]
    pub stack_trace: Option<String>,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new(component: &str, severity: ErrorSeverity) -> Self {
        Self {
            error_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            severity,
            component: component.to_string(),
            operation: None,
            metadata: HashMap::new(),
            stack_trace: None,
        }
    }
    
    /// Set component
    pub fn with_component(mut self, component: &str) -> Self {
        self.component = component.to_string();
        self
    }
    
    /// Set operation
    pub fn with_operation(mut self, operation: &str) -> Self {
        self.operation = Some(operation.to_string());
        self
    }
    
    /// Add metadata
    pub fn with_metadata<K: Into<String>, V: serde::Serialize>(mut self, key: K, value: V) -> Result<Self, Box<dyn std::error::Error>> {
        let serialized = serde_json::to_value(value)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        self.metadata.insert(key.into(), serialized);
        Ok(self)
    }
    
    /// Add stack trace (debug only)
    #[cfg(debug_assertions)]
    pub fn with_stack_trace(self) -> Self {
        use std::backtrace::Backtrace;
        Self {
            stack_trace: Some(format!("{:?}", Backtrace::capture())),
            ..self
        }
    }
    
    #[cfg(not(debug_assertions))]
    pub fn with_stack_trace(self) -> Self {
        self
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorSeverity::Low => write!(f, "low"),
            ErrorSeverity::Medium => write!(f, "medium"),
            ErrorSeverity::High => write!(f, "high"),
            ErrorSeverity::Critical => write!(f, "critical"),
        }
    }
}

/// Enhanced error with context
#[derive(Debug)]
pub struct ContextualError {
    pub error: String,
    pub context: ErrorContext,
}

impl ContextualError {
    /// Create a new contextual error
    pub fn new(error: String, context: ErrorContext) -> Self {
        Self { error, context }
    }
    
    /// Create from error with basic context
    pub fn from_error(error: String, component: &str, severity: ErrorSeverity) -> Self {
        let context = ErrorContext::new(component, severity);
        Self::new(error, context)
    }
    
    /// Create from error with operation context
    pub fn from_error_with_operation(
        error: String,
        component: &str,
        operation: &str,
        severity: ErrorSeverity,
    ) -> Self {
        let context = ErrorContext::new(component, severity)
            .with_operation(operation);
        Self::new(error, context)
    }
}

impl std::fmt::Display for ContextualError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", 
            self.context.severity,
            self.context.component,
            self.error
        )
    }
}

impl std::error::Error for ContextualError {}

/// Error reporting utilities
pub struct ErrorReporter;

impl ErrorReporter {
    /// Report error with context (in production, this would send to monitoring service)
    pub fn report(error: &ContextualError) {
        // Errors should be logged through the logging system
        // In a real implementation, this would send to external monitoring service
        // For now, we silently report errors without printing to stderr
        let _ = error;
        
        #[cfg(debug_assertions)]
        if let Some(stack_trace) = &error.context.stack_trace {
            // Stack trace would be logged through the logging system in production
            let _ = stack_trace;
        }
    }
    
    /// Report error with custom metadata
    pub fn report_with_metadata(
        error: &ContextualError,
        metadata: HashMap<String, serde_json::Value>,
    ) {
        let enriched_error = ContextualError {
            error: error.error.clone(),
            context: ErrorContext {
                error_id: error.context.error_id.clone(),
                timestamp: error.context.timestamp,
                severity: error.context.severity,
                component: error.context.component.clone(),
                operation: error.context.operation.clone(),
                metadata: {
                    let mut new_metadata = error.context.metadata.clone();
                    new_metadata.extend(metadata);
                    new_metadata
                },
                stack_trace: error.context.stack_trace.clone(),
            },
        };
        Self::report(&enriched_error);
    }
}

/// Trait for adding context to errors
pub trait ErrorContextExt<T> {
    /// Add error context
    fn with_context(self, component: &str, severity: ErrorSeverity) -> std::result::Result<T, Box<dyn std::error::Error>>;
    
    /// Add error context with operation
    fn with_context_operation(
        self,
        component: &str,
        operation: &str,
        severity: ErrorSeverity,
    ) -> std::result::Result<T, Box<dyn std::error::Error>>;
}

impl<T, E> ErrorContextExt<T> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn with_context(self, component: &str, severity: ErrorSeverity) -> std::result::Result<T, Box<dyn std::error::Error>> {
        self.map_err(|e| {
            let contextual_error = ContextualError::from_error(e.to_string(), component, severity);
            ErrorReporter::report(&contextual_error);
            Box::new(contextual_error) as Box<dyn std::error::Error>
        })
    }
    
    fn with_context_operation(
        self,
        component: &str,
        operation: &str,
        severity: ErrorSeverity,
    ) -> std::result::Result<T, Box<dyn std::error::Error>> {
        self.map_err(|e| {
            let contextual_error = ContextualError::from_error_with_operation(
                e.to_string(),
                component,
                operation,
                severity,
            );
            ErrorReporter::report(&contextual_error);
            Box::new(contextual_error) as Box<dyn std::error::Error>
        })
    }
}
