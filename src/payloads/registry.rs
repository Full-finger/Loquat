//! Payload 注册表 - 动态类型注册和解析
//!
//! 提供运行时 Payload 类型注册和反序列化支持

use prost_types::Any;
use crate::payloads::{UniversalPayload, PayloadError, TypeUrl};
use thiserror::Error;
use std::sync::RwLock;
use std::collections::HashMap;

// ========================================================================
// 注册错误
// ========================================================================

/// 注册表相关错误
#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Type already registered: {0}")]
    AlreadyRegistered(String),
    
    #[error("Type not registered: {0}")]
    NotRegistered(String),
    
    #[error("Payload deserialization failed: {0}")]
    DeserializationError(String),
}

/// 反序列化函数类型
pub type DeserializerFn = Box<dyn Fn(&Any) -> Result<Box<dyn UniversalPayload>, PayloadError> + Send + Sync>;

// ========================================================================
// Payload Registry
// ========================================================================

/// Payload 类型信息
struct TypeInfo {
    type_url: String,
    deserializer: DeserializerFn,
}

/// 全局 Payload 注册表
pub struct PayloadRegistry {
    types: RwLock<HashMap<String, TypeInfo>>,
}

impl PayloadRegistry {
    /// 创建新的注册表
    pub fn new() -> Self {
        Self {
            types: RwLock::new(HashMap::new()),
        }
    }
    
    /// 注册一个 Payload 类型
    ///
    /// # 参数
    /// - `T`: Payload 类型，必须实现 UniversalPayload
    ///
    /// # 示例
    /// ```rust
    /// use loquat::payloads::{PayloadRegistry, TextPayload};
    ///
    /// let registry = PayloadRegistry::new();
    /// registry.register::<TextPayload>().unwrap();
    /// ```
    pub fn register<T: UniversalPayload + TypeUrl>(&self) -> Result<(), RegistryError> {
        let type_url = T::type_url().to_string();
        let mut types = self.types.write().unwrap();
        
        if types.contains_key(&type_url) {
            return Err(RegistryError::AlreadyRegistered(type_url));
        }
        
        let type_info = TypeInfo {
            type_url: type_url.clone(),
            deserializer: Box::new(|any| {
                T::from_any(any)
                    .map(|p| Box::new(p) as Box<dyn UniversalPayload>)
                    .map_err(|e| PayloadError::from(e))
            }),
        };
        
        types.insert(type_url, type_info);
        Ok(())
    }
    
    /// 反序列化 Payload
    ///
    /// # 参数
    /// - `type_url`: Payload 的类型 URL
    /// - `any`: proto::Any 数据
    ///
    /// # 返回
    /// 反序列化的 Payload，或错误
    ///
    /// # 示例
    /// ```rust
    /// use loquat::payloads::{PayloadRegistry, from_any};
    /// use prost_types::Any;
    ///
    /// let registry = PayloadRegistry::new();
    /// registry.register::<TextPayload>().unwrap();
    ///
    /// let any = Any { /* ... */ };
    /// let payload = registry.deserialize(&any.type_url, &any).unwrap();
    /// ```
    pub fn deserialize(
        &self,
        type_url: &str,
        any: &Any,
    ) -> Result<Box<dyn UniversalPayload>, PayloadError> {
        let types = self.types.read().unwrap();
        
        match types.get(type_url) {
            Some(type_info) => (type_info.deserializer)(any),
            None => Err(PayloadError::UnknownType(type_url.to_string())),
        }
    }
    
    /// 检查类型是否已注册
    ///
    /// # 参数
    /// - `type_url`: 要检查的类型 URL
    ///
    /// # 返回
    /// `true` 如果类型已注册，否则 `false`
    pub fn is_registered(&self, type_url: &str) -> bool {
        let types = self.types.read().unwrap();
        types.contains_key(type_url)
    }
    
    /// 获取所有已注册的类型 URL
    ///
    /// # 返回
    /// 已注册类型 URL 的向量
    pub fn registered_types(&self) -> Vec<String> {
        let types = self.types.read().unwrap();
        types.keys().cloned().collect()
    }
    
    /// 获取已注册类型的数量
    ///
    /// # 返回
    /// 已注册类型的数量
    pub fn count(&self) -> usize {
        let types = self.types.read().unwrap();
        types.len()
    }
}

impl Default for PayloadRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ========================================================================
// 全局默认注册表
// ========================================================================

use std::sync::OnceLock;

/// 全局默认 Payload 注册表
static DEFAULT_REGISTRY: OnceLock<PayloadRegistry> = OnceLock::new();

/// 获取全局默认注册表
///
/// # 返回
/// 全局注册表的引用
///
/// # 示例
/// ```rust
/// use loquat::payloads::{default_registry, TextPayload};
///
/// let registry = default_registry();
/// registry.register::<TextPayload>().unwrap();
/// ```
pub fn default_registry() -> &'static PayloadRegistry {
    DEFAULT_REGISTRY.get_or_init(|| {
        let registry = PayloadRegistry::new();
        
        // 注册内置类型
        let _ = registry.register::<crate::payloads::TextPayload>();
        let _ = registry.register::<crate::payloads::BlobPayload>();
        let _ = registry.register::<crate::payloads::EventPayload>();
        
        registry
    })
}

// ========================================================================
// 便捷函数
// ========================================================================

/// 使用默认注册表反序列化 Payload
///
/// # 参数
/// - `any`: proto::Any 数据
///
/// # 返回
/// 反序列化的 Payload，或错误
///
/// # 示例
/// ```rust
/// use loquat::payloads::{deserialize_with_default};
/// use prost_types::Any;
///
/// let any = Any { /* TextPayload 的 Any */ };
/// let payload = deserialize_with_default(&any).unwrap();
/// ```
pub fn deserialize_with_default(any: &Any) -> Result<Box<dyn UniversalPayload>, PayloadError> {
    default_registry().deserialize(&any.type_url, any)
}

/// 初始化默认注册表（如果尚未初始化）
///
/// # 示例
/// ```rust
/// use loquat::payloads::init_default_registry;
///
/// init_default_registry(); // 确保默认类型已注册
/// ```
pub fn init_default_registry() {
    let _ = default_registry();
}

// ========================================================================
// 测试
// ========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payloads::{TextPayload, BlobPayload, EventPayload};

    #[test]
    fn test_registry_new() {
        let registry = PayloadRegistry::new();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_register_type() {
        let registry = PayloadRegistry::new();
        registry.register::<TextPayload>().unwrap();
        
        assert!(registry.is_registered(TextPayload::type_url()));
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_register_duplicate() {
        let registry = PayloadRegistry::new();
        registry.register::<TextPayload>().unwrap();
        
        let result = registry.register::<TextPayload>();
        assert!(result.is_err());
        match result.unwrap_err() {
            RegistryError::AlreadyRegistered(type_url) => {
                assert_eq!(type_url, TextPayload::type_url());
            }
            _ => panic!("Expected AlreadyRegistered error"),
        }
    }

    #[test]
    fn test_deserialize() {
        let registry = PayloadRegistry::new();
        registry.register::<TextPayload>().unwrap();
        
        let text = TextPayload::new("Test");
        let any = text.to_any().unwrap();
        
        let payload = registry.deserialize(&any.type_url, &any).unwrap();
        
        if let Some(restored) = payload.as_any().downcast_ref::<TextPayload>() {
            assert_eq!(restored.content, "Test");
        } else {
            panic!("Failed to downcast to TextPayload");
        }
    }

    #[test]
    fn test_deserialize_unregistered() {
        let registry = PayloadRegistry::new();
        // 不注册任何类型
        
        let any = Any {
            type_url: TextPayload::type_url().to_string(),
            value: vec![1, 2, 3],
        };
        
        let result = registry.deserialize(&any.type_url, &any);
        assert!(result.is_err());
        match result.unwrap_err() {
            PayloadError::UnknownType(type_url) => {
                assert_eq!(type_url, TextPayload::type_url());
            }
            _ => panic!("Expected UnknownType error"),
        }
    }

    #[test]
    fn test_registered_types() {
        let registry = PayloadRegistry::new();
        registry.register::<TextPayload>().unwrap();
        
        let types = registry.registered_types();
        assert_eq!(types.len(), 1);
        assert!(types.contains(&TextPayload::type_url().to_string()));
    }

    #[test]
    fn test_default_registry() {
        let registry = default_registry();
        
        // 默认注册表应该包含内置类型
        assert!(registry.is_registered(TextPayload::type_url()));
        assert!(registry.is_registered(BlobPayload::type_url()));
        assert!(registry.is_registered(EventPayload::type_url()));
        
        // 计数应该至少为 3
        assert!(registry.count() >= 3);
    }

    #[test]
    fn test_deserialize_with_default() {
        // 初始化默认注册表
        init_default_registry();
        
        let text = TextPayload::new("Hello from default registry!");
        let any = text.to_any().unwrap();
        
        let payload = deserialize_with_default(&any).unwrap();
        
        if let Some(restored) = payload.as_any().downcast_ref::<TextPayload>() {
            assert_eq!(restored.content, "Hello from default registry!");
        } else {
            panic!("Failed to deserialize with default registry");
        }
    }

    #[test]
    fn test_multiple_registries() {
        let registry1 = PayloadRegistry::new();
        let registry2 = PayloadRegistry::new();
        
        // 两个注册表应该独立
        registry1.register::<TextPayload>().unwrap();
        assert!(!registry2.is_registered(TextPayload::type_url()));
        
        registry2.register::<BlobPayload>().unwrap();
        assert!(!registry1.is_registered(BlobPayload::type_url()));
    }
}
