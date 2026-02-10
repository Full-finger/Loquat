//! Package structure for stream-based event processing
//!
//! Package is a basic unit processed on the stream,
//! containing target_sites, payload, and trace information.

use crate::events::{Block, TargetSite};
use crate::events::payloads::BoxedPayload;
use serde::Serialize;
use chrono::{DateTime, Utc};

/// Package - basic processing unit on stream (v2.0 with payload support)
#[derive(Debug, Serialize)]
// Note: Clone is not implemented because BoxedPayload (trait object) cannot be cloned
// Note: Deserialize is not implemented because BoxedPayload (trait object) cannot be deserialized
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
    
    /// Universal payload (v2.0 feature)
    #[serde(skip)]
    pub payload: Option<BoxedPayload>,
    
    /// Payload type name (for quick filtering without downcasting)
    pub payload_type: Option<String>,
    
    /// Processing trace - list of worker IDs that processed this package
    pub trace: Vec<String>,
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
            payload: None,
            payload_type: None,
            trace: Vec::new(),
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
    
    /// Add a payload to the package
    pub fn with_payload<P: crate::events::payloads::UniversalPayload + 'static>(mut self, payload: P) -> Self {
        self.payload_type = Some(payload.type_name().to_string());
        self.payload = Some(Box::new(payload));
        self
    }
    
    /// Get payload by type (type-safe downcast)
    pub fn get_payload<P: crate::events::payloads::UniversalPayload>(&self) -> Option<&P> {
        self.payload.as_ref()?.as_any().downcast_ref()
    }
    
    /// Get mutable payload by type (type-safe downcast)
    pub fn get_payload_mut<P: crate::events::payloads::UniversalPayload>(&mut self) -> Option<&mut P> {
        self.payload.as_mut()?.as_any_mut().downcast_mut()
    }
    
    /// Add a worker to the trace
    pub fn trace_worker(mut self, worker_name: &str) -> Self {
        self.trace.push(worker_name.to_string());
        self
    }
    
    /// Check if package has a specific payload type
    pub fn has_payload_type(&self, type_name: &str) -> bool {
        self.payload_type.as_deref() == Some(type_name)
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
