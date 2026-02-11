//! Payloads - 统一的 Payload 类型系统
//!
//! 这个模块提供了统一的 Payload 类型系统，支持：
//! - 具体类型：TextPayload, BlobPayload, EventPayload
//! - Proto Any 适配器
//! - 类型注册表
//! - 向后兼容的 Legacy Payload

// ========================================================================
// 模块导出
// ========================================================================

pub mod adapter;
pub mod registry;
pub mod types;

// 重新导出常用类型
pub use adapter::{PayloadAdapter, to_any, from_any};
pub use registry::{PayloadRegistry, RegistryError};
pub use types::{
    TextPayload, 
    BlobPayload, 
    EventPayload,
    UniversalPayload,
    PayloadType,
    TypeUrl,
    PayloadError,
    TextFormat,
};

// ========================================================================
// 向后兼容：Legacy Payload
// ========================================================================

// 重新导出旧系统用于向后兼容
// TODO: 这些将在后续版本中废弃
pub use crate::events::payloads::{
    MessagePayload, 
    NoticePayload, 
    RequestPayload,
    Payload as LegacyPayload,
};

#[deprecated(since = "0.3.0", note = "Use UniversalPayload trait instead")]
pub use crate::events::payloads::BoxedPayload as LegacyBoxedPayload;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // 验证模块导出
        let _text = TextPayload::new("test");
        let _blob = BlobPayload::new(vec![1, 2, 3], "application/octet-stream".to_string());
        
        // 验证向后兼容性
        let metadata = crate::events::EventMetadata::new("test");
        let _msg = MessagePayload {
            subtype: crate::events::payloads::MessageSubtype::Text,
            content: crate::events::payloads::MessageContent::Text {
                text: "hello".to_string(),
            },
            metadata,
        };
    }
}
