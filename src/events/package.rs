//! Package structure for stream-based event processing
//!
//! Package is the basic unit processed on the stream,
//! containing target_sites and blocks.

use crate::events::{Block, TargetSite};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

/// Package - basic processing unit on stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// Target sites - worker identifiers for this package
    /// "作用靶点" - worker's identification for Package
    pub target_sites: Vec<TargetSite>,
    
    /// Blocks - array of event blocks
    pub blocks: Vec<Block>,
    
    /// Package timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Package ID
    pub package_id: String,
    
    /// Additional metadata
    pub extra: serde_json::Value,
}

impl Package {
    /// Create a new package
    pub fn new() -> Self {
        Self {
            target_sites: Vec::new(),
            blocks: Vec::new(),
            timestamp: Utc::now(),
            package_id: format!("pkg-{}-{}", 
                Utc::now().timestamp_millis(),
                uuid::Uuid::new_v4()),
            extra: serde_json::json!({}),
        }
    }
    
    /// Add a target site
    pub fn with_target_site(mut self, site: TargetSite) -> Self {
        self.target_sites.push(site);
        self
    }
    
    /// Add multiple target sites
    pub fn with_target_sites(mut self, sites: Vec<TargetSite>) -> Self {
        self.target_sites.extend(sites);
        self
    }
    
    /// Add a block
    pub fn with_block(mut self, block: Block) -> Self {
        self.blocks.push(block);
        self
    }
    
    /// Add multiple blocks
    pub fn with_blocks(mut self, blocks: Vec<Block>) -> Self {
        self.blocks.extend(blocks);
        self
    }
    
    /// Set extra metadata
    pub fn with_extra(mut self, extra: serde_json::Value) -> Self {
        self.extra = extra;
        self
    }
}

impl Default for Package {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{SiteType, BlockType};

    #[test]
    fn test_package_creation() {
        let package = Package::new();
        
        assert!(package.target_sites.is_empty());
        assert!(package.blocks.is_empty());
        assert!(!package.package_id.is_empty());
    }
    
    #[test]
    fn test_package_builder() {
        let site = TargetSite::new("worker1", SiteType::Worker("worker1".to_string()));
        let block = Block::new(BlockType::Default);
        
        let package = Package::new()
            .with_target_site(site)
            .with_block(block);
        
        assert_eq!(package.target_sites.len(), 1);
        assert_eq!(package.blocks.len(), 1);
    }
}
