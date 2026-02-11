//! Engine event system

use crate::pools::PoolType;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

/// Engine event type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum EngineEvent {
    /// Package started processing
    PackageStarted(PackageStartedEvent),
    
    /// Package completed processing
    PackageCompleted(PackageCompletedEvent),
    
    /// Package failed processing
    PackageFailed(PackageFailedEvent),
    
    /// Pool activated
    PoolActivated(PoolActivatedEvent),
    
    /// Pool deactivated
    PoolDeactivated(PoolDeactivatedEvent),
    
    /// Worker registered
    WorkerRegistered(WorkerRegisteredEvent),
    
    /// Worker unregistered
    WorkerUnregistered(WorkerUnregisteredEvent),
    
    /// Engine started
    EngineStarted(EngineStartedEvent),
    
    /// Engine stopped
    EngineStopped(EngineStoppedEvent),
    
    /// Engine error
    EngineError(EngineErrorEvent),
}

impl fmt::Display for EngineEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PackageStarted(e) => write!(f, "PackageStarted: {}", e.package_id),
            Self::PackageCompleted(e) => write!(f, "PackageCompleted: {} ({}ms)", e.package_id, e.duration_ms),
            Self::PackageFailed(e) => write!(f, "PackageFailed: {} - {}", e.package_id, e.error),
            Self::PoolActivated(e) => write!(f, "PoolActivated: {}", e.pool_type),
            Self::PoolDeactivated(e) => write!(f, "PoolDeactivated: {}", e.pool_type),
            Self::WorkerRegistered(e) => write!(f, "WorkerRegistered: {}", e.worker_id),
            Self::WorkerUnregistered(e) => write!(f, "WorkerUnregistered: {}", e.worker_id),
            Self::EngineStarted(_) => write!(f, "EngineStarted"),
            Self::EngineStopped(_) => write!(f, "EngineStopped"),
            Self::EngineError(e) => write!(f, "EngineError: {}", e.message),
        }
    }
}

/// Package started event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageStartedEvent {
    /// Package ID
    pub package_id: String,
    
    /// Entry pool
    pub entry_pool: PoolType,
    
    /// Timestamp
    pub timestamp: u64,
}

/// Package completed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageCompletedEvent {
    /// Package ID
    pub package_id: String,
    
    /// Processing duration (ms)
    pub duration_ms: u64,
    
    /// Pools processed
    pub pools_processed: Vec<PoolType>,
    
    /// Timestamp
    pub timestamp: u64,
}

/// Package failed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageFailedEvent {
    /// Package ID
    pub package_id: String,
    
    /// Error message
    pub error: String,
    
    /// Failed pool
    pub failed_pool: Option<PoolType>,
    
    /// Timestamp
    pub timestamp: u64,
}

/// Pool activated event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolActivatedEvent {
    /// Pool type
    pub pool_type: PoolType,
    
    /// Timestamp
    pub timestamp: u64,
}

/// Pool deactivated event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolDeactivatedEvent {
    /// Pool type
    pub pool_type: PoolType,
    
    /// Timestamp
    pub timestamp: u64,
}

/// Worker registered event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRegisteredEvent {
    /// Worker ID
    pub worker_id: String,
    
    /// Pool type
    pub pool_type: PoolType,
    
    /// Timestamp
    pub timestamp: u64,
}

/// Worker unregistered event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerUnregisteredEvent {
    /// Worker ID
    pub worker_id: String,
    
    /// Pool type
    pub pool_type: PoolType,
    
    /// Timestamp
    pub timestamp: u64,
}

/// Engine started event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineStartedEvent {
    /// Timestamp
    pub timestamp: u64,
}

/// Engine stopped event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineStoppedEvent {
    /// Timestamp
    pub timestamp: u64,
}

/// Engine error event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineErrorEvent {
    /// Error message
    pub message: String,
    
    /// Error type
    pub error_type: String,
    
    /// Timestamp
    pub timestamp: u64,
}

/// Event callback trait - object-safe version
#[async_trait]
pub trait EventCallback: Send + Sync + std::fmt::Debug {
    /// Handle event asynchronously
    async fn handle(&self, event: EngineEvent);
}

/// Cloneable wrapper for EventCallback
#[derive(Clone)]
pub struct CloneableEventCallback {
    inner: Arc<dyn EventCallback>,
}

impl CloneableEventCallback {
    /// Create a new cloneable callback
    pub fn new<T: EventCallback + 'static>(callback: T) -> Self {
        Self {
            inner: Arc::new(callback),
        }
    }
    
    /// Handle the event
    pub async fn handle(&self, event: EngineEvent) {
        self.inner.handle(event).await;
    }
}

impl std::fmt::Debug for CloneableEventCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CloneableEventCallback")
            .finish()
    }
}

/// Event subscription
#[derive(Debug)]
pub struct EventSubscription {
    /// Subscription ID
    pub id: String,
    
    /// Event type pattern (supports wildcards)
    pub event_pattern: String,
    
    /// Callback
    pub callback: CloneableEventCallback,
}

/// Event filter
pub struct EventFilter {
    /// Event type pattern (e.g., "package.*" matches all package events)
    pub pattern: String,
}

impl EventFilter {
    /// Create a new event filter
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
        }
    }
    
    /// Check if an event matches filter
    pub fn matches(&self, event: &EngineEvent) -> bool {
        let event_type = match event {
            EngineEvent::PackageStarted(_) => "package.started",
            EngineEvent::PackageCompleted(_) => "package.completed",
            EngineEvent::PackageFailed(_) => "package.failed",
            EngineEvent::PoolActivated(_) => "pool.activated",
            EngineEvent::PoolDeactivated(_) => "pool.deactivated",
            EngineEvent::WorkerRegistered(_) => "worker.registered",
            EngineEvent::WorkerUnregistered(_) => "worker.unregistered",
            EngineEvent::EngineStarted(_) => "engine.started",
            EngineEvent::EngineStopped(_) => "engine.stopped",
            EngineEvent::EngineError(_) => "engine.error",
        };
        
        self.matches_pattern(event_type)
    }
    
    fn matches_pattern(&self, event_type: &str) -> bool {
        // Simple wildcard matching
        if self.pattern == "*" {
            return true;
        }
        
        if self.pattern.contains('*') {
            let parts: Vec<&str> = self.pattern.split('*').collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                let suffix = parts[1];
                return event_type.starts_with(prefix) && event_type.ends_with(suffix);
            }
        }
        
        self.pattern == event_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_filter_wildcard() {
        let filter = EventFilter::new("*");
        let event = EngineEvent::PackageStarted(PackageStartedEvent {
            package_id: "test".to_string(),
            entry_pool: PoolType::Input,
            timestamp: 0,
        });
        assert!(filter.matches(&event));
    }

    #[tokio::test]
    async fn test_event_filter_prefix() {
        let filter = EventFilter::new("package.*");
        
        let event1 = EngineEvent::PackageStarted(PackageStartedEvent {
            package_id: "test".to_string(),
            entry_pool: PoolType::Input,
            timestamp: 0,
        });
        assert!(filter.matches(&event1));
        
        let event2 = EngineEvent::EngineError(EngineErrorEvent {
            message: "test".to_string(),
            error_type: "test".to_string(),
            timestamp: 0,
        });
        assert!(!filter.matches(&event2));
    }

    #[tokio::test]
    async fn test_event_filter_exact() {
        let filter = EventFilter::new("package.started");
        
        let event1 = EngineEvent::PackageStarted(PackageStartedEvent {
            package_id: "test".to_string(),
            entry_pool: PoolType::Input,
            timestamp: 0,
        });
        assert!(filter.matches(&event1));
        
        let event2 = EngineEvent::PackageCompleted(PackageCompletedEvent {
            package_id: "test".to_string(),
            duration_ms: 100,
            pools_processed: vec![],
            timestamp: 0,
        });
        assert!(!filter.matches(&event2));
    }
}
