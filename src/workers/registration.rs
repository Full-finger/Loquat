//! Worker registration and matching rules
//!
//! This module handles worker registration with priority-based execution and target site matching.

use crate::events::TargetSite;
use crate::workers::Worker;
use regex::Regex;
use std::fmt::Debug;

/// TargetSite matching rule
/// 
/// Simplified matching rules that work with the new four-dimensional TargetSite system.
pub enum MatchingRule {
    /// Match all target sites (wildcard)
    All,
    
    /// Match specific worker name
    Worker(String),
    
    /// Match specific bot name
    Bot(String),
    
    /// Match specific group ID
    Group(String),
    
    /// Match specific user ID
    User(String),
    
    /// Match specific channel ID
    Channel(String),
    
    /// Regex pattern matching on target site tag string
    Regex(Regex),
    
    /// Custom matching logic based on TargetSite
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
    /// 
    /// Note: With the new four-dimensional TargetSite, this method primarily
    /// checks the tag string. For more sophisticated matching, use the Matcher
    /// system in `matcher.rs`.
    pub fn matches(&self, target_site: &TargetSite) -> bool {
        match self {
            Self::All => true,
            Self::Worker(name) => {
                // Check if target site is a worker-related state
                match target_site {
                    TargetSite::State(state) => {
                        state.tag_string().contains(name)
                    }
                    _ => false,
                }
            }
            Self::Bot(name) => {
                // Check if target site has bot context
                match target_site {
                    TargetSite::Context(ctx) => {
                        ctx.tag_string().contains(name)
                    }
                    _ => false,
                }
            }
            Self::Group(name) => {
                // Check if target site has group context
                match target_site {
                    TargetSite::Context(ctx) => {
                        ctx.tag_string().contains(name) || ctx.tag_string() == "Group"
                    }
                    _ => false,
                }
            }
            Self::User(name) => {
                // Check if target site has user state
                match target_site {
                    TargetSite::State(state) => {
                        state.tag_string().contains(name)
                    }
                    _ => false,
                }
            }
            Self::Channel(name) => {
                // Check if target site has channel context
                match target_site {
                    TargetSite::Context(ctx) => {
                        ctx.tag_string().contains(name) || ctx.tag_string() == "Channel"
                    }
                    _ => false,
                }
            }
            Self::Regex(regex) => {
                // Match against the tag string
                let tag_str = target_site.tag_string();
                regex.is_match(&tag_str)
            }
            Self::Custom(f) => f(target_site),
        }
    }
    
    /// Create a regex matching rule
    pub fn regex(pattern: &str) -> Result<Self, regex::Error> {
        Ok(Self::Regex(Regex::new(pattern)?))
    }
}

/// Worker registration with priority and matching rule
/// 
/// Workers are registered with a priority value (lower = higher priority).
/// When processing packages, workers are checked in priority order.
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
    /// 
    /// Returns true if:
    /// - The matching rule matches at least one target site
    /// - AND the worker itself accepts that target site
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
            WorkerResult::release(crate::events::Package::new())
        }
    }

    #[test]
    fn test_matching_rule_all() {
        let rule = MatchingRule::All;
        let site = TargetSite::state_user_vip();
        assert!(rule.matches(&site));
    }

    #[test]
    fn test_matching_rule_worker() {
        let rule = MatchingRule::Worker("worker1".to_string());
        let site1 = TargetSite::state_user_vip();
        let site2 = TargetSite::domain_text();
        
        // Worker rule matches state tags with "worker1" in their tag string
        // This is a simplified test - in practice, workers would have more specific matching
        assert!(!rule.matches(&site1)); // UserVip doesn't contain "worker1"
        assert!(!rule.matches(&site2)); // Text domain doesn't match worker rule
    }

    #[test]
    fn test_matching_rule_group() {
        let rule = MatchingRule::Group("123456".to_string());
        let site1 = TargetSite::context_group();
        let site2 = TargetSite::context_direct();
        
        // Group rule matches context with "Group"
        assert!(rule.matches(&site1)); // Group context matches
        assert!(!rule.matches(&site2)); // Direct context doesn't match
    }

    #[test]
    fn test_matching_rule_regex() {
        let rule = MatchingRule::regex(r"^Custom\(.*\)$").unwrap();
        let site1 = TargetSite::state_custom("worker123");
        let site2 = TargetSite::domain_text();
        
        assert!(rule.matches(&site1)); // Matches Custom("worker123")
        assert!(!rule.matches(&site2)); // Text doesn't match pattern
    }

    #[test]
    fn test_worker_registration() {
        let worker = Box::new(MockWorker::new("test_worker".to_string()));
        let rule = MatchingRule::All;
        let registration = WorkerRegistration::new(worker, rule, 0);
        
        assert_eq!(registration.worker.name(), "test_worker");
        assert_eq!(registration.priority, 0);
        assert!(registration.matches(&TargetSite::domain_text()));
    }

    #[test]
    fn test_worker_registration_matches_any() {
        let worker = Box::new(MockWorker::new("test_worker".to_string()));
        let rule = MatchingRule::All;
        let registration = WorkerRegistration::new(worker, rule, 0);
        
        let sites = vec![
            TargetSite::domain_text(),
            TargetSite::motif_command(),
            TargetSite::state_intent_weather(),
        ];
        
        assert!(registration.matches_any(&sites));
    }
}
