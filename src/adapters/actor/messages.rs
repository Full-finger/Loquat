//! Actor message definitions for adapter communication
//!
//! This module defines the message types used for communication
//! between adapters and the rest of the system via the actor pattern.

use crate::adapters::core::{AdapterConfig, AdapterStatus, types::AdapterStatistics};
use crate::errors::Result;
use tokio::sync::oneshot;

/// Messages that can be sent to an adapter actor
#[derive(Debug)]
pub enum AdapterMessage {
    /// Start the adapter
    /// Returns Result<()> indicating success or failure
    Start {
        respond_to: oneshot::Sender<Result<()>>,
    },

    /// Stop the adapter
    /// Returns Result<()> indicating success or failure
    Stop {
        respond_to: oneshot::Sender<Result<()>>,
    },

    /// Get the current status of the adapter
    /// Returns AdapterStatus
    GetStatus {
        respond_to: oneshot::Sender<AdapterStatus>,
    },

    /// Get adapter statistics
    /// Returns AdapterStatistics
    GetStatistics {
        respond_to: oneshot::Sender<AdapterStatistics>,
    },

    /// Get adapter configuration
    /// Returns AdapterConfig
    GetConfig {
        respond_to: oneshot::Sender<AdapterConfig>,
    },

    /// Check if adapter is running
    /// Returns bool
    IsRunning {
        respond_to: oneshot::Sender<bool>,
    },

    /// Check if adapter is connected
    /// Returns bool
    IsConnected {
        respond_to: oneshot::Sender<bool>,
    },

    /// Custom adapter-specific message
    /// The payload is a JSON value that can be deserialized by the specific adapter
    Custom {
        message_type: String,
        payload: serde_json::Value,
        respond_to: oneshot::Sender<Result<serde_json::Value>>,
    },
}

impl AdapterMessage {
    /// Create a Start message
    pub fn start() -> (Self, oneshot::Receiver<Result<()>>) {
        let (tx, rx) = oneshot::channel();
        (Self::Start { respond_to: tx }, rx)
    }

    /// Create a Stop message
    pub fn stop() -> (Self, oneshot::Receiver<Result<()>>) {
        let (tx, rx) = oneshot::channel();
        (Self::Stop { respond_to: tx }, rx)
    }

    /// Create a GetStatus message
    pub fn get_status() -> (Self, oneshot::Receiver<AdapterStatus>) {
        let (tx, rx) = oneshot::channel();
        (Self::GetStatus { respond_to: tx }, rx)
    }

    /// Create a GetStatistics message
    pub fn get_statistics() -> (Self, oneshot::Receiver<AdapterStatistics>) {
        let (tx, rx) = oneshot::channel();
        (Self::GetStatistics { respond_to: tx }, rx)
    }

    /// Create a GetConfig message
    pub fn get_config() -> (Self, oneshot::Receiver<AdapterConfig>) {
        let (tx, rx) = oneshot::channel();
        (Self::GetConfig { respond_to: tx }, rx)
    }

    /// Create an IsRunning message
    pub fn is_running() -> (Self, oneshot::Receiver<bool>) {
        let (tx, rx) = oneshot::channel();
        (Self::IsRunning { respond_to: tx }, rx)
    }

    /// Create an IsConnected message
    pub fn is_connected() -> (Self, oneshot::Receiver<bool>) {
        let (tx, rx) = oneshot::channel();
        (Self::IsConnected { respond_to: tx }, rx)
    }

    /// Create a Custom message
    pub fn custom(
        message_type: String,
        payload: serde_json::Value,
    ) -> (Self, oneshot::Receiver<Result<serde_json::Value>>) {
        let (tx, rx) = oneshot::channel();
        (
            Self::Custom {
                message_type,
                payload,
                respond_to: tx,
            },
            rx,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let (msg, _rx) = AdapterMessage::start();
        matches!(msg, AdapterMessage::Start { .. });

        let (msg, _rx) = AdapterMessage::stop();
        matches!(msg, AdapterMessage::Stop { .. });

        let (msg, _rx) = AdapterMessage::get_status();
        matches!(msg, AdapterMessage::GetStatus { .. });
    }

    #[test]
    fn test_custom_message_creation() {
        let (msg, _rx) = AdapterMessage::custom(
            "test".to_string(),
            serde_json::json!({"key": "value"}),
        );
        matches!(msg, AdapterMessage::Custom { .. });
    }
}
