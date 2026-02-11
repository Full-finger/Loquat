//! Payload 类型定义 - 具体类型实现
//!
//! 定义了具体的 Payload 类型，支持与 proto::Any 互操作

use prost_types as prost_types;
use prost_types::Any;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::fmt;

// ========================================================================
// 错误类型
// ========================================================================

/// Payload 操作相关错误
#[derive(Error, Debug)]
pub enum PayloadError {
    #[error("Payload type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },
    
    #[error("Failed to serialize payload: {0}")]
    SerializationError(String),
    
    #[error("Failed to deserialize payload: {0}")]
    DeserializationError(String),
    
    #[error("Unknown payload type: {0}")]
    UnknownType(String),
    
    #[error("Invalid type URL: {0}")]
    InvalidTypeUrl(String),
}

/// 类型 URL 映射
/// 格式: `type.googleapis.com/loquat.payloads.{Type}`
pub trait TypeUrl {
    /// 获取类型 URL
    fn type_url() -> &'static str;
}

// ========================================================================
// Payload Trait
// ========================================================================

/// Payload 类型标记
pub trait PayloadType: Send + Sync + 'static {
    /// 获取类型名称
    fn type_name(&self) -> &'static str;
    
    /// 获取大小估计（字节）
    fn size_estimate(&self) -> usize;
}

/// 通用 Payload trait - 支持与 proto::Any 互操作
pub trait UniversalPayload: PayloadType + fmt::Debug + Send + Sync + 'static {
    /// 序列化为 proto::Any
    fn to_any(&self) -> Result<Any, PayloadError>;
    
    /// 从 proto::Any 反序列化
    fn from_any(any: &Any) -> Result<Self, PayloadError>
    where
        Self: Sized;
    
    /// 返回 `self` 作为 `Any` 以允许 downcast
    fn as_any(&self) -> &dyn std::any::Any;
}

// ========================================================================
// Text Payload
// ========================================================================

/// 文本 Payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPayload {
    /// 文本内容
    pub content: String,
    
    /// 文本格式
    #[serde(default)]
    pub format: TextFormat,
}

impl TextPayload {
    /// 创建新的文本 Payload
    pub fn new<S: Into<String>>(content: S) -> Self {
        Self {
            content: content.into(),
            format: TextFormat::Plain,
        }
    }
    
    /// 设置文本格式
    pub fn with_format(mut self, format: TextFormat) -> Self {
        self.format = format;
        self
    }
}

impl TypeUrl for TextPayload {
    fn type_url() -> &'static str {
        "type.googleapis.com/loquat.payloads.TextPayload"
    }
}

impl PayloadType for TextPayload {
    fn type_name(&self) -> &'static str {
        "TextPayload"
    }
    
    fn size_estimate(&self) -> usize {
        self.content.len()
    }
}

impl UniversalPayload for TextPayload {
    fn to_any(&self) -> Result<Any, PayloadError> {
        // 序列化为 JSON 字节
        let json_bytes = serde_json::to_vec(self)
            .map_err(|e| PayloadError::SerializationError(e.to_string()))?;
        
        Ok(Any {
            type_url: Self::type_url().to_string(),
            value: json_bytes,
        })
    }
    
    fn from_any(any: &Any) -> Result<Self, PayloadError> {
        // 验证类型 URL
        if any.type_url != Self::type_url() {
            return Err(PayloadError::TypeMismatch {
                expected: Self::type_url().to_string(),
                actual: any.type_url.clone(),
            });
        }
        
        // 从 JSON 反序列化
        serde_json::from_slice(&any.value)
            .map_err(|e| PayloadError::DeserializationError(e.to_string()))
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ========================================================================
// Blob Payload
// ========================================================================

/// 二进制 Payload（用于图片、文件等）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobPayload {
    /// 二进制数据
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
    
    /// MIME 类型
    #[serde(default)]
    pub mime_type: String,
    
    /// 可选的 URL（用于大文件）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl BlobPayload {
    /// 创建新的二进制 Payload
    pub fn new(data: Vec<u8>, mime_type: String) -> Self {
        Self {
            data,
            mime_type,
            url: None,
        }
    }
    
    /// 从 URL 创建（用于大文件）
    pub fn from_url(url: String, mime_type: String) -> Self {
        Self {
            data: Vec::new(),
            mime_type,
            url: Some(url),
        }
    }
    
    /// 获取数据大小
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

impl TypeUrl for BlobPayload {
    fn type_url() -> &'static str {
        "type.googleapis.com/loquat.payloads.BlobPayload"
    }
}

impl PayloadType for BlobPayload {
    fn type_name(&self) -> &'static str {
        "BlobPayload"
    }
    
    fn size_estimate(&self) -> usize {
        self.data.len()
    }
}

impl UniversalPayload for BlobPayload {
    fn to_any(&self) -> Result<Any, PayloadError> {
        let json_bytes = serde_json::to_vec(self)
            .map_err(|e| PayloadError::SerializationError(e.to_string()))?;
        
        Ok(Any {
            type_url: Self::type_url().to_string(),
            value: json_bytes,
        })
    }
    
    fn from_any(any: &Any) -> Result<Self, PayloadError> {
        if any.type_url != Self::type_url() {
            return Err(PayloadError::TypeMismatch {
                expected: Self::type_url().to_string(),
                actual: any.type_url.clone(),
            });
        }
        
        serde_json::from_slice(&any.value)
            .map_err(|e| PayloadError::DeserializationError(e.to_string()))
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ========================================================================
// Event Payload
// ========================================================================

/// 事件 Payload（结构化事件数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPayload {
    /// 事件类型
    pub event_type: String,
    
    /// 事件数据（灵活的 JSON 结构）
    pub data: serde_json::Value,
}

impl EventPayload {
    /// 创建新的事件 Payload
    pub fn new<S: Into<String>>(event_type: S, data: serde_json::Value) -> Self {
        Self {
            event_type: event_type.into(),
            data,
        }
    }
    
    /// 从事件数据获取类型化数据
    pub fn get_data<T: serde::de::DeserializeOwned>(&self) -> Result<T, PayloadError> {
        serde_json::from_value(self.data.clone())
            .map_err(|e| PayloadError::DeserializationError(e.to_string()))
    }
    
    /// 设置事件数据
    pub fn with_data<T: serde::Serialize>(mut self, data: T) -> Result<Self, PayloadError> {
        self.data = serde_json::to_value(data)
            .map_err(|e| PayloadError::SerializationError(e.to_string()))?;
        Ok(self)
    }
}

impl TypeUrl for EventPayload {
    fn type_url() -> &'static str {
        "type.googleapis.com/loquat.payloads.EventPayload"
    }
}

impl PayloadType for EventPayload {
    fn type_name(&self) -> &'static str {
        "EventPayload"
    }
    
    fn size_estimate(&self) -> usize {
        self.data.to_string().len()
    }
}

impl UniversalPayload for EventPayload {
    fn to_any(&self) -> Result<Any, PayloadError> {
        let json_bytes = serde_json::to_vec(self)
            .map_err(|e| PayloadError::SerializationError(e.to_string()))?;
        
        Ok(Any {
            type_url: Self::type_url().to_string(),
            value: json_bytes,
        })
    }
    
    fn from_any(any: &Any) -> Result<Self, PayloadError> {
        if any.type_url != Self::type_url() {
            return Err(PayloadError::TypeMismatch {
                expected: Self::type_url().to_string(),
                actual: any.type_url.clone(),
            });
        }
        
        serde_json::from_slice(&any.value)
            .map_err(|e| PayloadError::DeserializationError(e.to_string()))
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ========================================================================
// Text Format
// ========================================================================

/// 文本格式枚举
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TextFormat {
    Plain,
    Markdown,
    Html,
    Json,
}

impl Default for TextFormat {
    fn default() -> Self {
        TextFormat::Plain
    }
}

impl fmt::Display for TextFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextFormat::Plain => write!(f, "plain"),
            TextFormat::Markdown => write!(f, "markdown"),
            TextFormat::Html => write!(f, "html"),
            TextFormat::Json => write!(f, "json"),
        }
    }
}

// ========================================================================
// 测试
// ========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_payload() {
        let payload = TextPayload::new("Hello, world!");
        assert_eq!(payload.type_name(), "TextPayload");
        assert_eq!(payload.content, "Hello, world!");
        
        // 测试序列化
        let any = payload.to_any().unwrap();
        assert_eq!(any.type_url, TextPayload::type_url());
        
        // 测试反序列化
        let restored = TextPayload::from_any(&any).unwrap();
        assert_eq!(restored.content, payload.content);
    }

    #[test]
    fn test_text_payload_format() {
        let payload = TextPayload::new("**Bold**")
            .with_format(TextFormat::Markdown);
        assert_eq!(payload.format, TextFormat::Markdown);
    }

    #[test]
    fn test_blob_payload() {
        let data = vec![1u8, 2u8, 3u8, 4u8];
        let payload = BlobPayload::new(data.clone(), "application/octet-stream".to_string());
        assert_eq!(payload.type_name(), "BlobPayload");
        assert_eq!(payload.data, data);
        
        // 测试序列化
        let any = payload.to_any().unwrap();
        assert_eq!(any.type_url, BlobPayload::type_url());
        
        // 测试反序列化
        let restored = BlobPayload::from_any(&any).unwrap();
        assert_eq!(restored.data, data);
    }

    #[test]
    fn test_blob_payload_url() {
        let payload = BlobPayload::from_url(
            "https://example.com/file.jpg".to_string(),
            "image/jpeg".to_string()
        );
        assert_eq!(payload.url, Some("https://example.com/file.jpg".to_string()));
        assert_eq!(payload.data, Vec::<u8>::new());
    }

    #[test]
    fn test_event_payload() {
        let data = serde_json::json!({
            "user_id": "user123",
            "message": "test"
        });
        let payload = EventPayload::new("user.login", data.clone());
        assert_eq!(payload.type_name(), "EventPayload");
        assert_eq!(payload.event_type, "user.login");
        
        // 测试类型化数据获取
        #[derive(Deserialize, Debug, PartialEq)]
        struct UserData {
            user_id: String,
            message: String,
        }
        
        let user_data: UserData = payload.get_data().unwrap();
        assert_eq!(user_data.message, "test");
        
        // 测试序列化
        let any = payload.to_any().unwrap();
        assert_eq!(any.type_url, EventPayload::type_url());
        
        // 测试反序列化
        let restored = EventPayload::from_any(&any).unwrap();
        assert_eq!(restored.event_type, "user.login");
    }

    #[test]
    fn test_type_mismatch_error() {
        let text_payload = TextPayload::new("test");
        let any = text_payload.to_any().unwrap();
        
        // 尝试用错误的类型反序列化
        let result = BlobPayload::from_any(&any);
        assert!(result.is_err());
        match result.unwrap_err() {
            PayloadError::TypeMismatch { expected, actual } => {
                assert_eq!(expected, BlobPayload::type_url());
                assert_eq!(actual, TextPayload::type_url());
            }
            _ => panic!("Expected TypeMismatch error"),
        }
    }

    #[test]
    fn test_text_format_display() {
        assert_eq!(format!("{}", TextFormat::Plain), "plain");
        assert_eq!(format!("{}", TextFormat::Markdown), "markdown");
        assert_eq!(format!("{}", TextFormat::Html), "html");
        assert_eq!(format!("{}", TextFormat::Json), "json");
    }
}
