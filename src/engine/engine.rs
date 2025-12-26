//! Standard Loquat Engine implementation

use crate::channel_manager::{StandardChannelManager, ChannelManager as _};
use crate::channels::types::ChannelType;
use crate::engine::types::{EngineConfig, EngineStats, EngineState, ProcessingContext, EngineStatus};
use crate::engine::traits::Engine;
use crate::errors::{LoquatError, Result};
use crate::events::Package;
use crate::logging::traits::{LogContext, LogLevel, Logger};
use crate::routers::{Router, StandardRouter};
use crate::streams::Stream;
use async_trait::async_trait;
use std::sync::Arc;

/// Standard Loquat Engine - core coordinator
pub struct StandardEngine {
    config: EngineConfig,
    stats: EngineStats,
    state: Arc<tokio::sync::RwLock<EngineState>>,
    router: Arc<StandardRouter>,
    channel_manager: Arc<StandardChannelManager>,
    logger: Arc<dyn Logger>,
}

impl std::fmt::Debug for StandardEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StandardEngine")
            .field("config", &self.config)
            .field("stats", &self.stats)
            .field("state", &self.state)
            .finish()
    }
}

impl StandardEngine {
    pub fn new(logger: Arc<dyn Logger>) -> Self {
        let logger_clone = logger.clone();
        Self {
            config: EngineConfig::new(),
            stats: EngineStats::new(),
            state: Arc::new(tokio::sync::RwLock::new(EngineState {
                status: EngineStatus::Idle,
                last_error: None,
            })),
            router: Arc::new(StandardRouter::new(logger_clone.clone())),
            channel_manager: Arc::new(StandardChannelManager::new(logger_clone)),
            logger,
        }
    }
    
    pub fn with_config(config: EngineConfig, logger: Arc<dyn Logger>) -> Self {
        let logger_clone = logger.clone();
        Self {
            config,
            stats: EngineStats::new(),
            state: Arc::new(tokio::sync::RwLock::new(EngineState {
                status: EngineStatus::Idle,
                last_error: None,
            })),
            router: Arc::new(StandardRouter::new(logger_clone.clone())),
            channel_manager: Arc::new(StandardChannelManager::new(logger_clone)),
            logger,
        }
    }
    
    async fn get_processing_context(&self, package: &Package) -> Result<ProcessingContext> {
        let mut context = ProcessingContext::new();
        
        if self.config.auto_route {
            let route_result = self.router.route_package(package).await;
            context.route_target = Some(route_result.state.adapter_target.clone());
            
            let message = format!(
                "Routed package {} to {:?}",
                package.package_id, route_result.state.adapter_target
            );
            let mut log_context = LogContext::new();
            log_context.component = Some("Engine".to_string());
            log_context.add("package_id", package.package_id.to_string());
            log_context.add("event_type", "route");
            self.logger.log(LogLevel::Info, &message, &log_context);
        }
        
        if self.config.auto_create_channels {
            if let Some(channel_type) = self.extract_channel_type(&package.package_id) {
                context.channel_type = Some(channel_type);
            }
        }
        
        Ok(context)
    }
    
    fn extract_channel_type(&self, adapter_id: &str) -> Option<ChannelType> {
        if adapter_id.starts_with("group:") {
            let id = adapter_id.trim_start_matches("group:");
            return Some(ChannelType::group(id));
        }
        if adapter_id.starts_with("private:") {
            let id = adapter_id.trim_start_matches("private:");
            return Some(ChannelType::private(id));
        }
        if adapter_id.starts_with("channel:") {
            let id = adapter_id.trim_start_matches("channel:");
            return Some(ChannelType::channel(id));
        }
        None
    }
    
    async fn process_pipeline(&self, package: &Package, context: &ProcessingContext) -> Result<Package> {
        let mut stream: Arc<dyn Stream> = {
            Arc::new(crate::streams::StandardStream::new(
                "default".to_string(),
                ChannelType::group("default"),
                self.logger.clone(),
            ))
        };
        
        if let Some(channel_type) = &context.channel_type {
            match self.channel_manager.get_or_create_channel(channel_type).await {
                Ok(s) => stream = s,
                Err(e) => {
                    let message = format!("Failed to get channel for {:?}: {}", channel_type, e);
                    let mut log_context = LogContext::new();
                    log_context.component = Some("Engine".to_string());
                    log_context.add("package_id", package.package_id.to_string());
                    log_context.add("event_type", "channel_error");
                    self.logger.log(LogLevel::Error, &message, &log_context);
                }
            }
        }
        
        match stream.process(vec![package.clone()]).await {
            Ok(processed) => {
                if let Some(p) = processed.into_iter().next() {
                    let message = format!("Processed package {}", package.package_id);
                    let mut log_context = LogContext::new();
                    log_context.component = Some("Engine".to_string());
                    log_context.add("package_id", package.package_id.to_string());
                    log_context.add("event_type", "process_success");
                    self.logger.log(LogLevel::Debug, &message, &log_context);
                    return Ok(p);
                }
            }
            Err(e) => {
                let message = format!("Failed to process package {:?}: {}", package.package_id, e);
                let mut log_context = LogContext::new();
                log_context.component = Some("Engine".to_string());
                log_context.add("package_id", package.package_id.to_string());
                log_context.add("event_type", "process_error");
                self.logger.log(LogLevel::Error, &message, &log_context);
                return Ok(package.clone());
            }
        }
        
        Ok(package.clone())
    }
}

#[async_trait]
impl Engine for StandardEngine {
    fn config(&self) -> &EngineConfig {
        &self.config
    }

    fn stats(&self) -> EngineStats {
        self.stats.clone()
    }

    fn state(&self) -> EngineState {
        // Try to acquire read lock without blocking
        // In async context, this will fail gracefully
        match self.state.try_read() {
            Ok(guard) => EngineState {
                status: guard.status,
                last_error: guard.last_error.clone(),
            },
            Err(_) => EngineState {
                status: EngineStatus::Idle,
                last_error: Some("Unable to acquire state lock".to_string()),
            },
        }
    }

    async fn set_config(&mut self, config: EngineConfig) -> Result<()> {
        let message = format!("Engine config updated: {:?}", config);
        self.config = config;
        
        let mut log_context = LogContext::new();
        log_context.component = Some("Engine".to_string());
        self.logger.log(LogLevel::Info, &message, &log_context);
        
        Ok(())
    }

    async fn start(&mut self) -> Result<()> {
        let mut state = self.state.write().await;
        
        if state.status.is_running() {
            let message = "Engine is already running";
            let mut log_context = LogContext::new();
            log_context.component = Some("Engine".to_string());
            self.logger.log(LogLevel::Warn, message, &log_context);
            return Err(LoquatError::Unknown(message.to_string()));
        }
        
        state.status = EngineStatus::Processing;
        state.last_error = None;
        drop(state);
        
        let mut log_context = LogContext::new();
        log_context.component = Some("Engine".to_string());
        self.logger.log(LogLevel::Info, "Engine started", &log_context);
        
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        let mut state = self.state.write().await;
        state.status = EngineStatus::Stopped;
        drop(state);
        
        let mut log_context = LogContext::new();
        log_context.component = Some("Engine".to_string());
        self.logger.log(LogLevel::Info, "Engine stopped", &log_context);
        
        Ok(())
    }

    async fn process(&mut self, package: Package) -> Result<Package> {
        let start_time = std::time::Instant::now();
        
        let context = self.get_processing_context(&package).await?;
        let result = self.process_pipeline(&package, &context).await?;
        
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let mut stats = self.stats.clone();
        stats.record_package(true);
        stats.update_avg_time(duration_ms);
        self.stats = stats;
        
        let mut state = self.state.write().await;
        state.status = EngineStatus::Idle;
        drop(state);
        
        Ok(result)
    }

    async fn get_channel(&self, channel_type: &ChannelType) -> Result<Option<Arc<dyn Stream>>> {
        self.channel_manager.get_channel(channel_type).await
    }

    fn is_running(&self) -> bool {
        // Try to acquire read lock without blocking
        // In async context, this will fail gracefully
        match self.state.try_read() {
            Ok(guard) => matches!(guard.status, EngineStatus::Processing),
            Err(_) => false, // Lock is held, assume not running to avoid blocking
        }
    }
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
    async fn test_engine_creation() {
        let logger = create_test_logger();
        let engine = StandardEngine::new(logger);
        
        assert!(!engine.is_running());
        assert_eq!(engine.config().auto_initialize, true);
        assert_eq!(engine.config().auto_route, true);
    }

    #[tokio::test]
    async fn test_engine_start_stop() {
        let logger = create_test_logger();
        let mut engine = StandardEngine::new(logger);
        
        assert!(!engine.is_running());
        
        assert!(engine.start().await.is_ok());
        assert!(engine.is_running());
        
        assert!(engine.stop().await.is_ok());
        assert!(!engine.is_running());
    }

    #[tokio::test]
    async fn test_extract_channel_type() {
        let logger = create_test_logger();
        let engine = StandardEngine::new(logger);
        
        assert_eq!(engine.extract_channel_type("group:test_group"), Some(ChannelType::group("test_group")));
        assert_eq!(engine.extract_channel_type("private:test_user"), Some(ChannelType::private("test_user")));
        assert_eq!(engine.extract_channel_type("channel:test_channel"), Some(ChannelType::channel("test_channel")));
        assert!(engine.extract_channel_type("unknown").is_none());
    }
}
