//! Kernel核心模块

use crate::config::KernelConfig;
use crate::engine::EngineManager;
use crate::grpc_server::GrpcServer;
use crate::http_server::HttpServer;
use crate::monitor::Monitor;
use crate::process_manager::ProcessManager;
use std::sync::Arc;
use tokio::sync::RwLock as TokioRwLock;
use tracing::info;
use chrono::Utc;

/// Kernel主结构
#[derive(Clone)]
pub struct Kernel {
    config: KernelConfig,
    engine_manager: Arc<EngineManager>,
    monitor: Arc<Monitor>,
    #[allow(dead_code)]
    process_manager: Arc<ProcessManager>,
    start_time: chrono::DateTime<Utc>,
    kernel_id: String,
}

impl Kernel {
    /// 创建新的Kernel实例
    pub fn new(config: KernelConfig) -> Self {
        let kernel_id = config.kernel.kernel_id.clone();
        let engine_manager = Arc::new(EngineManager::new(config.clone()));
        let process_manager = Arc::new(ProcessManager::new(Arc::new(config.clone())));
        let monitor = Arc::new(Monitor::new(
            config.monitoring.clone(),
            Arc::new(TokioRwLock::new((*engine_manager).clone())),
            process_manager.clone(),
        ));
        
        Self {
            config,
            engine_manager,
            monitor,
            process_manager,
            start_time: Utc::now(),
            kernel_id,
        }
    }
    
    /// 启动Kernel
    pub async fn start(&self) -> anyhow::Result<()> {
        info!("Starting Loquat Kernel (ID: {})", self.kernel_id);
        
        // 启动监控器
        self.monitor.start().await?;
        
        info!("Kernel started successfully");
        Ok(())
    }
    
    /// 启动gRPC服务器
    pub fn start_grpc(&self) -> GrpcServer {
        GrpcServer::new(
            Arc::new(self.config.clone()),
            Arc::new(TokioRwLock::new((*self.engine_manager).clone())),
            Arc::new(TokioRwLock::new((*self.monitor).clone())),
            Arc::new(TokioRwLock::new(self.clone())),
        )
    }
    
    /// 启动HTTP服务器
    pub fn start_http(&self) -> HttpServer {
        HttpServer::new(
            Arc::new(self.config.clone()),
            Arc::new(TokioRwLock::new((*self.engine_manager).clone())),
            Arc::new(TokioRwLock::new(self.clone())),
        )
    }
    
    /// 停止Kernel
    pub async fn stop(&self) -> anyhow::Result<()> {
        info!("Stopping Loquat Kernel");
        
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
