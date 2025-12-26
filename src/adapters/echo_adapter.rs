//! Echo Adapter - echoes back received messages

use crate::adapters::{
    Adapter, AdapterConfig, AdapterStatus,
    types::AdapterStatistics,
};
use crate::errors::{AdapterError, LoquatError, Result};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Echo adapter implementation
#[derive(Debug)]
pub struct EchoAdapter {
    config: AdapterConfig,
    status: Arc<RwLock<AdapterStatus>>,
    statistics: Arc<RwLock<AdapterStatistics>>,
    running: Arc<RwLock<bool>>,
}

impl EchoAdapter {
    /// Create a new echo adapter
    pub fn new(config: AdapterConfig) -> Self {
        Self {
            config,
            status: Arc::new(RwLock::new(AdapterStatus::Ready)),
            statistics: Arc::new(RwLock::new(AdapterStatistics::default())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Echo a message back
    pub async fn echo(&self, message: &str) -> String {
        // Update statistics
        let mut stats = self.statistics.write().await;
        stats.events_received += 1;
        stats.messages_sent += 1;
        stats.last_activity = Some(chrono::Utc::now().timestamp());
        drop(stats);

        format!("Echo: {}", message)
    }

    /// Start the echo adapter
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

        Ok(())
    }

    /// Stop the echo adapter
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        *self.status.write().await = AdapterStatus::Stopped;
        drop(running);

        Ok(())
    }
}

impl Adapter for EchoAdapter {
    fn name(&self) -> &str {
        "EchoAdapter"
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
    async fn test_echo_adapter_creation() {
        let config = AdapterConfig::new("echo", "echo-test-001", "echo://");
        let adapter = EchoAdapter::new(config);

        assert_eq!(adapter.name(), "EchoAdapter");
        assert_eq!(adapter.version(), "1.0.0");
        assert_eq!(adapter.adapter_id(), "echo-test-001");
        assert_eq!(adapter.status(), AdapterStatus::Ready);
        assert!(!adapter.is_running());
    }

    #[test]
    fn test_echo_adapter_config() {
        let config = AdapterConfig::new("echo", "echo-test-002", "echo://");
        let adapter = EchoAdapter::new(config.clone());

        assert_eq!(adapter.config().adapter_id, config.adapter_id);
        assert_eq!(adapter.config().adapter_type, config.adapter_type);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_echo_adapter_statistics() {
        let config = AdapterConfig::new("echo", "echo-test-003", "echo://");
        let adapter = EchoAdapter::new(config);

        let stats = adapter.statistics();
        assert_eq!(stats.events_received, 0);
        assert_eq!(stats.events_sent, 0);
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.errors, 0);
    }
}
