use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status, Streaming};
use tokio_stream::wrappers::ReceiverStream;

use crate::config::KernelConfig;
use crate::engine::EngineManager;
use crate::monitor::Monitor;
use crate::kernel::Kernel;
use crate::error::{KernelError, Result};

// 引入proto生成的模块
pub mod kernel_proto {
    tonic::include_proto!("kernel");
}

use kernel_proto::{
    kernel_service_server::KernelService,
    Empty, Error, EngineId, EngineInfo as ProtoEngineInfo, Config,
    Metric, LogEntry, EngineStatus as ProtoEngineStatus,
    HealthCheckResponse, SystemInfo, EngineList, StreamMetricsRequest, StreamLogsRequest,
};

// 将本地EngineInfo转换为proto格式
fn to_proto_engine_info(info: &crate::engine::EngineInfo) -> ProtoEngineInfo {
    ProtoEngineInfo {
        engine_id: info.engine_id.clone(),
        name: info.name.clone(),
        port: info.port as u32,
        status: match info.status {
            crate::engine::EngineStatus::Starting => ProtoEngineStatus::Starting.into(),
            crate::engine::EngineStatus::Running => ProtoEngineStatus::Running.into(),
            crate::engine::EngineStatus::Stopping => ProtoEngineStatus::Stopping.into(),
            crate::engine::EngineStatus::Stopped => ProtoEngineStatus::Stopped.into(),
            crate::engine::EngineStatus::Error => ProtoEngineStatus::Error.into(),
        },
        pid: info.pid.unwrap_or(0) as u64,
        uptime: info.uptime.as_secs() as u64,
        last_heartbeat: info.last_heartbeat.map(|t| t.timestamp()).unwrap_or(0),
    }
}

#[derive(Clone)]
pub struct GrpcServer {
    config: Arc<KernelConfig>,
    engine_manager: Arc<RwLock<EngineManager>>,
    monitor: Arc<RwLock<Monitor>>,
    kernel: Arc<RwLock<Kernel>>,
}

impl GrpcServer {
    pub fn new(
        config: Arc<KernelConfig>,
        engine_manager: Arc<RwLock<EngineManager>>,
        monitor: Arc<RwLock<Monitor>>,
        kernel: Arc<RwLock<Kernel>>,
    ) -> Self {
        Self {
            config,
            engine_manager,
            monitor,
            kernel,
        }
    }

    pub async fn run(&self) -> Result<()> {
        let addr: SocketAddr = self.config.kernel.grpc_address.parse()?;
        
        let svc = KernelServiceServer::new(self.clone());
        
        tracing::info!("gRPC server starting on {}", addr);
        
        tonic::transport::Server::builder()
            .add_service(svc)
            .serve(addr)
            .await
            .map_err(|e| KernelError::IoError(e.to_string()))?;

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        tracing::info!("gRPC server stopping");
        Ok(())
    }
}

#[tonic::async_trait]
impl KernelService for GrpcServer {
    async fn register_engine(
        &self,
        request: Request<ProtoEngineInfo>,
    ) -> Result<Response<EngineId>, Status> {
        let info = request.into_inner();
        
        tracing::info!("Registering engine: {}", info.engine_id);
        
        let mut manager = self.engine_manager.write().await;
        
        // 检查engine是否已存在
        if manager.get(&info.engine_id).is_some() {
            return Err(Status::already_exists(format!(
                "Engine {} already exists",
                info.engine_id
            )));
        }
        
        // 创建EngineInfo
        let engine_info = crate::engine::EngineInfo {
            engine_id: info.engine_id.clone(),
            name: info.name.clone(),
            port: info.port as u16,
            status: crate::engine::EngineStatus::Running,
            pid: Some(info.pid as u32),
            uptime: std::time::Duration::from_secs(info.uptime),
            last_heartbeat: Some(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(info.last_heartbeat)),
        };
        
        // 注册到管理器
        manager.register(engine_info)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        tracing::info!("Engine {} registered successfully", info.engine_id);
        
        Ok(Response::new(EngineId { id: info.engine_id }))
    }

    async fn unregister_engine(
        &self,
        request: Request<EngineId>,
    ) -> Result<Response<Empty>, Status> {
        let engine_id = request.into_inner().id;
        
        tracing::info!("Unregistering engine: {}", engine_id);
        
        let mut manager = self.engine_manager.write().await;
        
        manager.unregister(&engine_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
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
        let mut manager = self.engine_manager.write().await;
        
        if let Some(mut info) = manager.get_mut(&engine_id) {
            info.status = crate::engine::EngineStatus::Restarting;
            info.uptime = std::time::Duration::ZERO;
        } else {
            return Err(Status::not_found(format!("Engine {} not found", engine_id)));
        }
        
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
        let mut manager = self.engine_manager.write().await;
        
        if let Some(info) = manager.get_mut(&engine_id) {
            info.status = crate::engine::EngineStatus::Stopped;
        } else {
            return Err(Status::not_found(format!("Engine {} not found", engine_id)));
        }
        
        tracing::info!("Engine {} stopped successfully", engine_id);
        
        Ok(Response::new(Empty {}))
    }

    async fn get_engine_status(
        &self,
        request: Request<EngineId>,
    ) -> Result<Response<ProtoEngineInfo>, Status> {
        let engine_id = request.into_inner().id;
        
        let manager = self.engine_manager.read().await;
        
        match manager.get(&engine_id) {
            Some(info) => Ok(Response::new(to_proto_engine_info(info))),
            None => Err(Status::not_found(format!("Engine {} not found", engine_id))),
        }
    }

    async fn list_engines(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<EngineList>, Status> {
        let manager = self.engine_manager.read().await;
        
        let engines = manager.list()
            .iter()
            .map(to_proto_engine_info)
            .collect();
        
        Ok(Response::new(EngineList { engines }))
    }

    async fn health_check(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let kernel = self.kernel.read().await;
        let info = kernel.get_info().await;
        
        let manager = self.engine_manager.read().await;
        let running_count = manager.count().await;
        
        Ok(Response::new(HealthCheckResponse {
            healthy: true,
            message: "Kernel is healthy".to_string(),
            uptime: info.uptime.as_secs() as u64,
            engine_count: running_count as u32,
        }))
    }

    async fn get_system_info(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<SystemInfo>, Status> {
        let kernel = self.kernel.read().await;
        let info = kernel.get_info().await;
        
        Ok(Response::new(SystemInfo {
            kernel_id: info.kernel_id,
            version: info.version,
            uptime: info.uptime.as_secs() as u64,
            engine_count: info.engine_count as u32,
        }))
    }

    async fn get_config(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Config>, Status> {
        // TODO: 实现配置序列化
        Ok(Response::new(Config {
            data: serde_json::to_string(&*self.config).unwrap_or_default(),
        }))
    }

    async fn set_config(
        &self,
        request: Request<Config>,
    ) -> Result<Response<Empty>, Status> {
        let config_data = request.into_inner().data;
        
        tracing::info!("Updating kernel configuration");
        
        // TODO: 实现配置验证和应用
        // 暂时只记录日志
        
        Ok(Response::new(Empty {}))
    }

    type StreamMetricsStream = ReceiverStream<Result<Metric, Status>>;
    
    async fn stream_metrics(
        &self,
        _request: Request<StreamMetricsRequest>,
    ) -> Result<Response<Self::StreamMetricsStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        // TODO: 实现实际的指标流
        // 暂时返回空流
        
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    type StreamLogsStream = ReceiverStream<Result<LogEntry, Status>>;
    
    async fn stream_logs(
        &self,
        _request: Request<StreamLogsRequest>,
    ) -> Result<Response<Self::StreamLogsStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        // TODO: 实现实际的日志流
        // 暂时返回空流
        
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
