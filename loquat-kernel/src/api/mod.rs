//! API服务器 - gRPC和HTTP服务

use crate::engine::EngineManager;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::info;

/// gRPC服务器
pub struct GrpcServer {
    addr: SocketAddr,
    engine_manager: Arc<EngineManager>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl GrpcServer {
    /// 创建新的gRPC服务器
    pub fn new(addr: SocketAddr, engine_manager: Arc<EngineManager>) -> Self {
        Self {
            addr,
            engine_manager,
            shutdown_tx: None,
        }
    }
    
    /// 启动gRPC服务器
    pub async fn run(&mut self) -> anyhow::Result<()> {
        info!("Starting gRPC server on {}", self.addr);
        
        // TODO: 实现实际的gRPC服务
        // 需要使用tonic::Server构建服务
        // 实现KernelService trait
        
        Ok(())
    }
    
    /// 停止gRPC服务器
    pub async fn stop(&mut self) -> anyhow::Result<()> {
        info!("Stopping gRPC server");
        
        // TODO: 实现优雅关闭
        
        Ok(())
    }
}

/// HTTP服务器
pub struct HttpServer {
    addr: SocketAddr,
    engine_manager: Arc<EngineManager>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl HttpServer {
    /// 创建新的HTTP服务器
    pub fn new(addr: SocketAddr, engine_manager: Arc<EngineManager>) -> Self {
        Self {
            addr,
            engine_manager,
            shutdown_tx: None,
        }
    }
    
    /// 启动HTTP服务器
    pub async fn run(&mut self) -> anyhow::Result<()> {
        info!("Starting HTTP server on {}", self.addr);
        
        // TODO: 实现实际的HTTP服务
        // 需要使用axum构建路由
        // 实现RESTful API端点
        
        Ok(())
    }
    
    /// 停止HTTP服务器
    pub async fn stop(&mut self) -> anyhow::Result<()> {
        info!("Stopping HTTP server");
        
        // TODO: 实现优雅关闭
        
        Ok(())
    }
}
