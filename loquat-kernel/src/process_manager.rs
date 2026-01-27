use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::process::{Child, Command};

use crate::config::KernelConfig;
use crate::engine::{EngineInfo, EngineStatus};
use crate::error::{KernelError, Result};

#[derive(Clone)]
pub struct ProcessHandle {
    pub engine_id: String,
    pub pid: u32,
    pub start_time: std::time::Instant,
}

pub struct ProcessManager {
    config: Arc<KernelConfig>,
    processes: Arc<RwLock<HashMap<String, Child>>>,
    engine_binary: PathBuf,
}

impl ProcessManager {
    pub fn new(config: Arc<KernelConfig>) -> Self {
        // 查找loquat-engine二进制文件
        let engine_binary = Self::find_engine_binary();
        
        tracing::info!("Engine binary located at: {:?}", engine_binary);
        
        Self {
            config,
            processes: Arc::new(RwLock::new(HashMap::new())),
            engine_binary,
        }
    }
    
    fn find_engine_binary() -> PathBuf {
        // 尝试多个可能的路径
        let possible_paths = vec![
            "./target/debug/loquat-engine".into(),
            "./target/release/loquat-engine".into(),
            "./loquat-engine/target/debug/loquat-engine".into(),
            "./loquat-engine/target/release/loquat-engine".into(),
        ];
        
        for path in possible_paths {
            if path.exists() {
                return path;
            }
        }
        
        // 默认路径
        PathBuf::from("./target/debug/loquat-engine")
    }
    
    pub async fn start_engine(
        &self,
        engine_info: &EngineInfo,
    ) -> Result<ProcessHandle> {
        let engine_id = &engine_info.engine_id;
        let port = engine_info.port;
        
        tracing::info!("Starting engine {} on port {}", engine_id, port);
        
        // 构建命令行参数
        let args = vec![
            "--engine-id".to_string(),
            engine_id.clone(),
            "--kernel-address".to_string(),
            self.config.kernel.grpc_address.clone(),
            "--port".to_string(),
            port.to_string(),
        ];
        
        // 准备环境变量
        let mut envs = std::env::vars().collect::<HashMap<_, _>>();
        envs.insert("LOQUAT_ENGINE_ID".to_string(), engine_id.clone());
        envs.insert("LOQUAT_ENGINE_PORT".to_string(), port.to_string());
        envs.insert("RUST_LOG".to_string(), "info".to_string());
        
        // 启动进程
        let child = Command::new(&self.engine_binary)
            .args(&args)
            .envs(&envs)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                KernelError::ProcessError(format!(
                    "Failed to start engine {}: {}",
                    engine_id, e
                ))
            })?;
        
        let pid = child.id().ok_or_else(|| {
            KernelError::ProcessError(format!(
                "Failed to get PID for engine {}",
                engine_id
            ))
        })?;
        
        // 存储进程句柄
        let mut processes = self.processes.write().await;
        processes.insert(engine_id.clone(), child);
        
        let handle = ProcessHandle {
            engine_id: engine_id.clone(),
            pid,
            start_time: std::time::Instant::now(),
        };
        
        tracing::info!("Engine {} started with PID {}", engine_id, pid);
        
        Ok(handle)
    }
    
    pub async fn stop_engine(&self, engine_id: &str) -> Result<()> {
        tracing::info!("Stopping engine {}", engine_id);
        
        let mut processes = self.processes.write().await;
        
        if let Some(mut child) = processes.remove(engine_id) {
            // 先尝试优雅关闭（SIGTERM）
            let result = tokio::time::timeout(
                Duration::from_secs(10),
                async {
                    if let Err(e) = child.start_kill() {
                        tracing::warn!("Failed to send SIGTERM to {}: {}", engine_id, e);
                    }
                    
                    // 等待进程退出
                    child.wait().await
                }
            ).await;
            
            match result {
                Ok(Ok(exit_status)) => {
                    tracing::info!(
                        "Engine {} stopped with status: {}",
                        engine_id,
                        exit_status
                    );
                }
                Ok(Err(e)) => {
                    tracing::warn!("Error waiting for engine {} to stop: {}", engine_id, e);
                }
                Err(_) => {
                    // 超时，强制杀死
                    tracing::warn!("Engine {} did not stop gracefully, killing...", engine_id);
                    if let Err(e) = child.kill() {
                        tracing::warn!("Failed to kill engine {}: {}", engine_id, e);
                    }
                }
            }
            
            Ok(())
        } else {
            Err(KernelError::NotFound(format!(
                "Process for engine {} not found",
                engine_id
            )))
        }
    }
    
    pub async fn restart_engine(
        &self,
        engine_info: &EngineInfo,
    ) -> Result<ProcessHandle> {
        let engine_id = &engine_info.engine_id;
        
        tracing::info!("Restarting engine {}", engine_id);
        
        // 先停止现有进程
        if let Err(e) = self.stop_engine(engine_id).await {
            tracing::warn!("Failed to stop engine {}: {}", engine_id, e);
        }
        
        // 等待一段时间确保资源释放
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // 启动新进程
        self.start_engine(engine_info).await
    }
    
    pub async fn is_running(&self, engine_id: &str) -> bool {
        let processes = self.processes.read().await;
        
        if let Some(child) = processes.get(engine_id) {
            if let Ok(Some(_)) = child.try_wait() {
                false // 进程已经退出
            } else {
                true // 进程仍在运行
            }
        } else {
            false // 进程不存在
        }
    }
    
    pub async fn get_process_count(&self) -> usize {
        let processes = self.processes.read().await;
        processes.len()
    }
    
    pub async fn cleanup_dead_processes(&self) -> Vec<String> {
        let mut dead_engines = Vec::new();
        let mut processes = self.processes.write().await;
        
        for (engine_id, child) in processes.iter_mut() {
            if let Ok(Some(status)) = child.try_wait() {
                tracing::warn!(
                    "Engine {} died unexpectedly with status: {}",
                    engine_id,
                    status
                );
                dead_engines.push(engine_id.clone());
            }
        }
        
        // 移除死掉的进程
        for engine_id in &dead_engines {
            processes.remove(engine_id);
        }
        
        dead_engines
    }
    
    pub async fn stop_all(&self) -> Result<()> {
        tracing::info!("Stopping all engine processes");
        
        let processes = self.processes.read().await;
        let engine_ids: Vec<String> = processes.keys().cloned().collect();
        drop(processes);
        
        for engine_id in engine_ids {
            if let Err(e) = self.stop_engine(&engine_id).await {
                tracing::error!("Failed to stop engine {}: {}", engine_id, e);
            }
        }
        
        Ok(())
    }
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        tracing::info!("ProcessManager dropped, cleaning up processes");
        // 注意：Drop trait不支持async，所以这里只能记录日志
        // 实际的清理应该在stop_all中异步完成
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_find_engine_binary() {
        let binary = ProcessManager::find_engine_binary();
        println!("Engine binary: {:?}", binary);
    }
}
