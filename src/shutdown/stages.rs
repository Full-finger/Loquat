//! Shutdown stages and ordering
//!
//! Defines the stages and order of operations for graceful shutdown

use serde::{Deserialize, Serialize};
use std::fmt;

/// Shutdown stages in order of execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShutdownStage {
    /// Stop accepting new requests
    StopAcceptingRequests,
    /// Stop web service
    WebService,
    /// Stop adapter hot reload
    AdapterHotReload,
    /// Stop plugin hot reload
    PluginHotReload,
    /// Unload adapters
    Adapters,
    /// Unload plugins
    Plugins,
    /// Stop workers
    Workers,
    /// Stop channels
    Channels,
    /// Stop engine
    Engine,
    /// Flush and close logs
    Logging,
}

impl fmt::Display for ShutdownStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShutdownStage::StopAcceptingRequests => write!(f, "Stop Accepting Requests"),
            ShutdownStage::WebService => write!(f, "Web Service"),
            ShutdownStage::AdapterHotReload => write!(f, "Adapter Hot Reload"),
            ShutdownStage::PluginHotReload => write!(f, "Plugin Hot Reload"),
            ShutdownStage::Adapters => write!(f, "Adapters"),
            ShutdownStage::Plugins => write!(f, "Plugins"),
            ShutdownStage::Workers => write!(f, "Workers"),
            ShutdownStage::Channels => write!(f, "Channels"),
            ShutdownStage::Engine => write!(f, "Engine"),
            ShutdownStage::Logging => write!(f, "Logging"),
        }
    }
}

/// Result of a shutdown stage execution
#[derive(Debug, Clone)]
pub enum ShutdownStageResult {
    /// Stage completed successfully
    Success {
        stage: ShutdownStage,
        duration_ms: u64,
    },
    /// Stage failed but shutdown should continue
    FailedContinue {
        stage: ShutdownStage,
        error: String,
        duration_ms: u64,
    },
    /// Stage failed and shutdown should abort
    FailedAbort {
        stage: ShutdownStage,
        error: String,
        duration_ms: u64,
    },
    /// Stage timed out
    Timeout {
        stage: ShutdownStage,
        timeout_ms: u64,
    },
}

impl ShutdownStageResult {
    /// Check if the stage was successful
    pub fn is_success(&self) -> bool {
        matches!(self, ShutdownStageResult::Success { .. })
    }

    /// Check if the stage failed
    pub fn is_failure(&self) -> bool {
        matches!(self, ShutdownStageResult::FailedContinue { .. } 
                 | ShutdownStageResult::FailedAbort { .. } 
                 | ShutdownStageResult::Timeout { .. })
    }

    /// Check if the failure should abort shutdown
    pub fn should_abort(&self) -> bool {
        matches!(self, ShutdownStageResult::FailedAbort { .. })
    }

    /// Get the stage for this result
    pub fn stage(&self) -> ShutdownStage {
        match self {
            ShutdownStageResult::Success { stage, .. } => *stage,
            ShutdownStageResult::FailedContinue { stage, .. } => *stage,
            ShutdownStageResult::FailedAbort { stage, .. } => *stage,
            ShutdownStageResult::Timeout { stage, .. } => *stage,
        }
    }

    /// Get the duration in milliseconds
    pub fn duration_ms(&self) -> Option<u64> {
        match self {
            ShutdownStageResult::Success { duration_ms, .. } => Some(*duration_ms),
            ShutdownStageResult::FailedContinue { duration_ms, .. } => Some(*duration_ms),
            ShutdownStageResult::FailedAbort { duration_ms, .. } => Some(*duration_ms),
            ShutdownStageResult::Timeout { .. } => None,
        }
    }

    /// Get the error message if any
    pub fn error(&self) -> Option<&str> {
        match self {
            ShutdownStageResult::FailedContinue { error, .. } => Some(error),
            ShutdownStageResult::FailedAbort { error, .. } => Some(error),
            _ => None,
        }
    }
}

impl fmt::Display for ShutdownStageResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShutdownStageResult::Success { stage, duration_ms } => {
                write!(f, "{}: SUCCESS ({}ms)", stage, duration_ms)
            }
            ShutdownStageResult::FailedContinue { stage, error, duration_ms } => {
                write!(f, "{}: FAILED_CONTINUE - {} ({}ms)", stage, error, duration_ms)
            }
            ShutdownStageResult::FailedAbort { stage, error, duration_ms } => {
                write!(f, "{}: FAILED_ABORT - {} ({}ms)", stage, error, duration_ms)
            }
            ShutdownStageResult::Timeout { stage, timeout_ms } => {
                write!(f, "{}: TIMEOUT (exceeded {}ms)", stage, timeout_ms)
            }
        }
    }
}

/// Default shutdown order
#[derive(Debug, Clone, Default)]
pub struct ShutdownOrder {
    pub stages: Vec<ShutdownStage>,
    pub timeout_per_stage: u64,
    pub abort_on_failure: bool,
}

impl ShutdownOrder {
    /// Create a new shutdown order with default stages
    pub fn new() -> Self {
        Self {
            stages: vec![
                ShutdownStage::StopAcceptingRequests,
                ShutdownStage::WebService,
                ShutdownStage::AdapterHotReload,
                ShutdownStage::PluginHotReload,
                ShutdownStage::Adapters,
                ShutdownStage::Plugins,
                ShutdownStage::Workers,
                ShutdownStage::Channels,
                ShutdownStage::Engine,
                ShutdownStage::Logging,
            ],
            timeout_per_stage: 5000, // 5 seconds per stage
            abort_on_failure: false,
        }
    }

    /// Create a new shutdown order with custom timeout
    pub fn with_timeout(timeout_ms: u64) -> Self {
        Self {
            timeout_per_stage: timeout_ms,
            ..Self::new()
        }

    }

    /// Create a new shutdown order that aborts on failure
    pub fn abort_on_failure(mut self) -> Self {
        self.abort_on_failure = true;
        self
    }

    /// Add a custom stage
    pub fn add_stage(mut self, stage: ShutdownStage) -> Self {
        self.stages.push(stage);
        self
    }

    /// Remove a stage
    pub fn remove_stage(mut self, stage: ShutdownStage) -> Self {
        self.stages.retain(|s| s != &stage);
        self
    }

    /// Get total timeout for all stages
    pub fn total_timeout(&self) -> u64 {
        self.stages.len() as u64 * self.timeout_per_stage
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_stage_display() {
        assert_eq!(
            format!("{}", ShutdownStage::WebService),
            "Web Service"
        );
        assert_eq!(
            format!("{}", ShutdownStage::Engine),
            "Engine"
        );
    }

    #[test]
    fn test_shutdown_stage_result_success() {
        let result = ShutdownStageResult::Success {
            stage: ShutdownStage::Engine,
            duration_ms: 100,
        };
        assert!(result.is_success());
        assert!(!result.is_failure());
        assert_eq!(result.stage(), ShutdownStage::Engine);
        assert_eq!(result.duration_ms(), Some(100));
        assert!(result.error().is_none());
    }

    #[test]
    fn test_shutdown_stage_result_failure() {
        let result = ShutdownStageResult::FailedContinue {
            stage: ShutdownStage::Engine,
            error: "Test error".to_string(),
            duration_ms: 50,
        };
        assert!(!result.is_success());
        assert!(result.is_failure());
        assert!(!result.should_abort());
        assert_eq!(result.stage(), ShutdownStage::Engine);
        assert_eq!(result.duration_ms(), Some(50));
        assert_eq!(result.error(), Some("Test error"));
    }

    #[test]
    fn test_shutdown_stage_result_abort() {
        let result = ShutdownStageResult::FailedAbort {
            stage: ShutdownStage::Engine,
            error: "Critical error".to_string(),
            duration_ms: 30,
        };
        assert!(!result.is_success());
        assert!(result.is_failure());
        assert!(result.should_abort());
        assert_eq!(result.stage(), ShutdownStage::Engine);
    }

    #[test]
    fn test_shutdown_stage_result_timeout() {
        let result = ShutdownStageResult::Timeout {
            stage: ShutdownStage::Engine,
            timeout_ms: 5000,
        };
        assert!(!result.is_success());
        assert!(result.is_failure());
        assert!(!result.should_abort());
        assert_eq!(result.stage(), ShutdownStage::Engine);
        assert_eq!(result.duration_ms(), None);
        assert!(result.error().is_none());
    }

    #[test]
    fn test_shutdown_order_default() {
        let order = ShutdownOrder::new();
        assert_eq!(order.stages.len(), 10);
        assert_eq!(order.timeout_per_stage, 5000);
        assert!(!order.abort_on_failure);
    }

    #[test]
    fn test_shutdown_order_custom_timeout() {
        let order = ShutdownOrder::with_timeout(10000);
        assert_eq!(order.timeout_per_stage, 10000);
        assert_eq!(order.total_timeout(), 100000);
    }

    #[test]
    fn test_shutdown_order_add_remove_stage() {
        let order = ShutdownOrder::new()
            .add_stage(ShutdownStage::WebService)
            .remove_stage(ShutdownStage::PluginHotReload);
        
        // Should have removed one and added one (no duplicate)
        assert_eq!(order.stages.len(), 10);
        assert!(order.stages.contains(&ShutdownStage::WebService));
        assert!(!order.stages.contains(&ShutdownStage::PluginHotReload));
    }

    #[test]
    fn test_shutdown_order_abort_on_failure() {
        let order = ShutdownOrder::new().abort_on_failure();
        assert!(order.abort_on_failure);
    }
}
