//! Loquat Engine - 插件运行时
//! 
//! Engine是Loquat的插件运行时层，负责：
//! - 插件加载和执行
//! - 适配器管理
//! - 与Kernel通信
//! - 沙箱隔离

pub mod config;
pub mod kernel_client;
pub mod engine;

pub use config::EngineConfig;
pub use kernel_client::KernelClient;
pub use engine::Engine;

pub mod error {
    use thiserror::Error;
    
    #[derive(Debug, Error)]
    pub enum EngineError {
        #[error("Configuration error: {0}")]
        ConfigError(String),
        
        #[error("Kernel connection error: {0}")]
        KernelConnectionError(String),
        
        #[error("Plugin load error: {0}")]
        PluginLoadError(String),
        
        #[error("Adapter error: {0}")]
        AdapterError(String),
        
        #[error("Sandbox error: {0}")]
        SandboxError(String),
        
        #[error("IO error: {0}")]
        IoError(#[from] std::io::Error),
        
        #[error("gRPC error: {0}")]
        GrpcError(#[from] tonic::transport::Error),
        
        #[error("gRPC status error: {0}")]
        GrpcStatus(#[from] tonic::Status),
    }
    
    pub type Result<T> = std::result::Result<T, EngineError>;
}
