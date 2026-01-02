//! Actor pattern implementation for adapters
//!
//! This module provides the actor pattern implementation to solve
//! the async trait object problem. Actors handle messages asynchronously
//! and maintain their own state.

pub mod messages;
pub mod adapter_wrapper;
pub mod console_adapter_actor;
#[cfg(test)]
mod integration_test;

pub use messages::AdapterMessage;
pub use adapter_wrapper::AdapterWrapper;
pub use console_adapter_actor::{ConsoleAdapterActor, create_console_adapter_actor};

use crate::adapters::{AdapterConfig, AdapterStatus, types::AdapterStatistics};
use crate::errors::{AdapterError, LoquatError, Result};
use crate::events::EventEnum;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Trait for actor-based adapters
///
/// This trait should be implemented by adapter-specific actors
/// to handle custom messages and implement adapter-specific logic.
#[async_trait::async_trait]
pub trait AdapterActor: Send + Sync + Clone {
    /// Handle adapter-specific start logic
    async fn do_start(&mut self) -> Result<()> {
        Ok(())
    }

    /// Handle adapter-specific stop logic
    async fn do_stop(&mut self) -> Result<()> {
        Ok(())
    }

    /// Handle custom messages
    async fn handle_custom(&mut self, message_type: String, payload: serde_json::Value) -> Result<serde_json::Value> {
        Err(LoquatError::Adapter(AdapterError::LoadFailed(format!(
            "Custom message type '{}' not supported",
            message_type
        ))))
    }
}

/// Base adapter actor implementation
///
/// This struct provides core actor functionality including
/// message handling and state management with Arc<RwLock<>> for thread-safe access.
#[derive(Clone, Debug)]
pub struct BaseAdapterActor {
    /// Adapter configuration
    pub config: AdapterConfig,
    /// Adapter name
    pub name: String,
    /// Adapter version
    pub version: String,
    /// Current status (wrapped in Arc<RwLock<>> for thread-safe access)
    pub status: Arc<RwLock<AdapterStatus>>,
    /// Statistics (wrapped in Arc<RwLock<>> for thread-safe access)
    pub statistics: Arc<RwLock<AdapterStatistics>>,
}

impl BaseAdapterActor {
    /// Create a new base adapter actor
    pub fn new(config: AdapterConfig, name: String, version: String) -> Self {
        Self {
            config,
            name,
            version,
            status: Arc::new(RwLock::new(AdapterStatus::Ready)),
            statistics: Arc::new(RwLock::new(AdapterStatistics::default())),
        }
    }

    /// Get the adapter's name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the adapter's version
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get the adapter's configuration
    pub fn config(&self) -> &AdapterConfig {
        &self.config
    }

    /// Get the adapter's status
    pub async fn status(&self) -> AdapterStatus {
        self.status.read().await.clone()
    }

    /// Set the adapter's status
    pub async fn set_status(&self, status: AdapterStatus) {
        *self.status.write().await = status;
    }

    /// Get the adapter's statistics
    pub async fn statistics(&self) -> AdapterStatistics {
        self.statistics.read().await.clone()
    }

    /// Update statistics
    pub async fn update_statistics<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut AdapterStatistics),
    {
        let mut stats = self.statistics.write().await;
        update_fn(&mut stats);
    }

    /// Handle a message
    pub async fn handle_message(&mut self, msg: AdapterMessage) -> Result<()> {
        match msg {
            AdapterMessage::Start { respond_to } => {
                let result = self.handle_start().await;
                let _ = respond_to.send(result);
            }
            AdapterMessage::Stop { respond_to } => {
                let result = self.handle_stop().await;
                let _ = respond_to.send(result);
            }
            AdapterMessage::GetStatus { respond_to } => {
                let status = self.status().await;
                let _ = respond_to.send(status);
            }
            AdapterMessage::GetStatistics { respond_to } => {
                let stats = self.statistics().await;
                let _ = respond_to.send(stats);
            }
            AdapterMessage::GetConfig { respond_to } => {
                let _ = respond_to.send(self.config.clone());
            }
            AdapterMessage::IsRunning { respond_to } => {
                let is_running = self.status().await == AdapterStatus::Running;
                let _ = respond_to.send(is_running);
            }
            AdapterMessage::IsConnected { respond_to } => {
                let status = self.status().await;
                let _ = respond_to.send(status.is_active());
            }
            AdapterMessage::Custom {
                message_type,
                payload,
                respond_to,
            } => {
                let result = self.handle_custom(message_type, payload).await;
                let _ = respond_to.send(result);
            }
        }
        Ok(())
    }

    /// Handle start message
    async fn handle_start(&mut self) -> Result<()> {
        let current_status = self.status().await;
        if current_status == AdapterStatus::Running {
            return Err(AdapterError::LoadFailed("Adapter is already running".to_string()).into());
        }

        // Call the specific actor's do_start method
        self.do_start().await?;

        // Update status
        self.set_status(AdapterStatus::Running).await;

        // Update start time in statistics
        self.update_statistics(|stats| {
            stats.uptime_seconds = 0;
            stats.last_activity = Some(chrono::Utc::now().timestamp());
        })
        .await;

        Ok(())
    }

    /// Handle stop message
    async fn handle_stop(&mut self) -> Result<()> {
        let current_status = self.status().await;
        if current_status != AdapterStatus::Running {
            return Ok(()); // Already stopped
        }

        // Call the specific actor's do_stop method
        self.do_stop().await?;

        // Update status
        self.set_status(AdapterStatus::Stopped).await;

        Ok(())
    }
}

// Implement AdapterActor trait for BaseAdapterActor
#[async_trait::async_trait]
impl AdapterActor for BaseAdapterActor {
    async fn do_start(&mut self) -> Result<()> {
        // Default implementation does nothing
        Ok(())
    }

    async fn do_stop(&mut self) -> Result<()> {
        // Default implementation does nothing
        Ok(())
    }
}

// Implement Adapter trait for BaseAdapterActor
impl crate::adapters::Adapter for BaseAdapterActor {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn adapter_id(&self) -> &str {
        self.config.adapter_id.as_str()
    }

    fn config(&self) -> crate::adapters::AdapterConfig {
        self.config.clone()
    }

    fn status(&self) -> crate::adapters::AdapterStatus {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async {
                    self.status.read().await.clone()
                })
        })
    }

    fn is_running(&self) -> bool {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async {
                    self.status.read().await.clone() == crate::adapters::AdapterStatus::Running
                })
        })
    }

    fn is_connected(&self) -> bool {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async {
                    self.status.read().await.clone().is_active()
                })
        })
    }

    fn statistics(&self) -> crate::adapters::types::AdapterStatistics {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async {
                    self.statistics.read().await.clone()
                })
        })
    }

    fn set_event_sender(&self, _sender: Option<tokio::sync::mpsc::UnboundedSender<crate::events::EventEnum>>) {
        // Base adapter doesn't need to send events
    }

    fn send_event(&self, _event: crate::events::EventEnum) -> crate::errors::Result<()> {
        // Base adapter doesn't need to send events
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_base_actor_creation() {
        let config = AdapterConfig::new("test", "test-001", "ws://localhost");
        let actor = BaseAdapterActor::new(config, "TestAdapter".to_string(), "1.0.0".to_string());

        assert_eq!(actor.name(), "TestAdapter");
        assert_eq!(actor.version(), "1.0.0");
        assert_eq!(actor.status().await, AdapterStatus::Ready);
        assert_eq!(actor.statistics().await.events_received, 0);
    }

    #[tokio::test]
    async fn test_base_actor_status_management() {
        let config = AdapterConfig::new("test", "test-002", "ws://localhost");
        let mut actor = BaseAdapterActor::new(config, "TestAdapter".to_string(), "1.0.0".to_string());

        // Test initial status
        assert_eq!(actor.status().await, AdapterStatus::Ready);

        // Test status update
        actor.set_status(AdapterStatus::Running).await;
        assert_eq!(actor.status().await, AdapterStatus::Running);

        actor.set_status(AdapterStatus::Stopped).await;
        assert_eq!(actor.status().await, AdapterStatus::Stopped);
    }

    #[tokio::test]
    async fn test_base_actor_statistics_update() {
        let config = AdapterConfig::new("test", "test-003", "ws://localhost");
        let actor = BaseAdapterActor::new(config, "TestAdapter".to_string(), "1.0.0".to_string());

        // Test initial statistics
        let stats = actor.statistics().await;
        assert_eq!(stats.events_received, 0);

        // Test statistics update
        actor.update_statistics(|s| s.events_received = 5).await;
        let stats = actor.statistics().await;
        assert_eq!(stats.events_received, 5);
    }

    #[tokio::test]
    async fn test_base_actor_handle_message() {
        let config = AdapterConfig::new("test", "test-004", "ws://localhost");
        let mut actor = BaseAdapterActor::new(config, "TestAdapter".to_string(), "1.0.0".to_string());

        // Test Start message
        let (tx, rx) = tokio::sync::oneshot::channel();
        let msg = AdapterMessage::Start { respond_to: tx };
        actor.handle_message(msg).await.unwrap();
        let result = rx.await.unwrap();
        assert!(result.is_ok());
        assert_eq!(actor.status().await, AdapterStatus::Running);

        // Test Stop message
        let (tx, rx) = tokio::sync::oneshot::channel();
        let msg = AdapterMessage::Stop { respond_to: tx };
        actor.handle_message(msg).await.unwrap();
        let result = rx.await.unwrap();
        assert!(result.is_ok());
        assert_eq!(actor.status().await, AdapterStatus::Stopped);

        // Test double start (should fail)
        let (tx, rx) = tokio::sync::oneshot::channel();
        let msg = AdapterMessage::Start { respond_to: tx };
        actor.handle_message(msg).await.unwrap();
        let result = rx.await.unwrap();
        assert!(result.is_ok());

        // Try starting again while running
        actor.set_status(AdapterStatus::Running).await;
        let (tx, rx) = tokio::sync::oneshot::channel();
        let msg = AdapterMessage::Start { respond_to: tx };
        actor.handle_message(msg).await.unwrap();
        let result = rx.await.unwrap();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_base_actor_custom_message() {
        let config = AdapterConfig::new("test", "test-005", "ws://localhost");
        let mut actor = BaseAdapterActor::new(config, "TestAdapter".to_string(), "1.0.0".to_string());

        let (tx, rx) = tokio::sync::oneshot::channel();
        let msg = AdapterMessage::Custom {
            message_type: "test".to_string(),
            payload: serde_json::json!({}),
            respond_to: tx,
        };
        actor.handle_message(msg).await.unwrap();
        let result = rx.await.unwrap();
        assert!(result.is_err());
    }
}
