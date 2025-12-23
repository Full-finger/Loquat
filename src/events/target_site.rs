//! TargetSite - worker identification for Package targeting
//!
//! TargetSite is "作用靶点" - worker's identification for Package.

use serde::{Serialize, Deserialize};
use std::fmt::Debug;

/// Site type classification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SiteType {
    /// Worker type - plugin worker
    Worker(String),
    /// Bot type - main bot
    Bot(String),
    /// Group type - group chat
    Group(String),
    /// User type - direct user chat
    User(String),
    /// Channel type - channel
    Channel(String),
    /// Unknown type
    Unknown,
}

/// TargetSite - worker identification/label group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetSite {
    /// Site identifier/worker name
    pub site_id: String,
    
    /// Site type
    pub site_type: SiteType,
}

impl TargetSite {
    /// Create a new target site
    pub fn new(site_id: &str, site_type: SiteType) -> Self {
        Self {
            site_id: site_id.to_string(),
            site_type,
        }
    }
    
    /// Create a worker type target site
    pub fn worker(worker_name: &str) -> Self {
        Self {
            site_id: worker_name.to_string(),
            site_type: SiteType::Worker(worker_name.to_string()),
        }
    }
    
    /// Create a bot type target site
    pub fn bot(bot_name: &str) -> Self {
        Self {
            site_id: bot_name.to_string(),
            site_type: SiteType::Bot(bot_name.to_string()),
        }
    }
    
    /// Create a group type target site
    pub fn group(group_id: &str) -> Self {
        Self {
            site_id: group_id.to_string(),
            site_type: SiteType::Group(group_id.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_site_creation() {
        let site = TargetSite::new("test_site", SiteType::Worker("worker1".to_string()));
        
        assert_eq!(site.site_id, "test_site");
        matches!(site.site_type, SiteType::Worker(_));
    }
    
    #[test]
    fn test_target_site_worker() {
        let site = TargetSite::worker("test_worker");
        
        assert_eq!(site.site_id, "test_worker");
        matches!(site.site_type, SiteType::Worker(ref s) if s == "test_worker");
    }
    
    #[test]
    fn test_target_site_bot() {
        let site = TargetSite::bot("test_bot");
        
        assert_eq!(site.site_id, "test_bot");
        matches!(site.site_type, SiteType::Bot(ref s) if s == "test_bot");
    }
}
