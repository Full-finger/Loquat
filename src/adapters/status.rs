//! Adapter status definitions

use serde::{Deserialize, Serialize};

/// Adapter status - represents current state of an adapter
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdapterStatus {
    /// Not initialized
    Uninitialized,
    /// Initializing
    Initializing,
    /// Ready to start
    Ready,
    /// Running and processing events
    Running,
    /// Temporarily paused
    Paused,
    /// Stopped gracefully
    Stopped,
    /// Error occurred
    Error(String),
}

impl AdapterStatus {
    /// Check if adapter is active (ready, running, or paused)
    pub fn is_active(&self) -> bool {
        matches!(self, AdapterStatus::Ready | AdapterStatus::Running | AdapterStatus::Paused)
    }

    /// Check if adapter is currently processing events
    pub fn is_processing(&self) -> bool {
        matches!(self, AdapterStatus::Running)
    }

    /// Check if adapter has an error
    pub fn is_error(&self) -> bool {
        matches!(self, AdapterStatus::Error(_))
    }

    /// Get error message if in error state
    pub fn error_message(&self) -> Option<&str> {
        match self {
            AdapterStatus::Error(msg) => Some(msg),
            _ => None,
        }
    }
}

impl Default for AdapterStatus {
    fn default() -> Self {
        AdapterStatus::Uninitialized
    }
}

impl std::fmt::Display for AdapterStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterStatus::Uninitialized => write!(f, "Uninitialized"),
            AdapterStatus::Initializing => write!(f, "Initializing"),
            AdapterStatus::Ready => write!(f, "Ready"),
            AdapterStatus::Running => write!(f, "Running"),
            AdapterStatus::Paused => write!(f, "Paused"),
            AdapterStatus::Stopped => write!(f, "Stopped"),
            AdapterStatus::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_status_is_active() {
        assert!(!AdapterStatus::Uninitialized.is_active());
        assert!(!AdapterStatus::Initializing.is_active());
        assert!(AdapterStatus::Ready.is_active());
        assert!(AdapterStatus::Running.is_active());
        assert!(AdapterStatus::Paused.is_active());
        assert!(!AdapterStatus::Stopped.is_active());
        assert!(!AdapterStatus::Error("test".to_string()).is_active());
    }

    #[test]
    fn test_adapter_status_is_processing() {
        assert!(!AdapterStatus::Ready.is_processing());
        assert!(AdapterStatus::Running.is_processing());
        assert!(!AdapterStatus::Paused.is_processing());
    }

    #[test]
    fn test_adapter_status_is_error() {
        assert!(!AdapterStatus::Running.is_error());
        assert!(AdapterStatus::Error("test error".to_string()).is_error());
    }

    #[test]
    fn test_adapter_status_error_message() {
        let status = AdapterStatus::Error("Connection failed".to_string());
        assert_eq!(status.error_message(), Some("Connection failed"));

        let status = AdapterStatus::Running;
        assert_eq!(status.error_message(), None);
    }

    #[test]
    fn test_adapter_status_display() {
        assert_eq!(format!("{}", AdapterStatus::Running), "Running");
        assert_eq!(format!("{}", AdapterStatus::Error("test".to_string())), "Error: test");
    }
}
