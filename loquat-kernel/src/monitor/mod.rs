//! 监控器 - 负责健康检查和指标收集

use crate::engine::{EngineManager, EngineStatus};
use crate::config::MonitoringSection;
use crate::process_manager::ProcessManager;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tracing::{info, warn, error, debug};

/// 健康检查结果
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub engine_id: String,
    pub healthy: bool,
    pub message: String,
    pub latency_ms: u64,
}

/// 指标数据
#[derive(Debug, Clone)]
pub struct Metric {
    pub engine_id: String,
    pub timestamp: i64,
    pub metric_type: String,
    pub value: f64,
}

/// 监控事件
#[derive(Debug, Clone)]
pub enum MonitorEvent {
    EngineUnhealthy { engine_id: String, reason: String },
    EngineRestarted { engine_id: String },
    MetricCollected { metric: Metric },
    HealthCheckPassed { engine_id: String },
}

/// 监控器
pub struct Monitor {
    config: MonitoringSection,
    engine_manager: Arc<RwLock<EngineManager>>,
    process_manager: Arc<ProcessManager>,
    running: Arc<RwLock<bool>>,
    event_sender: Option<mpsc::UnboundedSender<MonitorEvent>>,
}

impl Monitor {
    /// 创建新的监控器
    pub fn new(
        config: MonitoringSection,
        engine_manager: Arc<RwLock<EngineManager>>,
        process_manager: Arc<ProcessManager>,
    ) -> Self {
        Self {
            config,
            engine_manager,
            process_manager,
            running: Arc::new(RwLock::new(false)),
            event_sender: None,
        }
    }
    
    /// 创建带事件通道的监控器
    pub fn with_event_channel(
        config: MonitoringSection,
        engine_manager: Arc<RwLock<EngineManager>>,
        process_manager: Arc<ProcessManager>,
    ) -> (Self, mpsc::UnboundedReceiver<MonitorEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        
        let monitor = Self {
            config,
            engine_manager,
            process_manager,
            running: Arc::new(RwLock::new(false)),
            event_sender: Some(tx),
        };
        
        (monitor, rx)
    }
    
    /// 启动监控器
    pub async fn start(&self) -> anyhow::Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Ok(());
        }
        *running = true;
        drop(running);
        
        info!(
            "Monitor started (check interval: {}s, auto-restart: {})",
            self.config.health_check_interval, self.config.auto_restart
        );
        
        let running = self.running.clone();
        let engine_manager = self.engine_manager.clone();
        let process_manager = self.process_manager.clone();
        let config_interval = Duration::from_secs(self.config.health_check_interval);
        let auto_restart = self.config.auto_restart;
        let event_sender = self.event_sender.clone();
        
        tokio::spawn(async move {
            let mut ticker = interval(config_interval);
            ticker.tick().await; // 跳过第一次立即触发
            
            while *running.read().await {
                ticker.tick().await;
                
                debug!("Monitor tick - checking engines");
                
                // 执行健康检查
                Self::health_check_loop(
                    &engine_manager,
                    &process_manager,
                    auto_restart,
                    &event_sender,
                ).await;
                
                // 收集指标
                Self::collect_metrics_loop(&engine_manager, &event_sender).await;
            }
            
            info!("Monitor stopped");
        });
        
        Ok(())
    }
    
    /// 停止监控器
    pub async fn stop(&self) -> anyhow::Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        drop(running);
        
        info!("Monitor stopped");
        Ok(())
    }
    
    /// 检查是否正在运行
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
    
    /// 健康检查循环
    async fn health_check_loop(
        engine_manager: &Arc<RwLock<EngineManager>>,
        process_manager: &Arc<ProcessManager>,
        auto_restart: bool,
        event_sender: &Option<mpsc::UnboundedSender<MonitorEvent>>,
    ) {
        let engines = {
            let manager = engine_manager.read().await;
            manager.list().clone()
        };
        
        for engine_info in engines {
            let engine_id = engine_info.engine_id.clone();
            
            // 跳过已停止的引擎
            if matches!(
                engine_info.status,
                EngineStatus::Stopped | EngineStatus::Stopping
            ) {
                continue;
            }
            
            // 检查进程是否存活
            let is_process_running = process_manager.is_running(&engine_id).await;
            
            if !is_process_running {
                let reason = "Process not running".to_string();
                warn!("Engine {} is unhealthy: {}", engine_id, reason);
                
                // 发送不健康事件
                if let Some(tx) = event_sender {
                    let _ = tx.send(MonitorEvent::EngineUnhealthy {
                        engine_id: engine_id.clone(),
                        reason: reason.clone(),
                    });
                }
                
                // 尝试自动重启
                if auto_restart {
                    info!("Attempting to restart engine {}", engine_id);
                    
                    match process_manager.restart_engine(&engine_info).await {
                        Ok(_) => {
                            // 更新状态
                            let mut manager = engine_manager.write().await;
                            if let Some(info) = manager.get_mut(&engine_id) {
                                info.status = EngineStatus::Running;
                                info.uptime = Duration::ZERO;
                                info.last_heartbeat = Some(Instant::now());
                            }
                            
                            info!("Engine {} restarted successfully", engine_id);
                            
                            // 发送重启事件
                            if let Some(tx) = event_sender {
                                let _ = tx.send(MonitorEvent::EngineRestarted {
                                    engine_id: engine_id.clone(),
                                });
                            }
                        }
                        Err(e) => {
                            error!("Failed to restart engine {}: {}", engine_id, e);
                            
                            // 标记为错误状态
                            let mut manager = engine_manager.write().await;
                            if let Some(info) = manager.get_mut(&engine_id) {
                                info.status = EngineStatus::Error;
                            }
                        }
                    }
                }
            } else {
                // 进程正常运行
                debug!("Engine {} is healthy", engine_id);
                
                // 更新运行时间
                let mut manager = engine_manager.write().await;
                if let Some(info) = manager.get_mut(&engine_id) {
                    if info.status == EngineStatus::Starting {
                        info.status = EngineStatus::Running;
                    }
                    info.uptime = info.start_time.elapsed();
                    info.last_heartbeat = Some(Instant::now());
                }
                
                // 发送健康检查通过事件
                if let Some(tx) = event_sender {
                    let _ = tx.send(MonitorEvent::HealthCheckPassed {
                        engine_id: engine_id.clone(),
                    });
                }
            }
        }
        
        // 清理死掉的进程
        let dead_engines = process_manager.cleanup_dead_processes().await;
        for engine_id in dead_engines {
            warn!("Cleaned up dead engine: {}", engine_id);
        }
    }
    
    /// 指标收集循环
    fn collect_metrics_loop(
        engine_manager: &Arc<RwLock<EngineManager>>,
        event_sender: &Option<mpsc::UnboundedSender<MonitorEvent>>,
    ) {
        // TODO: 实现更详细的指标收集
        // 当前只收集基本的计数指标
        
        let manager = engine_manager.blocking_read();
        let total_engines = manager.count();
        
        if let Some(tx) = event_sender {
            let metric = Metric {
                engine_id: "kernel".to_string(),
                timestamp: chrono::Utc::now().timestamp(),
                metric_type: "engine_count".to_string(),
                value: total_engines as f64,
            };
            
            let _ = tx.send(MonitorEvent::MetricCollected { metric });
        }
    }
    
    /// 手动触发健康检查
    pub async fn check_engine_health(&self, engine_id: &str) -> anyhow::Result<HealthCheckResult> {
        let manager = self.engine_manager.read().await;
        
        if let Some(engine_info) = manager.get(engine_id) {
            let is_running = self.process_manager.is_running(engine_id).await;
            
            if is_running {
                Ok(HealthCheckResult {
                    engine_id: engine_id.to_string(),
                    healthy: true,
                    message: "Engine is healthy".to_string(),
                    latency_ms: 0,
                })
            } else {
                Ok(HealthCheckResult {
                    engine_id: engine_id.to_string(),
                    healthy: false,
                    message: "Process not running".to_string(),
                    latency_ms: 0,
                })
            }
        } else {
            anyhow::bail!("Engine {} not found", engine_id);
        }
    }
    
    /// 获取所有引擎的健康状态
    pub async fn get_all_health_status(&self) -> Vec<HealthCheckResult> {
        let manager = self.engine_manager.read().await;
        let engines = manager.list().clone();
        drop(manager);
        
        let mut results = Vec::new();
        
        for engine_info in engines {
            let engine_id = engine_info.engine_id.clone();
            
            match self.check_engine_health(&engine_id).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    results.push(HealthCheckResult {
                        engine_id,
                        healthy: false,
                        message: e.to_string(),
                        latency_ms: 0,
                    });
                }
            }
        }
        
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_monitor_creation() {
        let config = MonitoringSection::default();
        let engine_manager = Arc::new(RwLock::new(EngineManager::new(config.clone())));
        let process_manager = Arc::new(ProcessManager::new(Arc::new(config.clone().into())));
        
        let monitor = Monitor::new(config, engine_manager, process_manager);
        
        assert!(!monitor.is_running().await);
    }
}
