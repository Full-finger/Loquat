use crate::events::traits::{Event, EventMetadata, EventSource};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// 元事件类型 - 系统级事件
/// 参考 onebot/napcat 风格
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum MetaEvent {
    /// 心跳事件
    Heartbeat {
        /// 心跳间隔（毫秒）
        interval: u32,
        /// 元数据
        #[serde(flatten)]
        metadata: EventMetadata,
    },

    /// 生命周期事件
    Lifecycle {
        /// 生命周期阶段
        phase: LifecyclePhase,
        /// 元数据
        #[serde(flatten)]
        metadata: EventMetadata,
    },

    /// 连接状态变更事件
    ConnectionChange {
        /// 连接状态
        status: ConnectionStatus,
        /// 连接类型（如 "ws", "http"）
        conn_type: Option<String>,
        /// 重连次数
        reconnect_count: Option<u32>,
        /// 错误信息（如果连接失败）
        error: Option<String>,
        /// 元数据
        #[serde(flatten)]
        metadata: EventMetadata,
    },

    /// 系统事件
    System {
        /// 系统事件类型
        event_type: SystemEventType,
        /// 事件描述
        description: String,
        /// 额外数据
        #[serde(default)]
        data: HashMap<String, serde_json::Value>,
        /// 元数据
        #[serde(flatten)]
        metadata: EventMetadata,
    },

    /// 性能指标事件
    Performance {
        /// CPU 使用率（百分比）
        cpu_usage: Option<f64>,
        /// 内存使用（字节）
        memory_usage: Option<u64>,
        /// 内存使用率（百分比）
        memory_usage_percent: Option<f64>,
        /// 消息队列大小
        queue_size: Option<u64>,
        /// 额外指标
        #[serde(default)]
        metrics: HashMap<String, serde_json::Value>,
        /// 元数据
        #[serde(flatten)]
        metadata: EventMetadata,
    },

    /// 插件事件
    Plugin {
        /// 插件事件类型
        plugin_event: PluginEventType,
        /// 插件名称
        plugin_name: String,
        /// 插件版本
        plugin_version: Option<String>,
        /// 事件描述
        description: Option<String>,
        /// 额外数据
        #[serde(default)]
        data: HashMap<String, serde_json::Value>,
        /// 元数据
        #[serde(flatten)]
        metadata: EventMetadata,
    },
}

/// 生命周期阶段
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LifecyclePhase {
    /// 启动中
    Starting,
    /// 已启动
    Started,
    /// 停止中
    Stopping,
    /// 已停止
    Stopped,
    /// 暂停中
    Pausing,
    /// 已暂停
    Paused,
    /// 恢复中
    Resuming,
    /// 已恢复
    Resumed,
    /// 重启中
    Restarting,
}

/// 连接状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    /// 已连接
    Connected,
    /// 断开连接
    Disconnected,
    /// 连接中
    Connecting,
    /// 重连中
    Reconnecting,
    /// 连接失败
    Failed,
}

/// 系统事件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SystemEventType {
    /// 配置更新
    ConfigUpdated,
    /// 数据库连接
    DatabaseConnected,
    /// 数据库断开
    DatabaseDisconnected,
    /// 缓存连接
    CacheConnected,
    /// 缓存断开
    CacheDisconnected,
    /// 存储空间警告
    StorageWarning,
    /// 存储空间严重警告
    StorageCritical,
    /// 其他系统事件
    Other(String),
}

/// 插件事件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PluginEventType {
    /// 插件加载
    Load,
    /// 插件卸载
    Unload,
    /// 插件启用
    Enable,
    /// 插件禁用
    Disable,
    /// 插件重载
    Reload,
    /// 插件更新
    Update,
    /// 插件错误
    Error,
}

impl Event for MetaEvent {
    fn event_id(&self) -> &str {
        match self {
            MetaEvent::Heartbeat { metadata, .. } => &metadata.event_id,
            MetaEvent::Lifecycle { metadata, .. } => &metadata.event_id,
            MetaEvent::ConnectionChange { metadata, .. } => &metadata.event_id,
            MetaEvent::System { metadata, .. } => &metadata.event_id,
            MetaEvent::Performance { metadata, .. } => &metadata.event_id,
            MetaEvent::Plugin { metadata, .. } => &metadata.event_id,
        }
    }
    
    fn event_type(&self) -> &str {
        match self {
            MetaEvent::Heartbeat { .. } => "meta.heartbeat",
            MetaEvent::Lifecycle { phase, .. } => match phase {
                LifecyclePhase::Starting => "meta.lifecycle.starting",
                LifecyclePhase::Started => "meta.lifecycle.started",
                LifecyclePhase::Stopping => "meta.lifecycle.stopping",
                LifecyclePhase::Stopped => "meta.lifecycle.stopped",
                LifecyclePhase::Pausing => "meta.lifecycle.pausing",
                LifecyclePhase::Paused => "meta.lifecycle.paused",
                LifecyclePhase::Resuming => "meta.lifecycle.resuming",
                LifecyclePhase::Resumed => "meta.lifecycle.resumed",
                LifecyclePhase::Restarting => "meta.lifecycle.restarting",
            },
            MetaEvent::ConnectionChange { status, .. } => match status {
                ConnectionStatus::Connected => "meta.connection.connected",
                ConnectionStatus::Disconnected => "meta.connection.disconnected",
                ConnectionStatus::Connecting => "meta.connection.connecting",
                ConnectionStatus::Reconnecting => "meta.connection.reconnecting",
                ConnectionStatus::Failed => "meta.connection.failed",
            },
            MetaEvent::System { event_type, .. } => match event_type {
                SystemEventType::ConfigUpdated => "meta.system.config_updated",
                SystemEventType::DatabaseConnected => "meta.system.database_connected",
                SystemEventType::DatabaseDisconnected => "meta.system.database_disconnected",
                SystemEventType::CacheConnected => "meta.system.cache_connected",
                SystemEventType::CacheDisconnected => "meta.system.cache_disconnected",
                SystemEventType::StorageWarning => "meta.system.storage_warning",
                SystemEventType::StorageCritical => "meta.system.storage_critical",
                SystemEventType::Other(_) => "meta.system.other",
            },
            MetaEvent::Performance { .. } => "meta.performance",
            MetaEvent::Plugin { plugin_event, .. } => match plugin_event {
                PluginEventType::Load => "meta.plugin.load",
                PluginEventType::Unload => "meta.plugin.unload",
                PluginEventType::Enable => "meta.plugin.enable",
                PluginEventType::Disable => "meta.plugin.disable",
                PluginEventType::Reload => "meta.plugin.reload",
                PluginEventType::Update => "meta.plugin.update",
                PluginEventType::Error => "meta.plugin.error",
            },
        }
    }
    
    fn timestamp(&self) -> DateTime<Utc> {
        match self {
            MetaEvent::Heartbeat { metadata, .. } => metadata.timestamp,
            MetaEvent::Lifecycle { metadata, .. } => metadata.timestamp,
            MetaEvent::ConnectionChange { metadata, .. } => metadata.timestamp,
            MetaEvent::System { metadata, .. } => metadata.timestamp,
            MetaEvent::Performance { metadata, .. } => metadata.timestamp,
            MetaEvent::Plugin { metadata, .. } => metadata.timestamp,
        }
    }
    
    fn source(&self) -> EventSource {
        match self {
            MetaEvent::Heartbeat { metadata, .. } => metadata.source.clone(),
            MetaEvent::Lifecycle { metadata, .. } => metadata.source.clone(),
            MetaEvent::ConnectionChange { metadata, .. } => metadata.source.clone(),
            MetaEvent::System { metadata, .. } => metadata.source.clone(),
            MetaEvent::Performance { metadata, .. } => metadata.source.clone(),
            MetaEvent::Plugin { metadata, .. } => metadata.source.clone(),
        }
    }
    
    fn user_id(&self) -> Option<&str> {
        match self {
            MetaEvent::Heartbeat { metadata, .. } => metadata.user_id.as_deref(),
            MetaEvent::Lifecycle { metadata, .. } => metadata.user_id.as_deref(),
            MetaEvent::ConnectionChange { metadata, .. } => metadata.user_id.as_deref(),
            MetaEvent::System { metadata, .. } => metadata.user_id.as_deref(),
            MetaEvent::Performance { metadata, .. } => metadata.user_id.as_deref(),
            MetaEvent::Plugin { metadata, .. } => metadata.user_id.as_deref(),
        }
    }
    
    fn group_id(&self) -> Option<&str> {
        match self {
            MetaEvent::Heartbeat { metadata, .. } => metadata.group_id.as_deref(),
            MetaEvent::Lifecycle { metadata, .. } => metadata.group_id.as_deref(),
            MetaEvent::ConnectionChange { metadata, .. } => metadata.group_id.as_deref(),
            MetaEvent::System { metadata, .. } => metadata.group_id.as_deref(),
            MetaEvent::Performance { metadata, .. } => metadata.group_id.as_deref(),
            MetaEvent::Plugin { metadata, .. } => metadata.group_id.as_deref(),
        }
    }
    
    fn self_id(&self) -> Option<&str> {
        match self {
            MetaEvent::Heartbeat { metadata, .. } => metadata.self_id.as_deref(),
            MetaEvent::Lifecycle { metadata, .. } => metadata.self_id.as_deref(),
            MetaEvent::ConnectionChange { metadata, .. } => metadata.self_id.as_deref(),
            MetaEvent::System { metadata, .. } => metadata.self_id.as_deref(),
            MetaEvent::Performance { metadata, .. } => metadata.self_id.as_deref(),
            MetaEvent::Plugin { metadata, .. } => metadata.self_id.as_deref(),
        }
    }
    
    fn correlation_id(&self) -> Option<&str> {
        match self {
            MetaEvent::Heartbeat { metadata, .. } => metadata.correlation_id.as_deref(),
            MetaEvent::Lifecycle { metadata, .. } => metadata.correlation_id.as_deref(),
            MetaEvent::ConnectionChange { metadata, .. } => metadata.correlation_id.as_deref(),
            MetaEvent::System { metadata, .. } => metadata.correlation_id.as_deref(),
            MetaEvent::Performance { metadata, .. } => metadata.correlation_id.as_deref(),
            MetaEvent::Plugin { metadata, .. } => metadata.correlation_id.as_deref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_event_serialization() {
        let heartbeat = MetaEvent::Heartbeat {
            interval: 5000,
            metadata: EventMetadata::default(),
        };

        let json = serde_json::to_string(&heartbeat).unwrap();
        let deserialized: MetaEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(heartbeat, deserialized);
    }

    #[test]
    fn test_lifecycle_event_serialization() {
        let lifecycle = MetaEvent::Lifecycle {
            phase: LifecyclePhase::Started,
            metadata: EventMetadata::default(),
        };

        let json = serde_json::to_string(&lifecycle).unwrap();
        let deserialized: MetaEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(lifecycle, deserialized);
    }

    #[test]
    fn test_connection_change_event_serialization() {
        let conn_change = MetaEvent::ConnectionChange {
            status: ConnectionStatus::Connected,
            conn_type: Some("ws".to_string()),
            reconnect_count: Some(0),
            error: None,
            metadata: EventMetadata::default(),
        };

        let json = serde_json::to_string(&conn_change).unwrap();
        let deserialized: MetaEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(conn_change, deserialized);
    }

    #[test]
    fn test_performance_event_serialization() {
        let mut metrics = HashMap::new();
        metrics.insert("disk_usage".to_string(), serde_json::json!(1024));

        let performance = MetaEvent::Performance {
            cpu_usage: Some(25.5),
            memory_usage: Some(1024 * 1024 * 1024),
            memory_usage_percent: Some(50.0),
            queue_size: Some(100),
            metrics,
            metadata: EventMetadata::default(),
        };

        let json = serde_json::to_string(&performance).unwrap();
        let deserialized: MetaEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(performance, deserialized);
    }

    #[test]
    fn test_plugin_event_serialization() {
        let mut data = HashMap::new();
        data.insert("load_time".to_string(), serde_json::json!(1000));

        let plugin_event = MetaEvent::Plugin {
            plugin_event: PluginEventType::Load,
            plugin_name: "test_plugin".to_string(),
            plugin_version: Some("1.0.0".to_string()),
            description: Some("Test plugin loaded".to_string()),
            data,
            metadata: EventMetadata::default(),
        };

        let json = serde_json::to_string(&plugin_event).unwrap();
        let deserialized: MetaEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(plugin_event, deserialized);
    }
}
