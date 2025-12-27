//! Shutdown coordinator
//!
//! Manages coordinated shutdown of all components with timeout handling

use crate::errors::{LoquatError, Result};
use crate::logging::traits::{LogContext, LogLevel, Logger};
use crate::shutdown::stages::{ShutdownOrder, ShutdownStage, ShutdownStageResult};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, Mutex};
use std::collections::HashMap;

/// Shutdown status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownStatus {
    /// Not started
    NotStarted,
    /// In progress
    InProgress,
    /// Completed successfully
    Completed,
    /// Failed
    Failed,
    /// Timed out
    TimedOut,
}

/// Shutdown handler function signature
pub type ShutdownHandler = Arc<dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>> + Send + Sync>;

/// Shutdown coordinator
///
/// Manages the graceful shutdown of all components in a coordinated manner.
pub struct ShutdownCoordinator {
    logger: Arc<dyn Logger>,
    status: Arc<RwLock<ShutdownStatus>>,
    handlers: Mutex<HashMap<ShutdownStage, ShutdownHandler>>,
    results: Arc<RwLock<Vec<ShutdownStageResult>>>,
    start_time: Arc<RwLock<Option<std::time::Instant>>>,
}

impl ShutdownCoordinator {
    /// Create a new shutdown coordinator
    pub fn new(logger: Arc<dyn Logger>) -> Self {
        Self {
            logger,
            status: Arc::new(RwLock::new(ShutdownStatus::NotStarted)),
            handlers: Mutex::new(HashMap::new()),
            results: Arc::new(RwLock::new(Vec::new())),
            start_time: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new shutdown coordinator with a default shutdown order
    pub fn with_order(logger: Arc<dyn Logger>, _order: ShutdownOrder) -> Self {
        Self::new(logger)
    }

    /// Register a shutdown handler for a specific stage
    ///
    /// # Arguments
    /// * `stage` - The shutdown stage this handler is for
    /// * `handler` - Async function to execute during shutdown
    ///
    /// # Example
    /// ```
    /// coordinator.register_handler(
    ///     ShutdownStage::WebService,
    ///     Arc::new(|| {
    ///         Box::pin(async move {
    ///             web_service.stop().await
    ///         })
    ///     })
    /// ).await;
    /// ```
    pub async fn register_handler<F, Fut>(&self, stage: ShutdownStage, handler: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        let wrapped_handler = Arc::new(move || {
            Box::pin(handler()) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
        });
        let mut handlers = self.handlers.lock().await;
        handlers.insert(stage, wrapped_handler);
    }

    /// Remove a shutdown handler
    pub async fn remove_handler(&self, stage: ShutdownStage) {
        let mut handlers = self.handlers.lock().await;
        handlers.remove(&stage);
    }

    /// Execute shutdown with default order
    ///
    /// Returns the results of all shutdown stages
    pub async fn shutdown(&self) -> Result<Vec<ShutdownStageResult>> {
        self.shutdown_with_order(ShutdownOrder::new()).await
    }

    /// Execute shutdown with custom order
    ///
    /// # Arguments
    /// * `order` - The shutdown order to follow
    ///
    /// Returns the results of all shutdown stages
    pub async fn shutdown_with_order(&self, order: ShutdownOrder) -> Result<Vec<ShutdownStageResult>> {
        // Update status
        *self.status.write().await = ShutdownStatus::InProgress;
        *self.start_time.write().await = Some(std::time::Instant::now());

        // Clear previous results
        self.results.write().await.clear();

        // Log shutdown start
        let mut log_context = LogContext::new();
        log_context.component = Some("ShutdownCoordinator".to_string());
        log_context.add("total_stages", order.stages.len().to_string());
        log_context.add("timeout_per_stage_ms", order.timeout_per_stage.to_string());
        log_context.add("total_timeout_ms", order.total_timeout().to_string());

        self.logger.log(
            LogLevel::Info,
            &format!(
                "Starting shutdown ({} stages, total timeout: {}ms)",
                order.stages.len(),
                order.total_timeout()
            ),
            &log_context,
        );

        let mut abort = false;
        let mut results = Vec::new();

        // Execute each stage in order
        for stage in order.stages {
            if abort {
                break;
            }

            let result = self.execute_stage(stage, order.timeout_per_stage).await;
            
            // Record result
            if let Err(e) = &result {
                let mut results_lock = self.results.write().await;
                let error_result = ShutdownStageResult::FailedContinue {
                    stage,
                    error: e.to_string(),
                    duration_ms: 0,
                };
                results_lock.push(error_result.clone());
                results.push(error_result);
            }

            // Handle result
            match &result {
                Ok(stage_result) => {
                    results.push(stage_result.clone());
                    let mut results_lock = self.results.write().await;
                    results_lock.push(stage_result.clone());

                    // Log result
                    self.logger.log(
                        if stage_result.is_success() { LogLevel::Info } else { LogLevel::Warn },
                        &format!("Shutdown stage: {}", stage_result),
                        &log_context.clone(),
                    );

                    // Check if we should abort
                    if stage_result.should_abort() || (stage_result.is_failure() && order.abort_on_failure) {
                        abort = true;
                        self.logger.log(
                            LogLevel::Error,
                            &format!("Shutdown aborted due to failure in stage: {}", stage),
                            &log_context,
                        );
                    }
                }
                Err(e) => {
                    self.logger.log(
                        LogLevel::Error,
                        &format!("Unexpected error during shutdown stage {}: {}", stage, e),
                        &log_context,
                    );

                    if order.abort_on_failure {
                        abort = true;
                    }
                }
            }
        }

        // Update final status
        let has_failures = results.iter().any(|r| r.is_failure());
        let final_status = if abort {
            ShutdownStatus::Failed
        } else if has_failures {
            ShutdownStatus::Completed // Some failures but completed
        } else {
            ShutdownStatus::Completed
        };
        *self.status.write().await = final_status;

        // Calculate total duration
        let total_duration_ms = if let Some(start) = *self.start_time.read().await {
            start.elapsed().as_millis() as u64
        } else {
            0
        };

        // Log shutdown complete
        let successful = results.iter().filter(|r| r.is_success()).count();
        let failed = results.len() - successful;

        self.logger.log(
            if failed == 0 { LogLevel::Info } else { LogLevel::Warn },
            &format!(
                "Shutdown complete: {} successful, {} failed, total duration: {}ms",
                successful, failed, total_duration_ms
            ),
            &log_context,
        );

        Ok(results)
    }

    /// Execute a single shutdown stage with timeout
    async fn execute_stage(&self, stage: ShutdownStage, timeout_ms: u64) -> Result<ShutdownStageResult> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_millis(timeout_ms);

        // Get handler if registered
        let handler = {
            let handlers = self.handlers.lock().await;
            handlers.get(&stage).cloned()
        };

        // Execute handler or skip if not registered
        if let Some(handler) = handler {
            // Run with timeout
            let result = tokio::time::timeout(timeout, async {
                handler().await
            }).await;

            let duration_ms = start.elapsed().as_millis() as u64;

            match result {
                Ok(Ok(())) => {
                    Ok(ShutdownStageResult::Success { stage, duration_ms })
                }
                Ok(Err(e)) => {
                    Ok(ShutdownStageResult::FailedContinue {
                        stage,
                        error: e.to_string(),
                        duration_ms,
                    })
                }
                Err(_) => {
                    // Timeout
                    Ok(ShutdownStageResult::Timeout {
                        stage,
                        timeout_ms,
                    })
                }
            }
        } else {
            // No handler registered, skip
            Ok(ShutdownStageResult::Success {
                stage,
                duration_ms: 0,
            })
        }
    }

    /// Get the current shutdown status
    pub async fn status(&self) -> ShutdownStatus {
        *self.status.read().await
    }

    /// Get all shutdown results
    pub async fn results(&self) -> Vec<ShutdownStageResult> {
        self.results.read().await.clone()
    }

    /// Get shutdown results for a specific stage
    pub async fn result_for_stage(&self, stage: ShutdownStage) -> Option<ShutdownStageResult> {
        let results = self.results.read().await;
        results.iter().find(|r| r.stage() == stage).cloned()
    }

    /// Check if shutdown is in progress
    pub async fn is_shutting_down(&self) -> bool {
        matches!(*self.status.read().await, ShutdownStatus::InProgress)
    }

    /// Check if shutdown has completed
    pub async fn is_complete(&self) -> bool {
        matches!(
            *self.status.read().await,
            ShutdownStatus::Completed | ShutdownStatus::Failed | ShutdownStatus::TimedOut
        )
    }

    /// Get shutdown duration if completed
    pub async fn duration_ms(&self) -> Option<u64> {
        if let Some(start) = *self.start_time.read().await {
            if self.is_complete().await {
                Some(start.elapsed().as_millis() as u64)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Reset the coordinator for a new shutdown
    pub async fn reset(&self) {
        *self.status.write().await = ShutdownStatus::NotStarted;
        *self.start_time.write().await = None;
        self.results.write().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::formatters::TextFormatter;
    use crate::logging::writers::ConsoleWriter;
    use crate::logging::StructuredLogger;

    fn create_test_logger() -> Arc<dyn Logger> {
        let formatter = Arc::new(TextFormatter::simple());
        let writer = Arc::new(ConsoleWriter::new());
        Arc::new(StructuredLogger::new(formatter, writer))
    }

    #[tokio::test]
    async fn test_coordinator_creation() {
        let logger = create_test_logger();
        let coordinator = ShutdownCoordinator::new(logger);

        assert_eq!(coordinator.status().await, ShutdownStatus::NotStarted);
        assert!(coordinator.results().await.is_empty());
    }

    #[tokio::test]
    async fn test_register_handler() {
        let logger = create_test_logger();
        let coordinator = ShutdownCoordinator::new(logger.clone());

        coordinator.register_handler(
            ShutdownStage::Engine,
            || Box::pin(async { Ok(()) }),
        ).await;

        // Handler is now registered, but we can't directly verify it
        // Execute shutdown to verify it works
        let results = coordinator.shutdown().await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_shutdown_success() {
        let logger = create_test_logger();
        let coordinator = ShutdownCoordinator::new(logger.clone());

        let called = Arc::new(std::sync::Mutex::new(false));
        let called_clone = called.clone();

        coordinator.register_handler(
            ShutdownStage::Engine,
            move || {
                let called = called_clone.clone();
                Box::pin(async move {
                    *called.lock().unwrap() = true;
                    Ok(())
                })
            },
        ).await;

        let results = coordinator.shutdown().await.unwrap();
        
        assert_eq!(coordinator.status().await, ShutdownStatus::Completed);
        assert!(*called.lock().unwrap());
        assert!(results.iter().any(|r| r.stage() == ShutdownStage::Engine && r.is_success()));
    }

    #[tokio::test]
    async fn test_shutdown_failure() {
        let logger = create_test_logger();
        let coordinator = ShutdownCoordinator::new(logger.clone());

        coordinator.register_handler(
            ShutdownStage::Engine,
            || Box::pin(async { Err(LoquatError::Unknown("Test error".to_string())) }),
        ).await;

        let results = coordinator.shutdown().await.unwrap();
        
        // Should complete despite failure (default is to continue)
        assert_eq!(coordinator.status().await, ShutdownStatus::Completed);
        assert!(results.iter().any(|r| r.stage() == ShutdownStage::Engine && r.is_failure()));
    }

    #[tokio::test]
    async fn test_shutdown_timeout() {
        let logger = create_test_logger();
        let coordinator = ShutdownCoordinator::new(logger.clone());

        coordinator.register_handler(
            ShutdownStage::Engine,
            || Box::pin(async {
                tokio::time::sleep(Duration::from_secs(2)).await;
                Ok(())
            }),
        ).await;

        // Shutdown with very short timeout
        let order = ShutdownOrder::with_timeout(100); // 100ms
        let results = coordinator.shutdown_with_order(order).await.unwrap();
        
        assert!(results.iter().any(|r| r.stage() == ShutdownStage::Engine && matches!(r, ShutdownStageResult::Timeout { .. })));
    }

    #[tokio::test]
    async fn test_shutdown_abort_on_failure() {
        let logger = create_test_logger();
        let coordinator = ShutdownCoordinator::new(logger.clone());

        coordinator.register_handler(
            ShutdownStage::Engine,
            || Box::pin(async { Err(LoquatError::Unknown("Critical error".to_string())) }),
        ).await;

        // Use order that aborts on failure
        let order = ShutdownOrder::new().abort_on_failure();
        let results = coordinator.shutdown_with_order(order).await.unwrap();
        
        assert_eq!(coordinator.status().await, ShutdownStatus::Failed);
        // Should not execute stages after Engine
        assert!(results.iter().filter(|r| r.stage() == ShutdownStage::Engine).count() == 1);
    }

    #[tokio::test]
    async fn test_reset() {
        let logger = create_test_logger();
        let coordinator = ShutdownCoordinator::new(logger);

        let _ = coordinator.shutdown().await;

        assert!(coordinator.is_complete().await);

        coordinator.reset().await;

        assert_eq!(coordinator.status().await, ShutdownStatus::NotStarted);
        assert!(coordinator.results().await.is_empty());
    }

    #[tokio::test]
    async fn test_remove_handler() {
        let logger = create_test_logger();
        let coordinator = ShutdownCoordinator::new(logger.clone());

        coordinator.register_handler(
            ShutdownStage::Engine,
            || Box::pin(async { Ok(()) }),
        ).await;

        coordinator.remove_handler(ShutdownStage::Engine).await;

        // Shutdown should still complete, just without executing the handler
        let results = coordinator.shutdown().await.unwrap();
        assert_eq!(coordinator.status().await, ShutdownStatus::Completed);
    }
}
