//! Console Adapter - reads input from stdin and writes output to stdout

use crate::adapters::{
    Adapter, AdapterConfig, AdapterStatus,
    types::AdapterStatistics,
};
use crate::events::EventEnum;
use crate::errors::{AdapterError, LoquatError, Result};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::io::{AsyncBufReadExt, BufReader};

/// Console adapter implementation
#[derive(Debug)]
pub struct ConsoleAdapter {
    config: AdapterConfig,
    status: Arc<RwLock<AdapterStatus>>,
    statistics: Arc<RwLock<AdapterStatistics>>,
    running: Arc<RwLock<bool>>,
    event_sender: Option<mpsc::UnboundedSender<EventEnum>>,
}

impl ConsoleAdapter {
    /// Create a new console adapter
    pub fn new(config: AdapterConfig) -> Self {
        Self {
            config,
            status: Arc::new(RwLock::new(AdapterStatus::Ready)),
            statistics: Arc::new(RwLock::new(AdapterStatistics::default())),
            running: Arc::new(RwLock::new(false)),
            event_sender: None,
        }
    }

    /// Set the event sender channel
    pub fn set_event_sender(&mut self, sender: mpsc::UnboundedSender<EventEnum>) {
        self.event_sender = Some(sender);
    }

    /// Start the console adapter
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(LoquatError::Adapter(AdapterError::LoadFailed(
                "Adapter is already running".to_string()
            )));
        }

        *running = true;
        *self.status.write().await = AdapterStatus::Running;
        drop(running);

        // Spawn stdin reader task
        let running_clone = Arc::clone(&self.running);
        let status_clone = Arc::clone(&self.status);
        let stats_clone = Arc::clone(&self.statistics);
        let sender_clone = self.event_sender.clone();
        let adapter_id = self.config.adapter_id.clone();

        tokio::spawn(async move {
            let stdin = tokio::io::stdin();
            let reader = BufReader::new(stdin);
            let mut lines = reader.lines();

            println!("[{}] Console adapter started. Type messages and press Enter to send.", adapter_id);
            println!("[{}] Type 'quit' or 'exit' to stop the adapter.", adapter_id);

            while *running_clone.read().await {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        let line = line.trim();
                        
                        // Check for quit command
                        if line.eq_ignore_ascii_case("quit") || line.eq_ignore_ascii_case("exit") {
                            println!("[{}] Stopping adapter...", adapter_id);
                            *running_clone.write().await = false;
                            *status_clone.write().await = AdapterStatus::Stopped;
                            break;
                        }

                        // Create a simple message event
                        // This is a simplified event - in a real implementation,
                        // you would create proper event structures
                        println!("[{}] Received: {}", adapter_id, line);
                        
                        // Update statistics
                        let mut stats = stats_clone.write().await;
                        stats.events_received += 1;
                        stats.last_activity = Some(chrono::Utc::now().timestamp());
                        drop(stats);

                        // Send event if channel exists
                        if let Some(ref sender) = sender_clone {
                            // For now, we'll just log that we would send an event
                            // In a full implementation, we would construct and send EventEnum::Message
                            println!("[{}] Event would be sent to event system", adapter_id);
                        }
                    }
                    Ok(None) => {
                        // EOF reached
                        println!("[{}] End of input", adapter_id);
                        *running_clone.write().await = false;
                        *status_clone.write().await = AdapterStatus::Stopped;
                        break;
                    }
                    Err(e) => {
                        println!("[{}] Error reading input: {}", adapter_id, e);
                        
                        let mut stats = stats_clone.write().await;
                        stats.errors += 1;
                        drop(stats);
                        
                        *running_clone.write().await = false;
                        *status_clone.write().await = AdapterStatus::Error(e.to_string());
                        break;
                    }
                }
            }

            println!("[{}] Console adapter stopped", adapter_id);
        });

        Ok(())
    }

    /// Stop the console adapter
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        *self.status.write().await = AdapterStatus::Stopped;
        drop(running);

        Ok(())
    }
}

impl Adapter for ConsoleAdapter {
    fn name(&self) -> &str {
        "ConsoleAdapter"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn adapter_id(&self) -> &str {
        &self.config.adapter_id
    }

    fn config(&self) -> AdapterConfig {
        self.config.clone()
    }

    fn status(&self) -> AdapterStatus {
        // Use blocking read for synchronous method
        tokio::task::block_in_place(|| {
            let guard = tokio::runtime::Handle::current()
                .block_on(self.status.read());
            guard.clone()
        })
    }

    fn is_running(&self) -> bool {
        self.status() == AdapterStatus::Running
    }

    fn is_connected(&self) -> bool {
        self.status().is_active()
    }

    fn statistics(&self) -> AdapterStatistics {
        // Use blocking read for synchronous method
        tokio::task::block_in_place(|| {
            let guard = tokio::runtime::Handle::current()
                .block_on(self.statistics.read());
            guard.clone()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_console_adapter_creation() {
        let config = AdapterConfig::new("console", "console-test-001", "stdio://");
        let adapter = ConsoleAdapter::new(config);

        assert_eq!(adapter.name(), "ConsoleAdapter");
        assert_eq!(adapter.version(), "1.0.0");
        assert_eq!(adapter.adapter_id(), "console-test-001");
        assert_eq!(adapter.status(), AdapterStatus::Ready);
        assert!(!adapter.is_running());
    }

    #[test]
    fn test_console_adapter_config() {
        let config = AdapterConfig::new("console", "console-test-002", "stdio://");
        let adapter = ConsoleAdapter::new(config.clone());

        assert_eq!(adapter.config().adapter_id, config.adapter_id);
        assert_eq!(adapter.config().adapter_type, config.adapter_type);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_console_adapter_statistics() {
        let config = AdapterConfig::new("console", "console-test-003", "stdio://");
        let adapter = ConsoleAdapter::new(config);

        let stats = adapter.statistics();
        assert_eq!(stats.events_received, 0);
        assert_eq!(stats.events_sent, 0);
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.errors, 0);
    }
}
