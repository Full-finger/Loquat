use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::config::KernelConfig;
use crate::engine::{EngineManager, EngineInfo as LocalEngineInfo, EngineStatus};
use crate::kernel::Kernel;

#[derive(Clone)]
pub struct HttpServer {
    config: Arc<KernelConfig>,
    engine_manager: Arc<RwLock<EngineManager>>,
    kernel: Arc<RwLock<Kernel>>,
}

impl HttpServer {
    pub fn new(
        config: Arc<KernelConfig>,
        engine_manager: Arc<RwLock<EngineManager>>,
        kernel: Arc<RwLock<Kernel>>,
    ) -> Self {
        Self {
            config,
            engine_manager,
            kernel,
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let app = Router::new()
            .route("/api/health", get(health_check))
            .route("/api/engines", get(list_engines).post(create_engine))
            .route("/api/engines/:id", get(get_engine).delete(delete_engine))
            .route("/api/engines/:id/restart", post(restart_engine))
            .route("/api/config", get(get_config).put(update_config))
            .route("/api/system/info", get(get_system_info))
            .with_state(self.clone());

        let addr: SocketAddr = self.config.kernel.bind_address.parse()?;
        
        tracing::info!("HTTP server starting on {}", addr);
        
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }

    pub async fn stop(&self) {
        tracing::info!("HTTP server stopping");
    }
}

// ===== API请求和响应结构 =====

#[derive(Debug, Serialize, Deserialize)]
pub struct EngineInfo {
    pub engine_id: String,
    pub name: String,
    pub port: u16,
    pub status: String,
    pub pid: Option<u32>,
    pub uptime: u64,
    pub last_heartbeat: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateEngineRequest {
    pub name: String,
    pub port: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

// ===== 转换函数 =====

fn to_api_engine_info(info: &LocalEngineInfo) -> EngineInfo {
    EngineInfo {
        engine_id: info.engine_id.clone(),
        name: info.name.clone(),
        port: info.port,
        status: match &info.status {
            EngineStatus::Starting => "Starting".to_string(),
            EngineStatus::Running => "Running".to_string(),
            EngineStatus::Stopping => "Stopping".to_string(),
            EngineStatus::Stopped => "Stopped".to_string(),
            EngineStatus::Restarting => "Restarting".to_string(),
            EngineStatus::Error { .. } => "Error".to_string(),
        },
        pid: info.pid,
        uptime: info.uptime.as_secs(),
        last_heartbeat: info.last_heartbeat.map(|_| chrono::Utc::now().timestamp()),
    }
}

// ===== API处理器 =====

async fn health_check(State(server): State<HttpServer>) -> Json<ApiResponse<serde_json::Value>> {
    let kernel = server.kernel.read().await;
    let info = kernel.get_info();
    
    let manager = server.engine_manager.read().await;
    let engine_count = manager.count();

    let health_data = serde_json::json!({
        "healthy": true,
        "message": "Kernel is healthy",
        "uptime": info.uptime.num_seconds(),
        "engine_count": engine_count,
        "version": info.version,
    });

    Json(ApiResponse::success(health_data))
}

async fn list_engines(State(server): State<HttpServer>) -> Json<ApiResponse<Vec<EngineInfo>>> {
    let manager = server.engine_manager.read().await;
    
    let engines: Vec<EngineInfo> = manager
        .list()
        .iter()
        .map(|info| to_api_engine_info(info))
        .collect();

    Json(ApiResponse::success(engines))
}

async fn get_engine(
    State(server): State<HttpServer>,
    Path(id): Path<String>,
) -> Json<ApiResponse<EngineInfo>> {
    let manager = server.engine_manager.read().await;
    
    match manager.get(&id) {
        Some(info) => Json(ApiResponse::success(to_api_engine_info(&info))),
        None => Json(ApiResponse::error(format!("Engine {} not found", id))),
    }
}

async fn create_engine(
    State(server): State<HttpServer>,
    Json(req): Json<CreateEngineRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let manager = server.engine_manager.read().await;
    
    // 检查是否达到最大数量限制
    let count = manager.count();
    if count >= server.config.kernel.max_engines {
        return Json(ApiResponse::<serde_json::Value>::error(
            format!("Maximum engine count ({}) reached", server.config.kernel.max_engines)
        ));
    }
    
    drop(manager);
    
    let manager = server.engine_manager.write().await;
    
    // 分配端口
    let port = match req.port {
        Some(p) => p,
        None => manager.get_next_available_port(),
    };
    
    // 注册
    let engine_id = match manager.register(req.name.clone(), "localhost".to_string()) {
        Ok(id) => id,
        Err(e) => {
            return Json(ApiResponse::<serde_json::Value>::error(e.to_string()));
        }
    };
    
    // 更新端口和状态
    manager.update_engine(&engine_id, |info| {
        info.port = port;
        info.status = EngineStatus::Running;
    });
    
    drop(manager);
    
    tracing::info!("Engine {} created (port: {})", engine_id, port);
    
    Json(ApiResponse::success(serde_json::json!({
        "engine_id": engine_id,
        "port": port
    })))
}

async fn delete_engine(
    State(server): State<HttpServer>,
    Path(id): Path<String>,
) -> Json<ApiResponse<()>> {
    let manager = server.engine_manager.read().await;
    
    if manager.get(&id).is_none() {
        return Json(ApiResponse::error(format!("Engine {} not found", id)));
    }
    
    drop(manager);
    
    let manager = server.engine_manager.write().await;
    
    // TODO: 实际停止进程
    manager.unregister(&id).ok();
    
    tracing::info!("Engine {} deleted", id);
    Json(ApiResponse::success(()))
}

async fn restart_engine(
    State(server): State<HttpServer>,
    Path(id): Path<String>,
) -> Json<ApiResponse<()>> {
    let manager = server.engine_manager.write().await;
    
    // 更新 engine 状态
    if manager.update_engine(&id, |info| {
        // TODO: 实际重启进程
        info.status = EngineStatus::Restarting;
        info.uptime = std::time::Duration::ZERO;
    }) {
        Json(ApiResponse::success(()))
    } else {
        tracing::info!("Engine {} not found", id);
        Json(ApiResponse::error(format!("Engine {} not found", id)))
    }
}

async fn get_config(State(server): State<HttpServer>) -> Json<ApiResponse<serde_json::Value>> {
    // TODO: 实现配置序列化
    let config_data = serde_json::to_value(&*server.config).unwrap_or_default();
    Json(ApiResponse::success(config_data))
}

async fn update_config(
    State(_server): State<HttpServer>,
    Json(_new_config): Json<serde_json::Value>,
) -> Json<ApiResponse<()>> {
    // TODO: 实现配置验证和应用
    tracing::info!("Configuration update requested");
    
    Json(ApiResponse::success(()))
}

async fn get_system_info(State(server): State<HttpServer>) -> Json<ApiResponse<serde_json::Value>> {
    let kernel = server.kernel.read().await;
    let info = kernel.get_info();
    
    let system_info = serde_json::json!({
        "kernel_id": info.kernel_id,
        "version": info.version,
        "uptime": info.uptime.num_seconds(),
        "engine_count": info.engine_count,
    });

    Json(ApiResponse::success(system_info))
}
