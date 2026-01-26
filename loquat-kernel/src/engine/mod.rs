//! Engine管理器 - 负责管理Engine进程的生命周期

use crate::config::KernelConfig;
use crate::error::KernelError;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use uuid::Uuid;
use chrono::Utc;
use tracing::{info, error, warn};
use tokio::process::Child;

/// Engine信息
#[derive(Debug, Clone)]
pub struct EngineInfo {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub pid: Option<u32>,
    pub status: EngineStatus,
    pub start_time: chrono::DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Engine状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Error { message: String },
}

/// Engine管理器
pub struct EngineManager {
    config: KernelConfig,
    engines: RwLock<HashMap<String, EngineInfo>>,
    next_port: RwLock<u16>,
}

impl EngineManager {
    /// 创建新的Engine管理器
    pub fn new(config: KernelConfig) -> Self {
        Self {
            config,
            engines: RwLock::new(HashMap::new()),
            next_port: RwLock::new(config.engine.default_port_range[0]),
        }
    }
    
    /// 注册新Engine
    pub fn register(&self, name: String, host: String) -> Result<String, KernelError> {
        let engines = self.engines.read();
        
        // 检查Engine数量限制
        if engines.len() >= self.config.kernel.max_engines {
            return Err(KernelError::EngineLimitReached(
                format!("Maximum engine limit ({}) reached", self.config.kernel.max_engines)
            ));
        }
        
        // 检查名称是否已存在
        if engines.values().any(|e| e.name == name) {
            return Err(KernelError::RegistrationFailed(
                format!("Engine name '{}' already exists", name)
            ));
        }
        
        drop(engines);
        
        // 分配端口
        let port = {
            let mut next_port = self.next_port.write();
            let port = *next_port;
            *next_port = if *next_port >= self.config.engine.default_port_range[1] {
                self.config.engine.default_port_range[0]
            } else {
                *next_port + 1
            };
            port
        };
        
        let id = Uuid::new_v4().to_string();
        let engine_info = EngineInfo {
            id: id.clone(),
            name: name.clone(),
            host,
            port,
            pid: None,
            status: EngineStatus::Starting,
            start_time: Utc::now(),
            metadata: HashMap::new(),
        };
        
        let mut engines = self.engines.write();
        engines.insert(id.clone(), engine_info);
        
        info!("Registered engine '{}' with ID: {}", name, id);
        Ok(id)
    }
    
    /// 注销Engine
    pub fn unregister(&self, id: &str) -> Result<(), KernelError> {
        let mut engines = self.engines.write();
        
        if let Some(engine) = engines.remove(id) {
            info!("Unregistered engine '{}'", engine.name);
            Ok(())
        } else {
            Err(KernelError::EngineNotFound(id.to_string()))
        }
    }
    
    /// 更新Engine状态
    pub fn update_status(&self, id: &str, status: EngineStatus, pid: Option<u32>) {
        let mut engines = self.engines.write();
        
        if let Some(engine) = engines.get_mut(id) {
            engine.status = status;
            engine.pid = pid;
            info!("Engine {} status updated: {:?}", id, status);
        } else {
            warn!("Attempted to update status for non-existent engine: {}", id);
        }
    }
    
    /// 获取Engine信息
    pub fn get(&self, id: &str) -> Option<EngineInfo> {
        self.engines.read().get(id).cloned()
    }
    
    /// 列出所有Engine
    pub fn list(&self) -> Vec<EngineInfo> {
        self.engines.read().values().cloned().collect()
    }
    
    /// 获取Engine数量
    pub fn count(&self) -> usize {
        self.engines.read().len()
    }
    
    /// 停止所有Engine
    pub async fn stop_all(&self) -> anyhow::Result<()> {
        let engines: Vec<String> = self.engines.read().keys().cloned().collect();
        
        for id in engines {
            if let Err(e) = self.stop(&id).await {
                error!("Failed to stop engine {}: {:?}", id, e);
            }
        }
        
        Ok(())
    }
    
    /// 停止指定Engine
    pub async fn stop(&self, id: &str) -> Result<(), KernelError> {
        // TODO: 实现实际的进程停止逻辑
        warn!("Stopping engine {} (not yet implemented)", id);
        Ok(())
    }
}
