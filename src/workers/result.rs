//! Worker processing result

use crate::events::Package;
use serde::{Deserialize, Serialize};

/// Worker processing result
#[derive(Debug, Serialize)]
// Note: Clone is not implemented because Package cannot be cloned (contains trait object)
// Note: Deserialize is not implemented because Package cannot be deserialized
pub enum WorkerResult {
    /// Processing complete, Package moves to next pool
    Release(Package),
    
    /// Modified packages, continue processing in current pool
    /// Workers must ensure output packages won't be matched by themselves again
    Modify(Vec<Package>),
}

impl WorkerResult {
    /// Create a Release result
    pub fn release(package: Package) -> Self {
        Self::Release(package)
    }
    
    /// Create a Modify result
    pub fn modify(packages: Vec<Package>) -> Self {
        Self::Modify(packages)
    }
    
    /// Check if this is a Release result
    pub fn is_release(&self) -> bool {
        matches!(self, Self::Release(_))
    }
    
    /// Check if this is a Modify result
    pub fn is_modify(&self) -> bool {
        matches!(self, Self::Modify(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_result_release() {
        let result = WorkerResult::release(Package::new());
        assert!(result.is_release());
        assert!(!result.is_modify());
    }

    #[test]
    fn test_worker_result_modify() {
        let packages = vec![Package::new()];
        let result = WorkerResult::modify(packages);
        assert!(!result.is_release());
        assert!(result.is_modify());
    }
}
