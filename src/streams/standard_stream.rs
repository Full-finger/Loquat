//! Standard stream implementation with 9 pools

use async_trait::async_trait;
use crate::channels::ChannelType;
use crate::events::Package;
use crate::pools::{Pool, PoolType, StandardPool};
use crate::streams::processor::StreamProcessor;
use crate::streams::traits::Stream;
use crate::logging::traits::Logger;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

/// Standard stream - contains 9 pools processing packages in sequence
#[derive(Debug)]
pub struct StandardStream {
    stream_id: String,
    channel_id: String,
    channel_type: ChannelType,
    pools: HashMap<PoolType, Arc<dyn Pool>>,
    processor: StreamProcessor,
}

impl StandardStream {
    /// Create a new standard stream with all 9 pools
    pub fn new(channel_id: String, channel_type: ChannelType, logger: Arc<dyn Logger>) -> Self {
        let stream_id = format!("stream_{}", channel_type.id());
        
        // Create 9 pools
        let mut pools: HashMap<PoolType, Arc<dyn Pool>> = HashMap::new();
        
        for pool_type in PoolType::processing_order() {
            let pool: Arc<dyn Pool> = Arc::new(StandardPool::new(pool_type, logger.clone()));
            pools.insert(pool_type, pool);
        }
        
        let processor = StreamProcessor::new(logger);
        
        Self {
            stream_id,
            channel_id,
            channel_type,
            pools,
            processor,
        }
    }
    
    /// Create a new standard stream with custom stream ID
    pub fn with_id(
        stream_id: String,
        channel_id: String,
        channel_type: ChannelType,
        logger: Arc<dyn Logger>,
    ) -> Self {
        let mut pools: HashMap<PoolType, Arc<dyn Pool>> = HashMap::new();
        
        for pool_type in PoolType::processing_order() {
            let pool: Arc<dyn Pool> = Arc::new(StandardPool::new(pool_type, logger.clone()));
            pools.insert(pool_type, pool);
        }
        
        let processor = StreamProcessor::new(logger);
        
        Self {
            stream_id,
            channel_id,
            channel_type,
            pools,
            processor,
        }
    }
    
    /// Get a specific pool by type
    pub fn get_pool(&self, pool_type: PoolType) -> Option<&Arc<dyn Pool>> {
        self.pools.get(&pool_type)
    }
    
    /// Get a mutable reference to a specific pool by type
    pub fn get_pool_mut(&mut self, pool_type: PoolType) -> Option<&mut Arc<dyn Pool>> {
        self.pools.get_mut(&pool_type)
    }
}

#[async_trait]
impl Stream for StandardStream {
    fn stream_id(&self) -> &str {
        &self.stream_id
    }
    
    fn channel_id(&self) -> &str {
        &self.channel_id
    }
    
    async fn process(&self, packages: Vec<Package>) -> crate::errors::Result<Vec<Package>> {
        // Build pool list in processing order
        let pool_list: Vec<(PoolType, Arc<dyn Pool>)> = PoolType::processing_order()
            .into_iter()
            .filter_map(|pt| self.pools.get(&pt).map(|p| (pt, p.clone())))
            .collect();
        
        // Process through all pools in sequence
        let result = self.processor.process_sequence(&pool_list, packages).await;
        
        Ok(result)
    }
}
