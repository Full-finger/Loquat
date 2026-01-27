//! Loquat Kernel - 微内核
//! 
//! 负责管理Engine生命周期、进程监控、资源分配

pub mod config;
pub mod kernel;
pub mod engine;
pub mod monitor;
pub mod api;
pub mod grpc_server;
pub mod http_server;
pub mod process_manager;

pub use config::KernelConfig;
pub use kernel::Kernel;
pub use engine::EngineManager;
pub use monitor::Monitor;
pub use grpc_server::GrpcServer;
pub use http_server::HttpServer;
pub use process_manager::{ProcessManager, ProcessHandle};

pub mod error {
    use thiserror::Error;
    
    #[derive(Debug, Error)]
    pub enum KernelError {
        #[error("Engine not found: {0}")]
        EngineNotFound(String),
        
        #[error("Engine registration failed: {0}")]
        RegistrationFailed(String),
        
        #[error("Engine limit reached: {0}")]
        EngineLimitReached(String),
        
        #[error("Process error: {0}")]
        ProcessError(String),
        
        #[error("Configuration error: {0}")]
        ConfigError(String),
        
        #[error("IO error: {0}")]
        IoError(#[from] std::io::Error),
        
        #[error("gRPC error: {0}")]
        GrpcError(#[from] tonic::transport::Error),
    }
    
    pub type Result<T> = std::result::Result<T, KernelError>;
}
