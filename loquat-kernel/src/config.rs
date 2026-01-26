//! Kernel配置管理

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelConfig {
    pub kernel: KernelSection,
    pub engine: EngineSection,
    pub monitoring: MonitoringSection,
    pub logging: LoggingSection,
    pub resource: ResourceSection,
    pub security: SecuritySection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelSection {
    pub bind_address: String,
    pub web_address: String,
    pub max_engines: usize,
    pub kernel_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineSection {
    pub default_host: String,
    pub default_port_range: [u16; 2],
    pub auto_restart: bool,
    pub restart_delay_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringSection {
    pub health_check_interval: u64,
    pub metric_collection_interval: u64,
    pub enable_auto_restart: bool,
    pub restart_failure_threshold: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSection {
    pub level: String,
    pub output: String,
    pub file_path: String,
    pub format: String,
    pub max_file_size: String,
    pub max_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSection {
    pub max_memory_per_engine: String,
    pub max_cpu_percent: u8,
    pub max_engines_per_user: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySection {
    pub enable_sandbox: bool,
    pub allow_network: bool,
    pub allowed_hosts: Vec<String>,
    pub require_authentication: bool,
}

impl KernelConfig {
    /// 从文件加载配置
    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: KernelConfig = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// 从默认路径加载配置
    pub fn from_default() -> Result<Self> {
        Self::from_file("config/kernel.toml")
    }
    
    /// 获取日志文件路径
    pub fn log_file_path(&self) -> PathBuf {
        PathBuf::from(&self.logging.file_path)
    }
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            kernel: KernelSection {
                bind_address: "127.0.0.1:50051".to_string(),
                web_address: "127.0.0.1:8080".to_string(),
                max_engines: 10,
                kernel_id: "kernel-001".to_string(),
            },
            engine: EngineSection {
                default_host: "127.0.0.1".to_string(),
                default_port_range: [50052, 50151],
                auto_restart: true,
                restart_delay_seconds: 5,
            },
            monitoring: MonitoringSection {
                health_check_interval: 30,
                metric_collection_interval: 10,
                enable_auto_restart: true,
                restart_failure_threshold: 3,
            },
            logging: LoggingSection {
                level: "info".to_string(),
                output: "file".to_string(),
                file_path: "logs/kernel.log".to_string(),
                format: "json".to_string(),
                max_file_size: "100MB".to_string(),
                max_files: 10,
            },
            resource: ResourceSection {
                max_memory_per_engine: "1GB".to_string(),
                max_cpu_percent: 80,
                max_engines_per_user: 5,
            },
            security: SecuritySection {
                enable_sandbox: true,
                allow_network: true,
                allowed_hosts: vec!["127.0.0.1".to_string(), "localhost".to_string()],
                require_authentication: false,
            },
        }
    }
}
