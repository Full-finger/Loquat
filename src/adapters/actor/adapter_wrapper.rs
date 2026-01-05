//! Adapter wrapper for bridging sync trait and async actor
//!
//! This module provides a wrapper that implements sync Adapter trait
//! while internally using an async actor for state management and operations.

use crate::adapters::core::{AdapterConfig, AdapterStatus, types::AdapterStatistics};
use crate::errors::Result;
use crate::actor::messages::AdapterMessage;
use crate::events::EventEnum;
use std::fmt::Debug;
use tokio::sync::mpsc;

/// Wrapper that implements sync Adapter trait but uses async actor internally
#[derive(Debug)]
pub struct AdapterWrapper {
    adapter_id: String,
    name: String,
    version: String,
    config: AdapterConfig,
    message_sender: mpsc::UnboundedSender<AdapterMessage>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl AdapterWrapper {
    /// Create a new adapter wrapper from an actor
    pub fn new(
        adapter_id: String,
        name: String,
        version: String,
        config: AdapterConfig,
        message_sender: mpsc::UnboundedSender<AdapterMessage>,
        task_handle: Option<tokio::task::JoinHandle<()>>,
    ) -> Self {
        Self {
            adapter_id,
            name,
            version,
            config,
            message_sender,
            task_handle,
        }
    }

    /// Get message sender for this adapter
    pub fn message_sender(&self) -> &mpsc::UnboundedSender<AdapterMessage> {
        &self.message_sender
    }

    /// Get task handle (for cleanup)
    pub fn task_handle(&self) -> Option<&tokio::task::JoinHandle<()>> {
        self.task_handle.as_ref()
    }

    /// Send a start message to actor
    pub async fn start(&self) -> Result<()> {
        let (msg, rx) = AdapterMessage::start();
        self.message_sender.send(msg).map_err(|e| {
            crate::errors::AdapterError::LoadFailed(format!("Failed to send start message: {}", e))
        })?;
        rx.await.map_err(|e| {
            crate::errors::Error::Adapter(crate::errors::AdapterError::LoadFailed(format!("Start response error: {}", e)))
        })?
    }

    /// Send a stop message to actor
    pub async fn stop(&self) -> Result<()> {
        let (msg, rx) = AdapterMessage::stop();
        self.message_sender.send(msg).map_err(|e| {
            crate::errors::AdapterError::LoadFailed(format!("Failed to send stop message: {}", e))
        })?;
        rx.await.map_err(|e| {
            crate::errors::Error::Adapter(crate::errors::AdapterError::LoadFailed(format!("Stop response error: {}", e)))
        })?
    }

    /// Get status from actor
    pub async fn status(&self) -> AdapterStatus {
        let (msg, rx) = AdapterMessage::get_status();
        let _ = self.message_sender.send(msg);
        rx.await.unwrap_or(AdapterStatus::Error("Channel closed".to_string()))
    }

    /// Get statistics from actor
    pub async fn statistics(&self) -> AdapterStatistics {
        let (msg, rx) = AdapterMessage::get_statistics();
        let _ = self.message_sender.send(msg);
        rx.await.unwrap_or_default()
    }

    /// Check if adapter is running
    pub async fn is_running(&self) -> bool {
        self.status().await == AdapterStatus::Running
    }

    /// Check if adapter is connected
    pub async fn is_connected(&self) -> bool {
        self.status().await.is_active()
    }

    /// Set event sender for this adapter
    pub async fn set_event_sender(&self, sender: Option<mpsc::UnboundedSender<EventEnum>>) {
        let payload = serde_json::json!({"has_sender": sender.is_some()});
        let (msg, _rx) = AdapterMessage::custom("set_event_sender".to_string(), payload);
        let _ = self.message_sender.send(msg);
    }

    /// Send an event through adapter
    pub async fn send_event(&self, event: EventEnum) -> Result<()> {
        let (msg, rx) = AdapterMessage::custom(
            "send_event".to_string(),
            serde_json::to_value(&event).unwrap(),
        );
        self.message_sender.send(msg).map_err(|e| {
            crate::errors::AdapterError::LoadFailed(format!("Failed to send event: {}", e))
        })?;
        
        let _ = rx.await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::core::AdapterConfig;

    #[tokio::test]
    async fn test_adapter_wrapper_creation() {
        let config = AdapterConfig::new("test", "test-001", "ws://localhost");
        let (actor_sender, _receiver) = mpsc::unbounded_channel();

        let wrapper = AdapterWrapper::new(
            "test-001".to_string(),
            "TestAdapter".to_string(),
            "1.0.0".to_string(),
            config,
            actor_sender,
            None,
        );

        assert_eq!(wrapper.adapter_id, "test-001");
        assert_eq!(wrapper.name, "TestAdapter");
        assert_eq!(wrapper.version, "1.0.0");
    }
}

// Implement Adapter trait for AdapterWrapper
// This allows AdapterWrapper to be used as a trait object (dyn Adapter)
impl crate::adapters::core::Adapter for AdapterWrapper {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn adapter_id(&self) -> &str {
        &self.adapter_id
    }

    fn config(&self) -> crate::adapters::core::AdapterConfig {
        self.config.clone()
    }

    fn status(&self) -> crate::adapters::core::AdapterStatus {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(self.status())
        })
    }

    fn is_running(&self) -> bool {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.status().await == crate::adapters::core::AdapterStatus::Running
            })
        })
    }

    fn is_connected(&self) -> bool {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.status().await.is_active()
            })
        })
    }

    fn statistics(&self) -> crate::adapters::core::types::AdapterStatistics {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(self.statistics())
        })
    }

    fn set_event_sender(&self, sender: Option<mpsc::UnboundedSender<EventEnum>>) {
        let payload = serde_json::json!({"has_sender": sender.is_some()});
        let (msg, _rx) = AdapterMessage::custom("set_event_sender".to_string(), payload);
        let _ = self.message_sender.send(msg);
    }

    fn send_event(&self, event: EventEnum) -> Result<()> {
        let event_value = serde_json::to_value(&event)
            .map_err(|e| {
                crate::errors::Error::Adapter(crate::errors::AdapterError::LoadFailed(format!("Failed to serialize event: {}", e)))
            })?;
        let (msg, _rx) = AdapterMessage::custom(
            "send_event".to_string(),
            event_value,
        );
        self.message_sender.send(msg).map_err(|e| {
            crate::errors::Error::Adapter(crate::errors::AdapterError::LoadFailed(format!("Failed to send event message: {}", e)))
        })?;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
