//! Worker trait and related types (v2.0 with Matcher support)

use crate::events::{Package, TargetSite};
use crate::workers::{Matcher, WorkerResult};
use async_trait::async_trait;
use std::fmt::Debug;

/// Worker type classification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerType {
    /// Input worker - registered in Input pool
    Input,
    /// Pre-process worker - registered in PreProcess pool
    PreProcess,
    /// Process worker - registered in Process pool
    Process,
    /// Output worker - registered in Output pool
    Output,
    /// Custom worker type
    Custom(String),
}

impl std::fmt::Display for WorkerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input => write!(f, "input"),
            Self::PreProcess => write!(f, "pre_process"),
            Self::Process => write!(f, "process"),
            Self::Output => write!(f, "output"),
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

/// Worker trait - processing unit registered to pools (v2.0 with Matcher)
#[async_trait]
pub trait Worker: Send + Sync + Debug {
    /// Worker name (unique identifier)
    fn name(&self) -> &str;
    
    /// Worker type
    fn worker_type(&self) -> WorkerType;
    
    /// Get the matcher for this worker (v2.0)
    /// Default implementation checks target sites (backward compatible)
    fn matcher(&self) -> &Matcher {
        // Default: create a matcher that checks target sites
        // Subclasses should override this
        static DEFAULT_MATCHER: Matcher = Matcher::Wildcard;
        &DEFAULT_MATCHER
    }
    
    /// Check if this worker matches a specific target site (legacy, for backward compatibility)
    /// V2.0 workers should implement matcher() instead
    fn matches(&self, target_site: &TargetSite) -> bool {
        // Use the matcher if available
        self.matcher().matches(&Package::new().with_target_site(target_site.clone()))
    }
    
    /// Check if this worker should process a given package (v2.0)
    /// Uses the matcher to determine if the package should be processed
    fn matches_package(&self, package: &Package) -> bool {
        self.matcher().matches(package)
    }
    
    /// Async handle a batch of packages
    /// Workers can split/merge packages
    async fn handle_batch(&self, packages: Vec<Package>) -> WorkerResult;
}

/// Compile-time safety trait for worker output
/// Ensures output packages won't create dead loops
pub trait OutputSafe<T>: Send + Sync {
    /// Check if output is safe (won't be matched by self again)
    fn is_output_safe(&self, output: &T) -> bool;
}

/// Default implementation for Worker checking Package safety
impl<T> OutputSafe<Package> for T
where
    T: Worker + ?Sized,
{
    fn is_output_safe(&self, output: &Package) -> bool {
        // Check that output package won't match this worker again
        !output.target_sites.iter().any(|ts| self.matches(ts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestWorker {
        name: String,
    }

    #[async_trait]
    impl Worker for TestWorker {
        fn name(&self) -> &str {
            &self.name
        }

        fn worker_type(&self) -> WorkerType {
            WorkerType::Process
        }

        fn matches(&self, _target_site: &TargetSite) -> bool {
            true
        }

        async fn handle_batch(&self, _packages: Vec<Package>) -> WorkerResult {
            WorkerResult::release(Package::new())
        }
    }

    #[test]
    fn test_worker_type_display() {
        assert_eq!(WorkerType::Input.to_string(), "input");
        assert_eq!(WorkerType::PreProcess.to_string(), "pre_process");
        assert_eq!(WorkerType::Process.to_string(), "process");
        assert_eq!(WorkerType::Output.to_string(), "output");
        assert_eq!(WorkerType::Custom("test".to_string()).to_string(), "test");
    }

    #[test]
    fn test_output_safe() {
        let worker = TestWorker {
            name: "test_worker".to_string(),
        };

        // Package without matching target sites
        let safe_package = Package::new();
        assert!(worker.is_output_safe(&safe_package));

        // Package with target sites that would match (any state tag)
        let unsafe_package = Package::new().with_target_site(TargetSite::state_user_vip());
        assert!(!worker.is_output_safe(&unsafe_package));
    }
}
