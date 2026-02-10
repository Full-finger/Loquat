//! Loquat Protocol Buffers
//!
//! 这个包包含了 Loquat 系统的所有 Protocol Buffers 定义
//!
//! # 模块结构
//! - `common`: 通用类型定义（来自 common.proto）
//! - `kernel`: Kernel 服务相关类型（来自 kernel.proto）
//! - `engine`: Engine 服务相关类型（来自 engine.proto）
//!
//! # 使用方式
//! 建议显式使用完整路径：
//! - `loquat_proto::common::EngineInfo`
//! - `loquat_proto::kernel::EngineStatusResponse`  
//! - `loquat_proto::engine::EngineStatus`
//!
//! 示例：
//! ```rust
//! use loquat_proto::common::{Empty, Error, HealthStatus};
//! use loquat_proto::kernel::KernelServiceClient;
//! use loquat_proto::engine::EngineServiceClient;
//! ```

// 生成的代码在 gen 目录中
// common 模块
pub mod common {
    include!("gen/common/loquat.common.rs");
}

// kernel 模块
pub mod kernel {
    include!("gen/kernel/loquat.kernel.rs");
}

// engine 模块
pub mod engine {
    include!("gen/engine/loquat.engine.rs");
}

// 重新导出一些常用的通用类型
pub use common::{Empty, Error, HealthStatus, Config, Event, EventResult, Metric, LogEntry};
