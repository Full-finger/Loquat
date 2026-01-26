//! Kernel核心模块

use crate::config::KernelConfig;
use crate::engine::EngineManager;
use crate::monitor::Monitor;
use crate::api::{GrpcServer, HttpServer};
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::Utc;
use uuid::Uuid;
use tracing::{info, error, warn};

/// Kernel主结构
pub struct Kernel {
    config: KernelConfig,
    engine_manager: Arc<EngineManager>,
    monitor: Arc<Monitor>,
    grpc_server: Option<GrpcServer>,
    http_server: Option<HttpServer>,
    start_time: chrono::DateTime<Utc>,
    kernel_id: String,
}

impl Kernel {
    /// 创建新的Kernel实例
    pub fn new(config: KernelConfig) -> Self {
        let kernel_id = config.kernel.kernel_id.clone();
        let engine_manager = Arc::new(EngineManager::new(config.clone()));
        let monitor = Arc::new(Monitor::new(
            config.monitoring.clone(),
            Arc::clone(&engine_manager),
        ));
        
        Self {
            config,
            engine_manager,
            monitor,
            grpc_server: None,
            http_server: None,
            start_time: Utc::now(),
            kernel_id,
        }
    }
    
    /// 启动Kernel
    pub async fn start(&mut self) -> anyhow::Result<()> {
        info!("Starting Loquat Kernel (ID: {})", self.kernel_id);
        
        // 启动监控器
        self.monitor.start().await?;
        
        // 启动gRPC服务器
        let grpc_addr = self.config.kernel.bind_address.parse()?;
        let grpc_server = GrpcServer::new(
            grpc_addr,
            Arc::clone(&self.engine_manager),
        );
        self.grpc_server = Some(grpc_server);
        
        // 启动HTTP服务器
        let http_addr = self.config.kernel.web_address.parse()?;
        let http_server = HttpServer::new(
            http_addr,
            Arc::clone(&self.engine_manager),
        );
        self.http_server = Some(http_server);
        
        info!("Kernel started successfully");
        Ok(())
    }
    
    /// 停止Kernel
    pub async fn stop(&mut self) -> anyhow::Result<()> {
        info!("Stopping Loquat Kernel");
        
        // 停止HTTP服务器
        if let Some(http_server) = self.http_server.take() {
            http_server.stop().await?;
        }
        
        // 停止gRPC服务器
        if let Some(grpc_server) = self.grpc_server.take() {
            grpc_server.stop().await?;
        }
        
        // 停止监控器
        self.monitor.stop().await?;
        
        // 停止所有Engine
        self.engine_manager.stop_all().await?;
        
        info!("Kernel stopped successfully");
        Ok(())
    }
    
    /// 获取Kernel信息
    pub fn get_info(&self) -> KernelInfo {
        KernelInfo {
            kernel_id: self.kernel_id.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            start_time: self.start_time,
            uptime: Utc::now() - self.start_time,
            engine_count: self.engine_manager.count(),
        }
    }
}

/// Kernel信息
#[derive(Debug, Clone)]
pub struct KernelInfo {
    pub kernel_id: String,
    pub version: String,
    pub start_time: chrono::DateTime<Utc>,
    pub uptime: chrono::Duration,
    pub engine_count: usize,
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new(KernelConfig::default())
    }
}
