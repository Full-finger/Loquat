//! Console Adapter Actor - Actor-based implementation for console I/O

use super::messages::AdapterMessage;
use super::{BaseAdapterActor, AdapterActor};
use crate::adapters::{AdapterConfig, AdapterStatus};
use crate::adapters::types::AdapterStatistics;
use crate::errors::{AdapterError, LoquatError, Result};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{mpsc, oneshot, RwLock};

/// Console adapter actor implementation
#[derive(Clone)]
pub struct ConsoleAdapterActor {
    /// Base actor functionality
    base: BaseAdapterActor,
    /// Running state
    running: Arc<RwLock<bool>>,
    /// Event sender channel
    event_sender: Arc<RwLock<Option<mpsc::UnboundedSender<crate::events::EventEnum>>>>,
}

impl ConsoleAdapterActor {
    /// Create a new console adapter actor
    pub fn new(config: AdapterConfig) -> Self {
        Self {
            base: BaseAdapterActor::new(
                config.clone(),
                "ConsoleAdapter".to_string(),
                "1.0.0".to_string(),
            ),
            running: Arc::new(RwLock::new(false)),
            event_sender: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the event sender channel
    pub async fn set_event_sender(&self, sender: mpsc::UnboundedSender<crate::events::EventEnum>) {
        *self.event_sender.write().await = Some(sender);
    }
}

#[async_trait::async_trait]
impl AdapterActor for ConsoleAdapterActor {
    async fn do_start(&mut self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(LoquatError::Adapter(AdapterError::LoadFailed(
                "Adapter is already running".to_string(),
            )));
        }
        *running = true;
        self.base.status = Arc::new(RwLock::new(AdapterStatus::Running));
        drop(running);

        // Spawn stdin reader task
        let running_clone = Arc::clone(&self.running);
        let event_sender_clone = Arc::clone(&self.event_sender);
        let stats_clone = Arc::clone(&self.base.statistics);
        let adapter_id = self.base.config.adapter_id.clone();

        tokio::spawn(async move {
            let stdin = tokio::io::stdin();
            let reader = BufReader::new(stdin);
            let mut lines = reader.lines();

            println!(
                "[{}] Console adapter started. Type messages and press Enter to send.",
                adapter_id
            );
            println!("[{}] Type 'quit' or 'exit' to stop the adapter.", adapter_id);

            while *running_clone.read().await {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        let line = line.trim();

                        // Check for quit command
                        if line.eq_ignore_ascii_case("quit") || line.eq_ignore_ascii_case("exit") {
                            println!("[{}] Stopping adapter...", adapter_id);
                            *running_clone.write().await = false;
                            break;
                        }

                        // Process the input
                        println!("[{}] Received: {}", adapter_id, line);

                        // Update statistics
                        let mut stats = stats_clone.write().await;
                        stats.events_received += 1;
                        stats.last_activity = Some(chrono::Utc::now().timestamp());
                        drop(stats);

                        // Send event if channel exists
                        let sender = event_sender_clone.read().await;
                        if let Some(sender) = sender.clone() {
                            // Create a text message event
                            let metadata = crate::events::EventMetadata::new("message.text")
                                .with_source(crate::events::EventSource::Worker(adapter_id.clone()));
                            
                            let event = crate::events::EventEnum::Message(
                                crate::events::MessageEvent::Text {
                                    text: line.to_string(),
                                    metadata,
                                }
                            );
                            
                            match sender.send(event) {
                                Ok(_) => {
                                    println!("[{}] Event sent to event system", adapter_id);
                                    let mut stats = stats_clone.write().await;
                                    stats.events_sent += 1;
                                }
                                Err(e) => {
                                    println!("[{}] Failed to send event: {}", adapter_id, e);
                                    let mut stats = stats_clone.write().await;
                                    stats.errors += 1;
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        // EOF reached
                        println!("[{}] End of input", adapter_id);
                        *running_clone.write().await = false;
                        break;
                    }
                    Err(e) => {
                        println!("[{}] Error reading input: {}", adapter_id, e);

                        let mut stats = stats_clone.write().await;
                        stats.errors += 1;
                        drop(stats);

                        *running_clone.write().await = false;
                        break;
                    }
                }
            }

            println!("[{}] Console adapter stopped", adapter_id);
        });

        Ok(())
    }

    async fn do_stop(&mut self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        self.base.status = Arc::new(RwLock::new(AdapterStatus::Stopped));
        drop(running);

        println!("[{}] Console adapter stopped", self.base.config.adapter_id);
        Ok(())
    }

    async fn handle_custom(
        &mut self,
        _message_type: String,
        _payload: serde_json::Value,
    ) -> Result<serde_json::Value> {
        Err(LoquatError::Adapter(AdapterError::LoadFailed(
            "Custom messages not supported by console adapter".to_string(),
        )))
    }
}

/// Create a console adapter actor with message channel
pub async fn create_console_adapter_actor(
    config: AdapterConfig,
) -> Result<(mpsc::UnboundedSender<AdapterMessage>, ConsoleAdapterActor)> {
    let actor = ConsoleAdapterActor::new(config);

    let (tx, mut rx) = mpsc::unbounded_channel::<AdapterMessage>();
    let mut actor_clone = actor.clone();

    // Spawn actor task
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if let Err(e) = actor_clone.base.handle_message(message).await {
                eprintln!("Error handling message in console adapter actor: {}", e);
            }
        }
    });

    Ok((tx, actor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_console_adapter_actor_creation() {
        let config = AdapterConfig::new("console", "console-test-001", "stdio://");
        let actor = ConsoleAdapterActor::new(config.clone());

        assert_eq!(actor.base.name(), "ConsoleAdapter");
        assert_eq!(actor.base.version(), "1.0.0");
        assert_eq!(actor.base.config.adapter_id, config.adapter_id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_create_console_adapter_actor() {
        let config = AdapterConfig::new("console", "console-test-002", "stdio://");
        let (tx, actor) = create_console_adapter_actor(config)
            .await
            .expect("Failed to create console adapter actor");

        // Verify actor is created
        assert_eq!(actor.base.name(), "ConsoleAdapter");

        // Test status
        let (response_tx, response_rx) = oneshot::channel();
        tx.send(AdapterMessage::GetStatus {
            respond_to: response_tx,
        })
        .expect("Failed to send GetStatus message");

        let status = response_rx
            .await
            .expect("Failed to receive status response");
        assert_eq!(status, AdapterStatus::Ready);

        // Test start
        let (response_tx, response_rx) = oneshot::channel();
        tx.send(AdapterMessage::Start {
            respond_to: response_tx,
        })
        .expect("Failed to send Start message");

        let result = response_rx
            .await
            .expect("Failed to receive start response");
        assert!(result.is_ok());

        // Test status after start
        let (response_tx, response_rx) = oneshot::channel();
        tx.send(AdapterMessage::GetStatus {
            respond_to: response_tx,
        })
        .expect("Failed to send GetStatus message");

        let status = response_rx
            .await
            .expect("Failed to receive status response");
        assert_eq!(status, AdapterStatus::Running);

        // Test is_running
        let (response_tx, response_rx) = oneshot::channel();
        tx.send(AdapterMessage::IsRunning {
            respond_to: response_tx,
        })
        .expect("Failed to send IsRunning message");

        let is_running = response_rx
            .await
            .expect("Failed to receive is_running response");
        assert!(is_running);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_console_adapter_actor_statistics() {
        let config = AdapterConfig::new("console", "console-test-003", "stdio://");
        let (tx, _actor) = create_console_adapter_actor(config)
            .await
            .expect("Failed to create console adapter actor");

        // Get statistics
        let (response_tx, response_rx) = oneshot::channel();
        tx.send(AdapterMessage::GetStatistics {
            respond_to: response_tx,
        })
        .expect("Failed to send GetStatistics message");

        let stats = response_rx
            .await
            .expect("Failed to receive statistics response");
        assert_eq!(stats.events_received, 0);
        assert_eq!(stats.events_sent, 0);
        assert_eq!(stats.errors, 0);
    }
}
