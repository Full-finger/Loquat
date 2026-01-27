use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Engine配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub engine: EngineSection,
    pub plugins: PluginsSection,
    pub adapters: AdaptersSection,
    pub api: ApiSection,
    pub logging: LoggingSection,
    pub resources: ResourcesSection,
    pub performance: PerformanceSection,
    pub security: SecuritySection,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            engine: EngineSection::default(),
            plugins: PluginsSection::default(),
            adapters: AdaptersSection::default(),
            api: ApiSection::default(),
            logging: LoggingSection::default(),
            resources: ResourcesSection::default(),
            performance: PerformanceSection::default(),
            security: SecuritySection::default(),
        }
    }
}

impl EngineConfig {
    /// 从文件加载配置
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// 保存配置到文件
    pub fn save_to_file(&self, path: &str) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

/// Engine基本配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineSection {
    pub engine_id: String,
    pub port: u16,
    pub kernel_address: String,
}

impl Default for EngineSection {
    fn default() -> Self {
        Self {
            engine_id: uuid::Uuid::new_v4().to_string(),
            port: 50052,
            kernel_address: "127.0.0.1:50051".to_string(),
        }
    }
}

/// 插件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginsSection {
    pub plugin_dir: PathBuf,
    pub auto_load: bool,
    pub hot_reload: bool,
    pub sandbox_mode: bool,
}

impl Default for PluginsSection {
    fn default() -> Self {
        Self {
            plugin_dir: PathBuf::from("./plugins"),
            auto_load: true,
            hot_reload: false,
            sandbox_mode: true,
        }
    }
}

/// 适配器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptersSection {
    pub adapter_dir: PathBuf,
    pub default_adapter: String,
}

impl Default for AdaptersSection {
    fn default() -> Self {
        Self {
            adapter_dir: PathBuf::from("./adapters"),
            default_adapter: "mock_test".to_string(),
        }
    }
}

/// API配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSection {
    pub http_address: String,
    pub grpc_address: String,
}

impl Default for ApiSection {
    fn default() -> Self {
        Self {
            http_address: "127.0.0.1:8081".to_string(),
            grpc_address: "127.0.0.1:50052".to_string(),
        }
    }
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSection {
    pub level: String,
    pub format: String,
    pub file: PathBuf,
}

impl Default for LoggingSection {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "pretty".to_string(),
            file: PathBuf::from("./logs/engine.log"),
        }
    }
}

/// 资源配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesSection {
    pub max_memory_mb: usize,
    pub max_threads: usize,
    pub connection_pool_size: usize,
}

impl Default for ResourcesSection {
    fn default() -> Self {
        Self {
            max_memory_mb: 512,
            max_threads: 4,
            connection_pool_size: 10,
        }
    }
}

/// 性能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSection {
    pub cache_size: usize,
    pub request_timeout: u64,
}

impl Default for PerformanceSection {
    fn default() -> Self {
        Self {
            cache_size: 100,
            request_timeout: 30,
        }
    }
}

/// 安全配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySection {
    pub enable_auth: bool,
    pub api_key: String,
}

impl Default for SecuritySection {
    fn default() -> Self {
        Self {
            enable_auth: false,
            api_key: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = EngineConfig::default();
        assert!(!config.engine.engine_id.is_empty());
        assert_eq!(config.engine.port, 50052);
    }
}
