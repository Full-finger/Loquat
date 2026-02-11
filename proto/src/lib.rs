//! Loquat Protocol Buffers
//!
//! 这个包包含了 Loquat 系统的所有 Protocol Buffers 定义
//!
//! # 模块结构
//! - `v1`: Package 核心类型（来自 package.proto）
//! - `common`: 通用类型定义（来自 common.proto）
//! - `kernel`: Kernel 服务相关类型（来自 kernel.proto）
//! - `engine`: Engine 服务相关类型（来自 engine.proto）
//!
//! # 使用方式
//! 建议显式使用完整路径：
//! - `loquat_proto::v1::Package`
//! - `loquat_proto::v1::TargetSite`
//! - `loquat_proto::kernel::KernelServiceClient`
//! - `loquat_proto::engine::EngineServiceClient`
//!
//! 示例：
//! ```rust
//! use loquat_proto::v1::{Package, PoolState, TargetSite};
//! use loquat_proto::engine::EngineServiceClient;
//! use loquat_proto::kernel::KernelServiceClient;
//! ```

// 生成的代码在 gen 目录中
// v1 模块 - Package 核心类型
pub mod v1 {
    include!("gen/v1/loquat.v1.rs");
}

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

// 重新导出一些常用的 v1 类型
pub use v1::{Package, TargetSite, PayloadMeta, PoolState, WorkerInfo, ProcessResult};
