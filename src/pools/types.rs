//! Pool type definitions

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Pool type classification - 9 pool types in processing order
/// 
/// Processing flow: PreInput → Input → PostInput → PreProcess → MidProcess → Process → PostProcess → Output → PostOutput
/// Public pools (third-party workers can register): Input, PostInput, PreProcess, Process, Output
/// Internal pools (system workers only): PreInput, MidProcess, PostProcess, PostOutput
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolType {
    /// Pre-input pool - system preprocessing (public)
    PreInput,
    /// Input pool - third-party workers can register
    Input,
    /// Post-input pool - target transformation (public)
    PostInput,
    /// Pre-process pool - before processing checks (public)
    PreProcess,
    /// Mid-process pool - intermediate processing (internal)
    MidProcess,
    /// Process pool - core business logic (public)
    Process,
    /// Post-process pool - after processing (internal)
    PostProcess,
    /// Output pool - prepare output (public)
    Output,
    /// Post-output pool - after output (internal)
    PostOutput,
}

impl PoolType {
    /// Get all pool types in processing order
    pub fn processing_order() -> Vec<Self> {
        vec![
            Self::PreInput,
            Self::Input,
            Self::PostInput,
            Self::PreProcess,
            Self::MidProcess,
            Self::Process,
            Self::PostProcess,
            Self::Output,
            Self::PostOutput,
        ]
    }
    
    /// Check if this pool allows third-party worker registration
    pub fn allows_third_party(&self) -> bool {
        matches!(
            self,
            Self::Input | Self::PostInput | Self::PreProcess | Self::Process | Self::Output
        )
    }
    
    /// Get the pool position in processing order (0-based)
    pub fn position(&self) -> usize {
        match self {
            Self::PreInput => 0,
            Self::Input => 1,
            Self::PostInput => 2,
            Self::PreProcess => 3,
            Self::MidProcess => 4,
            Self::Process => 5,
            Self::PostProcess => 6,
            Self::Output => 7,
            Self::PostOutput => 8,
        }
    }
}

impl std::fmt::Display for PoolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PreInput => write!(f, "pre_input"),
            Self::Input => write!(f, "input"),
            Self::PostInput => write!(f, "post_input"),
            Self::PreProcess => write!(f, "pre_process"),
            Self::MidProcess => write!(f, "mid_process"),
            Self::Process => write!(f, "process"),
            Self::PostProcess => write!(f, "post_process"),
            Self::Output => write!(f, "output"),
            Self::PostOutput => write!(f, "post_output"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_type_processing_order() {
        let order = PoolType::processing_order();
        assert_eq!(order.len(), 9);
        assert_eq!(order[0], PoolType::PreInput);
        assert_eq!(order[1], PoolType::Input);
        assert_eq!(order[8], PoolType::PostOutput);
    }

    #[test]
    fn test_pool_type_allows_third_party() {
        assert!(!PoolType::PreInput.allows_third_party());
        assert!(PoolType::Input.allows_third_party());
        assert!(PoolType::PostInput.allows_third_party());
        assert!(PoolType::PreProcess.allows_third_party());
        assert!(!PoolType::MidProcess.allows_third_party());
        assert!(PoolType::Process.allows_third_party());
        assert!(!PoolType::PostProcess.allows_third_party());
        assert!(PoolType::Output.allows_third_party());
        assert!(!PoolType::PostOutput.allows_third_party());
    }

    #[test]
    fn test_pool_type_position() {
        assert_eq!(PoolType::PreInput.position(), 0);
        assert_eq!(PoolType::Input.position(), 1);
        assert_eq!(PoolType::PostOutput.position(), 8);
    }

    #[test]
    fn test_pool_type_display() {
        assert_eq!(PoolType::Input.to_string(), "input");
        assert_eq!(PoolType::Process.to_string(), "process");
        assert_eq!(PoolType::Output.to_string(), "output");
    }
}
