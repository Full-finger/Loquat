//! Integration tests for Actor pattern implementation
//!
//! This module tests the complete flow:
//! 1. Create actor
//! 2. Wrap with AdapterWrapper
//! 3. Start adapter
//! 4. Query status
//! 5. Stop adapter
//! 6. Verify cleanup

use crate::adapters::actor::{
    AdapterWrapper, BaseAdapterActor, AdapterActor, AdapterMessage,
    create_console_adapter_actor,
};
use crate::adapters::core::{Adapter, AdapterConfig, AdapterStatus};
use crate::errors::Result;
use tokio::sync::mpsc;
use std::sync::Arc;

#[tokio::test]
async fn test_adapter_wrapper_lifecycle() -> Result<()> {
    // Create configuration
    let config = AdapterConfig::new("console", "test-001", "stdio://");
    
    // Create actor and message channel
    let (sender, actor) = create_console_adapter_actor(config.clone()).await?;
    
    // Create wrapper (actor is already running from create_console_adapter_actor)
    let wrapper = AdapterWrapper::new(
        config.adapter_id.clone(),
        "TestAdapter".to_string(),
        "1.0.0".to_string(),
        config.clone(),
        sender,
        None, // Actor is already running, no separate task handle
    );
    
    // Test: Check initial status
    assert_eq!(wrapper.adapter_id(), "test-001");
    assert_eq!(wrapper.name(), "TestAdapter");
    
    let status = wrapper.status().await;
    assert!(matches!(status, AdapterStatus::Ready));
    
    // Test: Start adapter
    wrapper.start().await?;
    
    let status = wrapper.status().await;
    assert!(matches!(status, AdapterStatus::Running));
    
    // Test: Check is_running
    assert!(wrapper.is_running().await);
    assert!(wrapper.is_connected().await);
    
    // Test: Get statistics
    let stats = wrapper.statistics().await;
    assert_eq!(stats.events_received, 0);
    
    // Test: Stop adapter
    wrapper.stop().await?;
    
    let status = wrapper.status().await;
    assert!(matches!(status, AdapterStatus::Stopped));
    
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_adapter_wrapper_as_trait_object() -> Result<()> {
    // Create a simple BaseAdapterActor
    let config = AdapterConfig::new("test", "test-002", "ws://localhost");
    let actor = BaseAdapterActor::new(config.clone(), "TestAdapter".to_string(), "1.0.0".to_string());
    
    // Create message channel
    let (sender, mut receiver) = mpsc::unbounded_channel();
    
    // Spawn actor task
    let mut actor_clone = actor.clone();
    let handle = tokio::spawn(async move {
        while let Some(msg) = receiver.recv().await {
            let _ = actor_clone.handle_message(msg).await;
        }
    });
    
    // Create wrapper
    let wrapper = AdapterWrapper::new(
        config.adapter_id.clone(),
        "TestAdapter".to_string(),
        "1.0.0".to_string(),
        config.clone(),
        sender,
        Some(handle),
    );
    
    // Test: Can be used as Arc<dyn Adapter>
    let adapter: Arc<dyn crate::adapters::core::Adapter> = Arc::new(wrapper);
    
    // Test: Synchronous methods work
    assert_eq!(adapter.name(), "TestAdapter");
    assert_eq!(adapter.adapter_id(), "test-002");
    
    let status = adapter.status();
    assert!(matches!(status, AdapterStatus::Ready));
    
    // Test: Async methods via wrapper still work
    if let Some(wrapper) = adapter.as_any().downcast_ref::<AdapterWrapper>() {
        wrapper.start().await?;
        
        let status = adapter.status();
        assert!(matches!(status, AdapterStatus::Running));
        
        wrapper.stop().await?;
    }
    
    Ok(())
}

#[tokio::test]
async fn test_multiple_concurrent_access() -> Result<()> {
    let config = AdapterConfig::new("console", "test-003", "stdio://");
    let (sender, _actor) = create_console_adapter_actor(config.clone()).await?;
    
    let wrapper = Arc::new(AdapterWrapper::new(
        config.adapter_id.clone(),
        "TestAdapter".to_string(),
        "1.0.0".to_string(),
        config.clone(),
        sender,
        None, // Actor is already running
    ));
    
    // Spawn multiple concurrent tasks
    let mut handles = vec![];
    
    for i in 0..10 {
        let wrapper_clone = Arc::clone(&wrapper);
        let handle = tokio::spawn(async move {
            // Each task queries status
            let _status = wrapper_clone.status().await;
            let _stats = wrapper_clone.statistics().await;
            i
        });
        handles.push(handle);
    }
    
    // Wait for all tasks
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.unwrap());
    }
    
    // Verify all tasks completed
    assert_eq!(results.len(), 10);
    
    Ok(())
}

#[tokio::test]
async fn test_message_communication() -> Result<()> {
    let config = AdapterConfig::new("test", "test-004", "ws://localhost");
    let actor = BaseAdapterActor::new(config.clone(), "TestAdapter".to_string(), "1.0.0".to_string());
    
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let mut actor_clone = actor.clone();
    
    let handle = tokio::spawn(async move {
        while let Some(msg) = receiver.recv().await {
            let _ = actor_clone.handle_message(msg).await;
        }
    });
    
    let wrapper = AdapterWrapper::new(
        config.adapter_id.clone(),
        "TestAdapter".to_string(),
        "1.0.0".to_string(),
        config.clone(),
        sender,
        Some(handle),
    );
    
    // Test: GetStatus message
    let status = wrapper.status().await;
    assert!(matches!(status, AdapterStatus::Ready));
    
    // Test: GetStatistics message
    let stats = wrapper.statistics().await;
    assert_eq!(stats.events_received, 0);
    
    // Test: Start message
    wrapper.start().await?;
    let status = wrapper.status().await;
    assert!(matches!(status, AdapterStatus::Running));
    
    // Test: IsRunning message
    let is_running = wrapper.is_running().await;
    assert!(is_running);
    
    // Test: IsConnected message
    let is_connected = wrapper.is_connected().await;
    assert!(is_connected);
    
    // Test: GetConfig message
    let _config = wrapper.config();
    
    // Test: Stop message
    wrapper.stop().await?;
    let status = wrapper.status().await;
    assert!(matches!(status, AdapterStatus::Stopped));
    
    Ok(())
}
