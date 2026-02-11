//! Standard Loquat Engine implementation

use crate::channel_manager::{StandardChannelManager, ChannelManager as _};
use crate::channels::types::ChannelType;
use crate::engine::types::{EngineConfig, EngineStats, EngineState, ProcessingContext, EngineStatus};
use crate::engine::events::{EngineEvent, EventCallback, EventSubscription, EventFilter, CloneableEventCallback};
use crate::engine::traits::Engine;
use crate::errors::{LoquatError, Result};
use crate::events::Package;
use crate::logging::traits::{LogContext, LogLevel, Logger};
use crate::pools::{Pool, PoolType};
use crate::routers::{Router, StandardRouter, RouteTarget};
use crate::streams::Stream;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Standard Loquat Engine - core coordinator
#[derive(Clone)]
pub struct StandardEngine {
    config: EngineConfig,
    stats: EngineStats,
    state: Arc<tokio::sync::RwLock<EngineState>>,
    router: Arc<StandardRouter>,
    channel_manager: Arc<StandardChannelManager>,
    logger: Arc<dyn Logger>,
    
    pools: Arc<tokio::sync::RwLock<HashMap<PoolType, Arc<dyn Pool>>>>,
    subscriptions: Arc<tokio::sync::RwLock<HashMap<String, EventSubscriptionEntry>>>,
    subscription_counter: Arc<AtomicU64>,
}

/// Event subscription entry
struct EventSubscriptionEntry {
    id: String,
    event_pattern: String,
    callback: CloneableEventCallback,
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
                status: EngineStatus::Stopped,
                last_error: None,
            })),
            router: Arc::new(StandardRouter::new(logger_clone.clone())),
            channel_manager: Arc::new(StandardChannelManager::new(logger_clone)),
            logger,
            
            pools: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            subscriptions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            subscription_counter: Arc::new(AtomicU64::new(0)),
        }
    }
    
    pub fn with_config(config: EngineConfig, logger: Arc<dyn Logger>) -> Self {
        let logger_clone = logger.clone();
        Self {
            config,
            stats: EngineStats::new(),
            state: Arc::new(tokio::sync::RwLock::new(EngineState {
                status: EngineStatus::Stopped,
                last_error: None,
            })),
            router: Arc::new(StandardRouter::new(logger_clone.clone())),
            channel_manager: Arc::new(StandardChannelManager::new(logger_clone)),
            logger,
            
            pools: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            subscriptions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            subscription_counter: Arc::new(AtomicU64::new(0)),
        }
    }
    
    async fn get_processing_context(&self, package_id: &str) -> Result<ProcessingContext> {
        let mut context = ProcessingContext::new();
        
        if self.config.auto_route {
            context.route_target = Some(RouteTarget::Adapter("adapter:placeholder".to_string()));
            
            let message = format!("Routing package {} (simplified routing in v2.0)", package_id);
            let mut log_context = LogContext::new();
            log_context.component = Some("Engine".to_string());
            log_context.add("package_id", package_id.to_string());
            log_context.add("event_type", "route");
            self.logger.log(LogLevel::Warn, &message, &log_context);
        }
        
        if self.config.auto_create_channels {
            if let Some(channel_type) = self.extract_channel_type(package_id) {
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
    
    async fn process_pipeline(&self, package: Package, _context: &ProcessingContext) -> Result<Package> {
        let package_id = &package.package_id;
        let message = format!("Package {} processed (stream disabled)", package_id);
        let mut log_context = LogContext::new();
        log_context.component = Some("Engine".to_string());
        log_context.add("package_id", package_id.to_string());
        log_context.add("event_type", "process_skipped");
        self.logger.log(LogLevel::Warn, &message, &log_context);
        
        Ok(package)
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

    async fn state(&self) -> EngineState {
        let guard = self.state.read().await;
        EngineState {
            status: guard.status,
            last_error: guard.last_error.clone(),
        }
    }

    fn try_state(&self) -> EngineState {
        match self.state.try_read() {
            Ok(guard) => EngineState {
                status: guard.status,
                last_error: guard.last_error.clone(),
            },
            Err(_) => EngineState {
                status: EngineStatus::Stopped,
                last_error: Some("Unable to acquire state lock (non-blocking)".to_string()),
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
        
        if state.status.is_running() || state.status.is_transitioning() {
            let message = "Engine is already running or starting";
            let mut log_context = LogContext::new();
            log_context.component = Some("Engine".to_string());
            self.logger.log(LogLevel::Warn, message, &log_context);
            return Err(LoquatError::Unknown(message.to_string()));
        }
        
        state.status = EngineStatus::Starting;
        state.last_error = None;
        drop(state);
        
        let mut log_context = LogContext::new();
        log_context.component = Some("Engine".to_string());
        self.logger.log(LogLevel::Info, "Engine starting...", &log_context);
        
        let mut state = self.state.write().await;
        state.status = EngineStatus::Running;
        drop(state);
        
        self.logger.log(LogLevel::Info, "Engine started and ready to process", &log_context);
        
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
        {
            let state = self.state.read().await;
            if !state.status.is_running() {
                return Err(LoquatError::Unknown("Engine is not running".to_string()));
            }
        }
        
        let start_time = std::time::Instant::now();
        
        let context = self.get_processing_context(&package.package_id).await?;
        let result = self.process_pipeline(package, &context).await?;
        
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let mut stats = self.stats.clone();
        stats.record_package(true);
        stats.update_avg_time(duration_ms);
        self.stats = stats;
        
        Ok(result)
    }

    async fn get_channel(&self, channel_type: &ChannelType) -> Result<Option<Arc<dyn Stream>>> {
        self.channel_manager.get_channel(channel_type).await
    }

    fn is_running(&self) -> bool {
        match self.state.try_read() {
            Ok(guard) => guard.status.is_running(),
            Err(_) => false,
        }
    }

    async fn register_pool(&mut self, pool_type: PoolType, pool: Arc<dyn Pool>) -> Result<()> {
        if !self.config.is_pool_enabled(&pool_type) {
            return Err(LoquatError::Unknown(format!("Pool {} is disabled", pool_type)));
        }

        let mut pools = self.pools.write().await;
        let old_pool = pools.insert(pool_type, pool);
        drop(pools);

        let mut log_context = LogContext::new();
        log_context.component = Some("Engine".to_string());
        
        if old_pool.is_some() {
            self.logger.log(LogLevel::Info, &format!("Pool {} replaced", pool_type), &log_context);
        } else {
            self.logger.log(LogLevel::Info, &format!("Pool {} registered", pool_type), &log_context);
            
            if self.config.enable_events {
                let _ = self.emit_event_internal(EngineEvent::PoolActivated(
                    crate::engine::events::PoolActivatedEvent {
                        pool_type,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
                    },
                )).await;
            }
        }

        Ok(())
    }

    async fn unregister_pool(&mut self, pool_type: PoolType) -> Result<()> {
        let mut pools = self.pools.write().await;
        let pool = pools.remove(&pool_type);
        drop(pools);

        if pool.is_none() {
            return Err(LoquatError::Unknown(format!("Pool {} not found", pool_type)));
        }

        let mut log_context = LogContext::new();
        log_context.component = Some("Engine".to_string());
        self.logger.log(LogLevel::Info, &format!("Pool {} unregistered", pool_type), &log_context);

        if self.config.enable_events {
            let _ = self.emit_event_internal(EngineEvent::PoolDeactivated(
                crate::engine::events::PoolDeactivatedEvent {
                    pool_type,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                },
            )).await;
        }

        Ok(())
    }

    fn get_pools(&self) -> HashMap<PoolType, Arc<dyn Pool>> {
        match self.pools.try_read() {
            Ok(guard) => guard.clone(),
            Err(_) => HashMap::new(),
        }
    }

    async fn emit_event(&self, event: EngineEvent) -> Result<()> {
        self.emit_event_internal(event).await
    }

    async fn subscribe(&mut self, event_pattern: String, callback: CloneableEventCallback) -> Result<EventSubscription> {
        let id = format!("sub-{}", self.subscription_counter.fetch_add(1, Ordering::SeqCst));
        
        let entry = EventSubscriptionEntry {
            id: id.clone(),
            event_pattern: event_pattern.clone(),
            callback: callback.clone(),
        };

        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.insert(id.clone(), entry);
        drop(subscriptions);

        let mut log_context = LogContext::new();
        log_context.component = Some("Engine".to_string());
        log_context.add("subscription_id", id.clone());
        let event_pattern_for_log = event_pattern.clone();
        log_context.add("event_pattern", &event_pattern_for_log);
        self.logger.log(LogLevel::Info, &format!("Event subscription created: {}", id), &log_context);

        Ok(EventSubscription {
            id,
            event_pattern,
            callback: callback,
        })
    }

    async fn unsubscribe(&mut self, subscription_id: &str) -> Result<()> {
        let mut subscriptions = self.subscriptions.write().await;
        let removed = subscriptions.remove(subscription_id);
        drop(subscriptions);

        if removed.is_none() {
            return Err(LoquatError::Unknown(format!("Subscription {} not found", subscription_id)));
        }

        let mut log_context = LogContext::new();
        log_context.component = Some("Engine".to_string());
        log_context.add("subscription_id", subscription_id.to_string());
        self.logger.log(LogLevel::Info, &format!("Event subscription removed: {}", subscription_id), &log_context);

        Ok(())
    }
}

impl StandardEngine {
    async fn emit_event_internal(&self, event: EngineEvent) -> Result<()> {
        let subscriptions = self.subscriptions.read().await;
        
        let mut tasks = Vec::new();
        for entry in subscriptions.values() {
            let filter = EventFilter::new(&entry.event_pattern);
            if filter.matches(&event) {
                let callback = entry.callback.clone();
                let event_clone = event.clone();
                tasks.push(async move {
                    callback.handle(event_clone).await;
                });
            }
        }
        drop(subscriptions);

        for task in tasks {
            tokio::spawn(task);
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct DummyCallback;

#[async_trait]
impl EventCallback for DummyCallback {
    async fn handle(&self, _event: EngineEvent) {
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
