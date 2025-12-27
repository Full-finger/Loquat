//! Adapter state management
//!
//! Provides real-time tracking and management of adapter states
//! with support for health checks and status notifications.

use crate::adapters::status::AdapterStatus;
use crate::logging::traits::{LogContext, LogLevel, Logger};
use std::sync::Arc;
use tokio::sync::RwLock;

/// State transition history entry
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub from: AdapterStatus,
    pub to: AdapterStatus,
    pub timestamp: i64,
    pub reason: String,
}

/// Adapter state manager
///
/// Manages the current state of an adapter, tracks state transitions,
/// and provides health check functionality.
#[derive(Clone)]
pub struct AdapterStateManager {
    adapter_id: String,
    state: Arc<RwLock<AdapterStatus>>,
    history: Arc<RwLock<Vec<StateTransition>>>,
    max_history_size: usize,
    logger: Arc<dyn Logger>,
}

impl AdapterStateManager {
    /// Create a new adapter state manager
    pub fn new(adapter_id: String, logger: Arc<dyn Logger>) -> Self {
        Self {
            adapter_id,
            state: Arc::new(RwLock::new(AdapterStatus::Uninitialized)),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history_size: 100,
            logger,
        }
    }

    /// Create a new adapter state manager with custom history size
    pub fn with_history_size(
        adapter_id: String,
        max_history_size: usize,
        logger: Arc<dyn Logger>,
    ) -> Self {
        Self {
            adapter_id,
            state: Arc::new(RwLock::new(AdapterStatus::Uninitialized)),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history_size,
            logger,
        }
    }

    /// Get the current state
    pub async fn get_state(&self) -> AdapterStatus {
        self.state.read().await.clone()
    }

    /// Set the state with a reason
    pub async fn set_state(&self, new_state: AdapterStatus, reason: &str) {
        let mut state = self.state.write().await;
        let old_state = state.clone();
        
        if old_state != new_state {
            *state = new_state.clone();
            
            // Record transition
            let transition = StateTransition {
                from: old_state.clone(),
                to: new_state.clone(),
                timestamp: chrono::Utc::now().timestamp_millis(),
                reason: reason.to_string(),
            };
            
            self.record_transition(transition).await;
            
            // Log state change
            let mut log_context = LogContext::new();
            log_context.component = Some("AdapterStateManager".to_string());
            log_context.add("adapter_id", self.adapter_id.clone());
            log_context.add("old_state", format!("{:?}", old_state));
            log_context.add("new_state", format!("{:?}", new_state));
            log_context.add("reason", reason.to_string());
            
            self.logger.log(
                LogLevel::Info,
                &format!("Adapter {} state changed: {:?} -> {:?}", self.adapter_id, old_state, new_state),
                &log_context,
            );
        }
    }

    /// Check if the adapter is in a specific state
    pub async fn is_state(&self, status: AdapterStatus) -> bool {
        self.get_state().await == status
    }

    /// Check if the adapter is running
    pub async fn is_running(&self) -> bool {
        matches!(self.get_state().await, AdapterStatus::Running)
    }

    /// Check if the adapter is ready
    pub async fn is_ready(&self) -> bool {
        matches!(self.get_state().await, AdapterStatus::Ready)
    }

    /// Check if the adapter is healthy
    pub async fn is_healthy(&self) -> bool {
        let state = self.get_state().await;
        state.is_active()
    }

    /// Get state transition history
    pub async fn get_history(&self) -> Vec<StateTransition> {
        let history = self.history.read().await;
        history.clone()
    }

    /// Get recent state transitions
    pub async fn get_recent_history(&self, limit: usize) -> Vec<StateTransition> {
        let history = self.history.read().await;
        let len = history.len();
        if len <= limit {
            history.clone()
        } else {
            history[len - limit..].to_vec()
        }
    }

    /// Get the number of state transitions
    pub async fn transition_count(&self) -> usize {
        self.history.read().await.len()
    }

    /// Perform a health check
    ///
    /// Returns true if the adapter is healthy, false otherwise
    pub async fn health_check(&self) -> bool {
        let state = self.get_state().await;
        
        // Basic health check: adapter must be running or connected
        if !self.is_healthy().await {
            let mut log_context = LogContext::new();
            log_context.component = Some("AdapterStateManager".to_string());
            log_context.add("adapter_id", self.adapter_id.clone());
            log_context.add("state", format!("{:?}", state));
            
            self.logger.log(
                LogLevel::Warn,
                &format!("Health check failed for adapter {}: not healthy", self.adapter_id),
                &log_context,
            );
            
            return false;
        }
        
        true
    }

    /// Get health status description
    pub async fn health_status(&self) -> String {
        let state = self.get_state().await;
        
        match state {
            AdapterStatus::Running => "Healthy".to_string(),
            AdapterStatus::Ready => "Ready".to_string(),
            AdapterStatus::Paused => "Paused".to_string(),
            AdapterStatus::Stopped => "Stopped".to_string(),
            AdapterStatus::Initializing => "Initializing".to_string(),
            AdapterStatus::Uninitialized => "Uninitialized".to_string(),
            AdapterStatus::Error(msg) => format!("Error: {}", msg),
        }
    }

    /// Reset to uninitialized state
    pub async fn reset(&self) {
        self.set_state(AdapterStatus::Uninitialized, "State manager reset").await;
    }

    /// Record a state transition
    async fn record_transition(&self, transition: StateTransition) {
        let mut history = self.history.write().await;
        
        // Add new transition
        history.push(transition);
        
        // Trim history if needed
        if history.len() > self.max_history_size {
            let remove_count = history.len() - self.max_history_size;
            history.drain(0..remove_count);
        }
    }

    /// Clear state history
    pub async fn clear_history(&self) {
        let mut history = self.history.write().await;
        history.clear();
    }

    /// Get state statistics
    pub async fn get_stats(&self) -> AdapterStateStats {
        let state = self.get_state().await;
        let history = self.history.read().await;
        
        let mut state_counts = std::collections::HashMap::new();
        for transition in history.iter() {
            *state_counts.entry(format!("{:?}", transition.to)).or_insert(0) += 1;
        }
        
        AdapterStateStats {
            current_state: state,
            transition_count: history.len(),
            state_counts,
        }
    }
}

/// Adapter state statistics
#[derive(Debug, Clone)]
pub struct AdapterStateStats {
    pub current_state: AdapterStatus,
    pub transition_count: usize,
    pub state_counts: std::collections::HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_logger() -> Arc<dyn crate::logging::Logger> {
        let formatter = Arc::new(crate::logging::formatters::JsonFormatter::new());
        let writer = Arc::new(crate::logging::writers::ConsoleWriter::new());
        Arc::new(crate::logging::StructuredLogger::new(formatter, writer))
    }

    #[tokio::test]
    async fn test_state_manager_creation() {
        let logger = create_test_logger();
        let manager = AdapterStateManager::new("test_adapter".to_string(), logger);
        
        assert_eq!(manager.get_state().await, AdapterStatus::Uninitialized);
        assert_eq!(manager.transition_count().await, 0);
    }

    #[tokio::test]
    async fn test_state_transition() {
        let logger = create_test_logger();
        let manager = AdapterStateManager::new("test_adapter".to_string(), logger);
        
        manager.set_state(AdapterStatus::Running, "Starting adapter").await;
        
        assert_eq!(manager.get_state().await, AdapterStatus::Running);
        assert_eq!(manager.transition_count().await, 1);
    }

    #[tokio::test]
    async fn test_state_is_running() {
        let logger = create_test_logger();
        let manager = AdapterStateManager::new("test_adapter".to_string(), logger);
        
        assert!(!manager.is_running().await);
        
        manager.set_state(AdapterStatus::Running, "Starting adapter").await;
        assert!(manager.is_running().await);
    }

    #[tokio::test]
    async fn test_state_is_ready() {
        let logger = create_test_logger();
        let manager = AdapterStateManager::new("test_adapter".to_string(), logger);
        
        assert!(!manager.is_ready().await);
        
        manager.set_state(AdapterStatus::Ready, "Adapter ready").await;
        assert!(manager.is_ready().await);
    }

    #[tokio::test]
    async fn test_state_is_healthy() {
        let logger = create_test_logger();
        let manager = AdapterStateManager::new("test_adapter".to_string(), logger);
        
        assert!(!manager.is_healthy().await);
        
        manager.set_state(AdapterStatus::Running, "Adapter running").await;
        assert!(manager.is_healthy().await);
    }

    #[tokio::test]
    async fn test_health_check() {
        let logger = create_test_logger();
        let manager = AdapterStateManager::new("test_adapter".to_string(), logger);
        
        assert!(!manager.health_check().await);
        
        manager.set_state(AdapterStatus::Running, "Adapter running").await;
        assert!(manager.health_check().await);
    }

    #[tokio::test]
    async fn test_state_history() {
        let logger = create_test_logger();
        let manager = AdapterStateManager::new("test_adapter".to_string(), logger);
        
        manager.set_state(AdapterStatus::Initializing, "Initializing").await;
        manager.set_state(AdapterStatus::Ready, "Ready").await;
        manager.set_state(AdapterStatus::Running, "Running").await;
        
        let history = manager.get_history().await;
        assert_eq!(history.len(), 3);
        
        let recent = manager.get_recent_history(2).await;
        assert_eq!(recent.len(), 2);
    }

    #[tokio::test]
    async fn test_state_history_limit() {
        let logger = create_test_logger();
        let manager = AdapterStateManager::with_history_size("test_adapter".to_string(), 5, logger);
        
        for i in 0..10 {
            manager.set_state(AdapterStatus::Running, &format!("Attempt {}", i)).await;
        }
        
        assert!(manager.transition_count().await <= 5);
    }

    #[tokio::test]
    async fn test_reset() {
        let logger = create_test_logger();
        let manager = AdapterStateManager::new("test_adapter".to_string(), logger);
        
        manager.set_state(AdapterStatus::Running, "Starting").await;
        manager.reset().await;
        
        assert_eq!(manager.get_state().await, AdapterStatus::Uninitialized);
    }

    #[tokio::test]
    async fn test_clear_history() {
        let logger = create_test_logger();
        let manager = AdapterStateManager::new("test_adapter".to_string(), logger);
        
        manager.set_state(AdapterStatus::Running, "Starting").await;
        manager.clear_history().await;
        
        assert_eq!(manager.transition_count().await, 0);
    }

    #[tokio::test]
    async fn test_health_status() {
        let logger = create_test_logger();
        let manager = AdapterStateManager::new("test_adapter".to_string(), logger);
        
        let status = manager.health_status().await;
        assert_eq!(status, "Uninitialized");
        
        manager.set_state(AdapterStatus::Running, "Starting").await;
        let status = manager.health_status().await;
        assert_eq!(status, "Healthy");
    }

    #[tokio::test]
    async fn test_get_stats() {
        let logger = create_test_logger();
        let manager = AdapterStateManager::new("test_adapter".to_string(), logger);
        
        manager.set_state(AdapterStatus::Running, "Starting").await;
        manager.set_state(AdapterStatus::Ready, "Ready").await;
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.current_state, AdapterStatus::Ready);
        assert_eq!(stats.transition_count, 2);
        assert!(stats.state_counts.contains_key(&"Running".to_string()));
        assert!(stats.state_counts.contains_key(&"Ready".to_string()));
    }
}
