use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock as TokioRwLock;
use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;

use crate::config::KernelConfig;
use crate::engine::EngineManager;
use crate::monitor::Monitor;
use crate::kernel::Kernel;

// 暂时禁用proto，因为proto编译有很多问题
// TODO: 修复proto编译后重新启用
// include!("loquat.common.rs");
// include!("loquat.kernel.rs");

// 临时的简化类型定义
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Empty {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineId {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub host: String,
    pub port: i32,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineList {
    pub engines: Vec<EngineInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HealthCheckResponse {
    pub healthy: bool,
    pub message: String,
    pub uptime: u64,
    pub engine_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemInfo {
    pub kernel_id: String,
    pub version: String,
    pub start_time: Option<i64>,
    pub uptime: Option<u64>,
    pub engine_count: i32,
    pub max_engines: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub values: std::collections::HashMap<String, String>,
    pub environment: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metric {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamMetricsRequest {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamLogsRequest {}

// 临时的trait定义
#[tonic::async_trait]
pub trait KernelService: Send + Sync + 'static {
    async fn register_engine(
        &self,
        request: Request<EngineInfo>,
    ) -> Result<Response<EngineId>, Status>;
    
    async fn unregister_engine(
        &self,
        request: Request<EngineId>,
    ) -> Result<Response<Empty>, Status>;
    
    async fn restart_engine(
        &self,
        request: Request<EngineId>,
    ) -> Result<Response<Empty>, Status>;
    
    async fn stop_engine(
        &self,
        request: Request<EngineId>,
    ) -> Result<Response<Empty>, Status>;
    
    async fn get_engine_status(
        &self,
        request: Request<EngineId>,
    ) -> Result<Response<EngineInfo>, Status>;
    
    async fn list_engines(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<EngineList>, Status>;
    
    async fn get_engine_info(
        &self,
        request: Request<EngineId>,
    ) -> Result<Response<EngineInfo>, Status>;
    
    async fn health_check(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<HealthCheckResponse>, Status>;
    
    async fn get_config(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<Config>, Status>;
    
    async fn update_config(
        &self,
        request: Request<Config>,
    ) -> Result<Response<Empty>, Status>;
    
    async fn reload_config(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<Empty>, Status>;
    
    async fn stream_metrics(
        &self,
        request: Request<StreamMetricsRequest>,
    ) -> Result<Response<ReceiverStream<Result<Metric, Status>>>, Status>;
    
    async fn stream_logs(
        &self,
        request: Request<StreamLogsRequest>,
    ) -> Result<Response<ReceiverStream<Result<LogEntry, Status>>>, Status>;
    
    async fn get_system_info(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<SystemInfo>, Status>;
    
    async fn shutdown(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<Empty>, Status>;
}

#[derive(Clone)]
pub struct GrpcServer {
    config: Arc<KernelConfig>,
    engine_manager: Arc<TokioRwLock<EngineManager>>,
    #[allow(dead_code)]
    monitor: Arc<TokioRwLock<Monitor>>,
    kernel: Arc<TokioRwLock<Kernel>>,
}

impl GrpcServer {
    pub fn new(
        config: Arc<KernelConfig>,
        engine_manager: Arc<TokioRwLock<EngineManager>>,
        monitor: Arc<TokioRwLock<Monitor>>,
        kernel: Arc<TokioRwLock<Kernel>>,
    ) -> Self {
        Self {
            config,
            engine_manager,
            monitor,
            kernel,
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let addr: SocketAddr = self.config.kernel.bind_address.parse()?;
        
        // TODO: 重新启用真正的gRPC服务器
        tracing::info!("gRPC server starting on {} (proto temporarily disabled)", addr);
        
        // tonic::transport::Server::builder()
        //     .add_service(svc)
        //     .serve(addr)
        //     .await
        //     .map_err(|e| KernelError::IoError(e.to_string()))?;

        Ok(())
    }

    pub async fn stop(&self) -> anyhow::Result<()> {
        tracing::info!("gRPC server stopping");
        Ok(())
    }
}

#[tonic::async_trait]
impl KernelService for GrpcServer {
    async fn register_engine(
        &self,
        request: Request<EngineInfo>,
    ) -> Result<Response<EngineId>, Status> {
        let info = request.into_inner();
        
        tracing::info!("Registering engine: {}", info.id);
        
        // 检查engine是否已存在
        let manager = self.engine_manager.read().await;
        if manager.contains_engine(&info.id) {
            drop(manager);
            return Err(Status::already_exists(format!(
                "Engine {} already exists",
                info.id
            )));
        }
        drop(manager);
        
        // 创建EngineInfo
        let host = info.host.clone();
        let name = info.name.clone();
        let port = info.port as u16;
        
        // 注册到管理器
        let manager = self.engine_manager.write().await;
        let engine_id = manager.register(name, host)
            .map_err(|e| Status::internal(e.to_string()))?;
        drop(manager);
        
        // 更新端口
        self.engine_manager.write().await.update_engine(&engine_id, |engine_info| {
            engine_info.port = port;
            engine_info.status = crate::engine::EngineStatus::Running;
        });
        
        tracing::info!("Engine {} registered successfully", engine_id);
        
        Ok(Response::new(EngineId { id: engine_id }))
    }

    async fn unregister_engine(
        &self,
        request: Request<EngineId>,
    ) -> Result<Response<Empty>, Status> {
        let engine_id = request.into_inner().id;
        
        tracing::info!("Unregistering engine: {}", engine_id);
        
        let manager = self.engine_manager.write().await;
        manager.unregister(&engine_id)
            .map_err(|e| Status::internal(e.to_string()))?;
        drop(manager);
        
        tracing::info!("Engine {} unregistered successfully", engine_id);
        
        Ok(Response::new(Empty {}))
    }

    async fn restart_engine(
        &self,
        request: Request<EngineId>,
    ) -> Result<Response<Empty>, Status> {
        let engine_id = request.into_inner().id;
        
        tracing::info!("Restarting engine: {}", engine_id);
        
        // TODO: 实现实际的进程重启逻辑
        // 这里暂时只更新状态
        let manager = self.engine_manager.write().await;
        if !manager.update_engine(&engine_id, |info| {
            info.status = crate::engine::EngineStatus::Restarting;
            info.uptime = std::time::Duration::ZERO;
        }) {
            drop(manager);
            return Err(Status::not_found(format!("Engine {} not found", engine_id)));
        }
        drop(manager);
        
        tracing::info!("Engine {} restarted successfully", engine_id);
        
        Ok(Response::new(Empty {}))
    }

    async fn stop_engine(
        &self,
        request: Request<EngineId>,
    ) -> Result<Response<Empty>, Status> {
        let engine_id = request.into_inner().id;
        
        tracing::info!("Stopping engine: {}", engine_id);
        
        // TODO: 实现实际的进程停止逻辑
        // 这里暂时只更新状态
        let manager = self.engine_manager.write().await;
        if !manager.update_engine(&engine_id, |info| {
            info.status = crate::engine::EngineStatus::Stopped;
        }) {
            drop(manager);
            return Err(Status::not_found(format!("Engine {} not found", engine_id)));
        }
        drop(manager);
        
        tracing::info!("Engine {} stopped successfully", engine_id);
        
        Ok(Response::new(Empty {}))
    }

    async fn get_engine_status(
        &self,
        request: Request<EngineId>,
    ) -> Result<Response<EngineInfo>, Status> {
        let engine_id = request.into_inner().id;
        
        let manager = self.engine_manager.read().await;
        match manager.get(&engine_id) {
            Some(info) => {
                let response = Response::new(EngineInfo {
                    id: info.engine_id.clone(),
                    name: info.name.clone(),
                    version: "0.2.0".to_string(),
                    host: info.host.clone(),
                    port: info.port as i32,
                    metadata: std::collections::HashMap::new(),
                });
                drop(manager);
                Ok(response)
            }
            None => {
                drop(manager);
                Err(Status::not_found(format!("Engine {} not found", engine_id)))
            }
        }
    }

    async fn list_engines(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<EngineList>, Status> {
        let manager = self.engine_manager.read().await;
        let engines = manager.list();
        drop(manager);
        
        let proto_engines: Vec<EngineInfo> = engines
            .iter()
            .map(|info| EngineInfo {
                id: info.engine_id.clone(),
                name: info.name.clone(),
                version: "0.2.0".to_string(),
                host: info.host.clone(),
                port: info.port as i32,
                metadata: std::collections::HashMap::new(),
            })
            .collect();
        
        Ok(Response::new(EngineList { engines: proto_engines }))
    }

    async fn get_engine_info(
        &self,
        request: Request<EngineId>,
    ) -> Result<Response<EngineInfo>, Status> {
        let engine_id = request.into_inner().id;
        
        let manager = self.engine_manager.read().await;
        match manager.get(&engine_id) {
            Some(info) => {
                let response = Response::new(EngineInfo {
                    id: info.engine_id.clone(),
                    name: info.name.clone(),
                    version: "0.2.0".to_string(),
                    host: info.host.clone(),
                    port: info.port as i32,
                    metadata: std::collections::HashMap::new(),
                });
                drop(manager);
                Ok(response)
            }
            None => {
                drop(manager);
                Err(Status::not_found(format!("Engine {} not found", engine_id)))
            }
        }
    }

    async fn health_check(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let kernel = self.kernel.read().await;
        let info = kernel.get_info();
        drop(kernel);
        let manager = self.engine_manager.read().await;
        let running_count = manager.count();
        drop(manager);
        
        Ok(Response::new(HealthCheckResponse {
            healthy: true,
            message: "Kernel is healthy".to_string(),
            uptime: info.uptime.num_seconds() as u64,
            engine_count: running_count as u32,
        }))
    }

    async fn get_config(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Config>, Status> {
        // TODO: 实现配置序列化
        Ok(Response::new(Config {
            values: std::collections::HashMap::new(),
            environment: "production".to_string(),
        }))
    }

    async fn update_config(
        &self,
        _request: Request<Config>,
    ) -> Result<Response<Empty>, Status> {
        tracing::info!("Updating kernel configuration");
        
        // TODO: 实现配置验证和应用
        // 暂时只记录日志
        
        Ok(Response::new(Empty {}))
    }

    async fn reload_config(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Empty>, Status> {
        tracing::info!("Reloading kernel configuration");
        
        // TODO: 实现配置重载
        
        Ok(Response::new(Empty {}))
    }

    async fn stream_metrics(
        &self,
        _request: Request<StreamMetricsRequest>,
    ) -> Result<Response<ReceiverStream<Result<Metric, Status>>>, Status> {
        let (_tx, rx) = tokio::sync::mpsc::channel(100);
        
        // TODO: 实现实际的指标流
        // 暂时返回空流
        
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn stream_logs(
        &self,
        _request: Request<StreamLogsRequest>,
    ) -> Result<Response<ReceiverStream<Result<LogEntry, Status>>>, Status> {
        let (_tx, rx) = tokio::sync::mpsc::channel(100);
        
        // TODO: 实现实际的日志流
        // 暂时返回空流
        
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn get_system_info(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<SystemInfo>, Status> {
        let kernel = self.kernel.read().await;
        let info = kernel.get_info();
        drop(kernel);
        
        Ok(Response::new(SystemInfo {
            kernel_id: info.kernel_id,
            version: info.version,
            start_time: Some(info.start_time.timestamp()),
            uptime: Some(info.uptime.num_seconds() as u64),
            engine_count: info.engine_count as i32,
            max_engines: self.config.kernel.max_engines as i32,
        }))
    }

    async fn shutdown(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Empty>, Status> {
        tracing::info!("Shutting down kernel");
        
        // TODO: 实现优雅关闭
        
        Ok(Response::new(Empty {}))
    }
}
