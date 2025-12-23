//! Stream processor for processing packages through pools in sequence

use crate::events::Package;
use crate::logging::traits::{LogLevel, LogContext};
use crate::pools::{Pool, PoolType};
use std::sync::Arc;

/// Stream processor - processes packages through 9 pools in sequence
pub struct StreamProcessor {
    logger: Arc<dyn crate::logging::traits::Logger>,
}

impl StreamProcessor {
    /// Create a new stream processor
    pub fn new(logger: Arc<dyn crate::logging::traits::Logger>) -> Self {
        Self { logger }
    }
    
    /// Process packages through pools in sequence
    /// Takes a list of (pool_type, pool) tuples
    pub async fn process_sequence(
        &self,
        pools: &[(PoolType, Arc<dyn Pool>)],
        packages: Vec<Package>,
    ) -> Vec<Package> {
        let mut current_packages = packages;
        
        for (pool_type, pool) in pools {
            let message = format!(
                "Processing {} packages through {:?} pool",
                current_packages.len(),
                pool_type
            );
            let context = LogContext::new().with_component("StreamProcessor");
            self.logger.log(LogLevel::Debug, &message, &context);
            
            let next_packages = pool.process_batch(current_packages).await;
            current_packages = next_packages;
        }
        
        current_packages
    }
}

impl std::fmt::Debug for StreamProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamProcessor").finish()
    }
}
