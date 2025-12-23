//! Pool type definitions

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Pool type classification - 9 pool types in processing order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolType {
    /// 预输入池
    PreInput,
    /// 输入池 - 第三方可注册
    Input,
    /// 输入中间池
    InputMiddle,
    /// 预处理池（交换池）- 第三方可注册
    PreProcess,
    /// 处理中间池
    ProcessMiddle,
    /// 处理池 - 第三方可注册
    Process,
    /// 后处理池
    PostProcess,
    /// 输出池 - 第三方可注册
    Output,
    /// 后输出池
    PostOutput,
}

impl PoolType {
    /// Get all pool types in processing order
    pub fn processing_order() -> Vec<Self> {
        vec![
            Self::PreInput,
            Self::Input,
            Self::InputMiddle,
            Self::PreProcess,
            Self::ProcessMiddle,
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
            Self::Input | Self::PreProcess | Self::Process | Self::Output
        )
    }
    
    /// Get the pool position in processing order (0-based)
    pub fn position(&self) -> usize {
        match self {
            Self::PreInput => 0,
            Self::Input => 1,
            Self::InputMiddle => 2,
            Self::PreProcess => 3,
            Self::ProcessMiddle => 4,
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
            Self::InputMiddle => write!(f, "input_middle"),
            Self::PreProcess => write!(f, "pre_process"),
            Self::ProcessMiddle => write!(f, "process_middle"),
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
        assert!(!PoolType::InputMiddle.allows_third_party());
        assert!(PoolType::PreProcess.allows_third_party());
        assert!(!PoolType::ProcessMiddle.allows_third_party());
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
