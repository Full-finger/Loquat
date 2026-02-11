//! Payload 适配器 - 与 proto::Any 互操作
//!
//! 提供便捷函数和 trait 用于与 proto::Any 互操作

use prost_types as prost_types;
use prost_types::Any;
use crate::payloads::{
    UniversalPayload, PayloadError, TextPayload, BlobPayload, EventPayload,
    TypeUrl,
};

// ========================================================================
// Payload Adapter Trait
// ========================================================================

/// Payload 适配器 trait
/// 为需要与 proto::Any 互操作的类型提供统一接口
pub trait PayloadAdapter: Send + Sync + 'static {
    /// 序列化为 proto::Any
    fn to_any(&self) -> Result<Any, PayloadError>;
    
    /// 从 proto::Any 反序列化
    fn from_any(any: &Any) -> Result<Self, PayloadError>
    where
        Self: Sized;
    
    /// 获取类型 URL
    fn type_url() -> &'static str;
}

// ========================================================================
// 为所有 UniversalPayload 实现 PayloadAdapter
// ========================================================================

impl<T: UniversalPayload + TypeUrl> PayloadAdapter for T {
    fn to_any(&self) -> Result<Any, PayloadError> {
        <T as UniversalPayload>::to_any(self)
    }
    
    fn from_any(any: &Any) -> Result<Self, PayloadError>
    where
        Self: Sized,
    {
        <T as UniversalPayload>::from_any(any)
    }
    
    fn type_url() -> &'static str {
        <T as TypeUrl>::type_url()
    }
}

// ========================================================================
// 便捷函数
// ========================================================================

/// 将 Payload 转换为 proto::Any
/// 
/// # 示例
/// ```rust
/// use loquat::payloads::{TextPayload, to_any};
/// 
/// let text = TextPayload::new("Hello");
/// let any = to_any(&text).unwrap();
/// ```
pub fn to_any<T: PayloadAdapter>(payload: &T) -> Result<Any, PayloadError> {
    payload.to_any()
}

/// 从 proto::Any 转换为 Payload
///
/// # 示例
/// ```rust
/// use loquat::payloads::{TextPayload, from_any};
/// use prost_types::Any;
///
/// let any = Any { /* ... */ };
/// let text: TextPayload = from_any(&any).unwrap();
/// ```
pub fn from_any<T: PayloadAdapter>(any: &Any) -> Result<T, PayloadError> {
    T::from_any(any)
}

/// 尝试从 proto::Any 解析任意支持的 Payload 类型
///
/// 按顺序尝试：TextPayload → BlobPayload → EventPayload
/// 返回第一个成功的解析结果
///
/// # 示例
/// ```rust
/// use loquat::payloads::try_parse_any;
/// use prost_types::Any;
///
/// let any = Any { /* TextPayload 的 Any */ };
/// if let Ok(payload) = try_parse_any(&any) {
///     // payload 可能是 TextPayload, BlobPayload 或 EventPayload
/// }
/// ```
pub fn try_parse_any(any: &Any) -> Result<Box<dyn UniversalPayload>, PayloadError> {
    // 尝试 TextPayload
    if let Ok(text) = <TextPayload as UniversalPayload>::from_any(any) {
        return Ok(Box::new(text));
    }
    
    // 尝试 BlobPayload
    if let Ok(blob) = <BlobPayload as UniversalPayload>::from_any(any) {
        return Ok(Box::new(blob));
    }
    
    // 尝试 EventPayload
    if let Ok(event) = <EventPayload as UniversalPayload>::from_any(any) {
        return Ok(Box::new(event));
    }
    
    // 都失败了，返回未知类型错误
    Err(PayloadError::UnknownType(any.type_url.clone()))
}

/// 从 proto::Any 反序列化为指定类型
///
/// # 类型参数
/// `T`：目标类型，必须实现 UniversalPayload
///
/// # 示例
/// ```rust
/// use loquat::payloads::{TextPayload, from_any_typed};
/// use prost_types::Any;
///
/// let any = Any { /* TextPayload 的 Any */ };
/// let text: TextPayload = from_any_typed(&any).unwrap();
/// ```
pub fn from_any_typed<T: UniversalPayload>(any: &Any) -> Result<T, PayloadError> {
    T::from_any(any)
}

// ========================================================================
// Package Payload 便捷函数
// ========================================================================

/// 为 proto::Package 的 payload 字段提供便捷访问
///
/// # 示例
/// ```rust
/// use loquat_proto::v1::Package;
/// use loquat::payloads::get_payload_from_package;
///
/// let package = Package { /* ... */ };
/// if let Some(any) = &package.payload {
///     if let Ok(text) = get_payload_from_package::<TextPayload>(&package) {
///         println!("Text: {}", text.content);
///     }
/// }
/// ```
pub fn get_payload_from_package<T: UniversalPayload>(
    package: &loquat_proto::v1::Package
) -> Result<T, PayloadError> {
    package.payload
        .as_ref()
        .ok_or_else(|| PayloadError::DeserializationError("Package has no payload".to_string()))
        .and_then(|any| T::from_any(any))
}

/// 设置 proto::Package 的 payload 字段
///
/// # 示例
/// ```rust
/// use loquat_proto::v1::Package;
/// use loquat::payloads::{TextPayload, set_payload_in_package};
///
/// let mut package = Package::default();
/// let text = TextPayload::new("Hello");
/// set_payload_in_package(&mut package, text).unwrap();
/// ```
pub fn set_payload_in_package<T: UniversalPayload + TypeUrl>(
    package: &mut loquat_proto::v1::Package,
    payload: T,
) -> Result<(), PayloadError> {
    let any = <T as UniversalPayload>::to_any(&payload)?;
    
    // 更新 payload 字段
    package.payload = Some(any);
    
    // 更新 meta 字段（如果有）
    if package.meta.is_none() {
        use loquat_proto::v1::PayloadMeta;
        package.meta = Some(PayloadMeta {
            type_url: <T as TypeUrl>::type_url().to_string(),
            size_bytes: payload.size_estimate() as u32,
            is_stream: false,
            hints: Default::default(),
        });
    }
    
    Ok(())
}

// ========================================================================
// 测试
// ========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_to_any_text() {
        let text = TextPayload::new("Hello, world!");
        let any = to_any(&text).unwrap();
        
        assert_eq!(any.type_url, TextPayload::type_url());
        assert!(!any.value.is_empty());
    }

    #[test]
    fn test_from_any_text() {
        let text = TextPayload::new("Test");
        let any = text.to_any().unwrap();
        let restored = from_any::<TextPayload>(&any).unwrap();
        
        assert_eq!(restored.content, "Test");
        assert_eq!(restored.format, crate::payloads::TextFormat::Plain);
    }

    #[test]
    fn test_to_any_blob() {
        let data = vec![1u8, 2u8, 3u8];
        let blob = BlobPayload::new(data.clone(), "application/octet-stream".to_string());
        let any = to_any(&blob).unwrap();
        
        assert_eq!(any.type_url, BlobPayload::type_url());
    }

    #[test]
    fn test_from_any_blob() {
        let data = vec![10u8, 20u8];
        let blob = BlobPayload::new(data.clone(), "application/octet-stream".to_string());
        let any = blob.to_any().unwrap();
        let restored = from_any::<BlobPayload>(&any).unwrap();
        
        assert_eq!(restored.data, data);
    }

    #[test]
    fn test_to_any_event() {
        let event_data = json!({"key": "value"});
        let event = EventPayload::new("test.event", event_data);
        let any = to_any(&event).unwrap();
        
        assert_eq!(any.type_url, EventPayload::type_url());
    }

    #[test]
    fn test_from_any_event() {
        let event_data = json!({"msg": "hello"});
        let event = EventPayload::new("test.event", event_data);
        let any = event.to_any().unwrap();
        let restored = from_any::<EventPayload>(&any).unwrap();
        
        assert_eq!(restored.event_type, "test.event");
    }

    #[test]
    fn test_try_parse_any_text() {
        let text = TextPayload::new("Test");
        let any = text.to_any().unwrap();
        let payload = try_parse_any(&any).unwrap();
        
        // downcast 验证类型
        if let Some(text) = payload.as_any().downcast_ref::<TextPayload>() {
            assert_eq!(text.content, "Test");
        } else {
            panic!("Failed to downcast to TextPayload");
        }
    }

    #[test]
    fn test_try_parse_any_blob() {
        let data = vec![5u8, 10u8];
        let blob = BlobPayload::new(data.clone(), "application/octet-stream".to_string());
        let any = blob.to_any().unwrap();
        let payload = try_parse_any(&any).unwrap();
        
        if let Some(blob) = payload.as_any().downcast_ref::<BlobPayload>() {
            assert_eq!(blob.data, data);
        } else {
            panic!("Failed to downcast to BlobPayload");
        }
    }

    #[test]
    fn test_try_parse_any_event() {
        let event_data = json!({"test": "data"});
        let event = EventPayload::new("test.event", event_data);
        let any = event.to_any().unwrap();
        let payload = try_parse_any(&any).unwrap();
        
        if let Some(event) = payload.as_any().downcast_ref::<EventPayload>() {
            assert_eq!(event.event_type, "test.event");
        } else {
            panic!("Failed to downcast to EventPayload");
        }
    }

    #[test]
    fn test_try_parse_any_unknown() {
        let any = Any {
            type_url: "unknown.type".to_string(),
            value: vec![1, 2, 3],
        };
        
        let result = try_parse_any(&any);
        assert!(result.is_err());
        match result.unwrap_err() {
            PayloadError::UnknownType(type_url) => {
                assert_eq!(type_url, "unknown.type");
            }
            _ => panic!("Expected UnknownType error"),
        }
    }

    #[test]
    fn test_get_payload_from_package() {
        let mut package = loquat_proto::v1::Package {
            flow_id: "test-flow".to_string(),
            pool: loquat_proto::v1::PoolState::Input as i32,
            targets: vec![],
            payload: None,
            meta: None,
            trace: vec![],
            created_at: None,
            updated_at: None,
        };
        
        let text = TextPayload::new("Test payload");
        set_payload_in_package(&mut package, text).unwrap();
        
        let extracted = get_payload_from_package::<TextPayload>(&package).unwrap();
        assert_eq!(extracted.content, "Test payload");
    }

    #[test]
    fn test_get_payload_from_package_no_payload() {
        let package = loquat_proto::v1::Package {
            flow_id: "test".to_string(),
            pool: 0,
            targets: vec![],
            payload: None,
            meta: None,
            trace: vec![],
            created_at: None,
            updated_at: None,
        };
        
        let result = get_payload_from_package::<TextPayload>(&package);
        assert!(result.is_err());
    }
}
