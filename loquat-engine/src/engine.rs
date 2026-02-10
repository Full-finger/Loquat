use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::EngineConfig;
use crate::kernel_client::KernelClient;
use crate::error::{EngineError, Result};

/// Engine - 插件运行时核心
#[derive(Clone)]
pub struct Engine {
    config: Arc<EngineConfig>,
    kernel_client: Arc<KernelClient>,
    running: Arc<RwLock<bool>>,
}

impl Engine {
    /// 创建新的Engine实例
    pub async fn new(config: Arc<EngineConfig>) -> Result<Self> {
        let kernel_client = KernelClient::new(config.clone()).await?;
        let engine_id = config.engine.engine_id.clone();
        
        tracing::info!("Creating Engine {}", engine_id);
        
        Ok(Self {
            config,
            kernel_client: Arc::new(kernel_client),
            running: Arc::new(RwLock::new(false)),
        })
    }
    
    /// 启动Engine
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Ok(());
        }
        *running = true;
        drop(running);
        
        tracing::info!("Starting Engine {}", self.config.engine.engine_id);
        
        // 连接到Kernel
        self.kernel_client.connect().await?;
        
        // TODO: 启动HTTP API服务
        // TODO: 启动gRPC服务
        // TODO: 加载插件
        
        tracing::info!("Engine {} started successfully", self.config.engine.engine_id);
        
        Ok(())
    }
    
    /// 停止Engine
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }
        *running = false;
        drop(running);
        
        tracing::info!("Stopping Engine {}", self.config.engine.engine_id);
        
        // 从Kernel断开
        self.kernel_client.disconnect().await?;
        
        // TODO: 停止HTTP API服务
        // TODO: 停止gRPC服务
        // TODO: 卸载插件
        
        tracing::info!("Engine {} stopped", self.config.engine.engine_id);
        
        Ok(())
    }
    
    /// 检查是否正在运行
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
    
    /// 获取配置
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }
    
    /// 获取Kernel客户端
    pub fn kernel_client(&self) -> &KernelClient {
        &self.kernel_client
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_engine_creation() {
        let config = Arc::new(EngineConfig::default());
        // 这个测试需要实际的Kernel运行
        // 暂时跳过
    }
}
