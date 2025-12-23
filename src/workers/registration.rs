//! Worker registration and matching rules

use crate::events::TargetSite;
use crate::workers::Worker;
use regex::Regex;
use std::fmt::Debug;

/// TargetSite matching rule
pub enum MatchingRule {
    /// Match all target sites
    All,
    
    /// Match specific worker
    Worker(String),
    
    /// Match specific bot
    Bot(String),
    
    /// Match specific group
    Group(String),
    
    /// Match specific user
    User(String),
    
    /// Match specific channel
    Channel(String),
    
    /// Regex pattern matching on site_id
    Regex(Regex),
    
    /// Custom matching logic
    Custom(Box<dyn Fn(&TargetSite) -> bool + Send + Sync>),
}

impl std::fmt::Debug for MatchingRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => write!(f, "All"),
            Self::Worker(name) => write!(f, "Worker({})", name),
            Self::Bot(name) => write!(f, "Bot({})", name),
            Self::Group(name) => write!(f, "Group({})", name),
            Self::User(name) => write!(f, "User({})", name),
            Self::Channel(name) => write!(f, "Channel({})", name),
            Self::Regex(regex) => write!(f, "Regex({:?})", regex.as_str()),
            Self::Custom(_) => write!(f, "Custom(<closure>)"),
        }
    }
}

impl MatchingRule {
    /// Check if target site matches this rule
    pub fn matches(&self, target_site: &TargetSite) -> bool {
        match self {
            Self::All => true,
            Self::Worker(name) => {
                if let crate::events::SiteType::Worker(ref w) = target_site.site_type {
                    w == name
                } else {
                    false
                }
            }
            Self::Bot(name) => {
                if let crate::events::SiteType::Bot(ref b) = target_site.site_type {
                    b == name
                } else {
                    false
                }
            }
            Self::Group(name) => {
                if let crate::events::SiteType::Group(ref g) = target_site.site_type {
                    g == name
                } else {
                    false
                }
            }
            Self::User(name) => {
                if let crate::events::SiteType::User(ref u) = target_site.site_type {
                    u == name
                } else {
                    false
                }
            }
            Self::Channel(name) => {
                if let crate::events::SiteType::Channel(ref c) = target_site.site_type {
                    c == name
                } else {
                    false
                }
            }
            Self::Regex(regex) => regex.is_match(&target_site.site_id),
            Self::Custom(f) => f(target_site),
        }
    }
    
    /// Create a regex matching rule
    pub fn regex(pattern: &str) -> Result<Self, regex::Error> {
        Ok(Self::Regex(Regex::new(pattern)?))
    }
}

/// Worker registration with priority and matching rule
pub struct WorkerRegistration {
    /// Worker instance
    pub worker: Box<dyn Worker>,
    
    /// TargetSite matching rule
    pub matching_rule: MatchingRule,
    
    /// Worker priority (assigned at load time, starting from 0)
    pub priority: u32,
}

impl WorkerRegistration {
    /// Create a new worker registration
    pub fn new(worker: Box<dyn Worker>, matching_rule: MatchingRule, priority: u32) -> Self {
        Self {
            worker,
            matching_rule,
            priority,
        }
    }
    
    /// Check if this registration matches any of the target sites
    pub fn matches_any(&self, target_sites: &[TargetSite]) -> bool {
        target_sites
            .iter()
            .any(|ts| self.matching_rule.matches(ts) && self.worker.matches(ts))
    }
    
    /// Check if this registration matches a specific target site
    pub fn matches(&self, target_site: &TargetSite) -> bool {
        self.matching_rule.matches(target_site) && self.worker.matches(target_site)
    }
}

impl Debug for WorkerRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerRegistration")
            .field("worker_name", &self.worker.name())
            .field("worker_type", &self.worker.worker_type())
            .field("priority", &self.priority)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::SiteType;
    use crate::workers::WorkerResult;
    use crate::workers::WorkerType;
    use async_trait::async_trait;

    #[derive(Debug)]
    struct MockWorker {
        name: String,
    }

    impl MockWorker {
        pub fn new(name: String) -> Self {
            Self { name }
        }
    }

    #[async_trait]
    impl Worker for MockWorker {
        fn name(&self) -> &str {
            &self.name
        }

        fn worker_type(&self) -> WorkerType {
            WorkerType::Custom("mock".to_string())
        }

        fn matches(&self, _target_site: &TargetSite) -> bool {
            true
        }

        async fn handle_batch(&self, _packages: Vec<crate::events::Package>) -> WorkerResult {
            WorkerResult::release()
        }
    }

    #[test]
    fn test_matching_rule_all() {
        let rule = MatchingRule::All;
        let site = TargetSite::new("test", SiteType::Worker("worker1".to_string()));
        assert!(rule.matches(&site));
    }

    #[test]
    fn test_matching_rule_worker() {
        let rule = MatchingRule::Worker("worker1".to_string());
        let site1 = TargetSite::new("test", SiteType::Worker("worker1".to_string()));
        let site2 = TargetSite::new("test", SiteType::Worker("worker2".to_string()));
        
        assert!(rule.matches(&site1));
        assert!(!rule.matches(&site2));
    }

    #[test]
    fn test_matching_rule_group() {
        let rule = MatchingRule::Group("123456".to_string());
        let site1 = TargetSite::new("test", SiteType::Group("123456".to_string()));
        let site2 = TargetSite::new("test", SiteType::Group("789012".to_string()));
        
        assert!(rule.matches(&site1));
        assert!(!rule.matches(&site2));
    }

    #[test]
    fn test_matching_rule_regex() {
        let rule = MatchingRule::regex(r"^worker\d+$").unwrap();
        let site1 = TargetSite::new("worker123", SiteType::Worker("worker1".to_string()));
        let site2 = TargetSite::new("test_worker", SiteType::Worker("worker2".to_string()));
        
        assert!(rule.matches(&site1));
        assert!(!rule.matches(&site2));
    }

    #[test]
    fn test_worker_registration() {
        let worker = Box::new(MockWorker::new("test_worker".to_string()));
        let rule = MatchingRule::All;
        let registration = WorkerRegistration::new(worker, rule, 0);
        
        assert_eq!(registration.worker.name(), "test_worker");
        assert_eq!(registration.priority, 0);
        assert!(registration.matches(&TargetSite::new("test", SiteType::Unknown)));
    }

    #[test]
    fn test_worker_registration_matches_any() {
        let worker = Box::new(MockWorker::new("test_worker".to_string()));
        let rule = MatchingRule::Group("123456".to_string());
        let registration = WorkerRegistration::new(worker, rule, 0);
        
        let sites = vec![
            TargetSite::new("test", SiteType::Group("123456".to_string())),
            TargetSite::new("test", SiteType::Group("789012".to_string())),
        ];
        
        assert!(registration.matches_any(&sites));
    }
}
