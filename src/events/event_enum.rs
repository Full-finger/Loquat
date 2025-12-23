//! Event enum module - unified event types

use crate::events::message::MessageEvent;
use crate::events::notice::NoticeEvent;
use crate::events::request::RequestEvent;
use crate::events::meta::MetaEvent;
use crate::events::traits::Event;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// 统一的事件枚举，包装所有事件类型
/// 用于需要使用 trait 对象的场景
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event_category")]
#[serde(rename_all = "snake_case")]
pub enum EventEnum {
    /// 消息事件
    Message(MessageEvent),
    /// 通知事件
    Notice(NoticeEvent),
    /// 请求事件
    Request(RequestEvent),
    /// 元事件
    Meta(MetaEvent),
}

/// 事件状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// 未知状态
    Unknown,
    /// 等待处理
    Pending,
    /// 处理中
    Processing,
    /// 已成功
    Succeeded,
    /// 已失败
    Failed,
    /// 已取消
    Cancelled,
}

impl EventEnum {
    /// 获取事件 ID
    pub fn event_id(&self) -> &str {
        match self {
            EventEnum::Message(evt) => evt.event_id(),
            EventEnum::Notice(evt) => evt.event_id(),
            EventEnum::Request(evt) => evt.event_id(),
            EventEnum::Meta(evt) => evt.event_id(),
        }
    }

    /// 获取事件类型
    pub fn event_type(&self) -> &str {
        match self {
            EventEnum::Message(evt) => evt.event_type(),
            EventEnum::Notice(evt) => evt.event_type(),
            EventEnum::Request(evt) => evt.event_type(),
            EventEnum::Meta(evt) => evt.event_type(),
        }
    }

    /// 获取时间戳
    pub fn timestamp(&self) -> i64 {
        match self {
            EventEnum::Message(evt) => evt.timestamp().timestamp_millis(),
            EventEnum::Notice(evt) => evt.timestamp().timestamp_millis(),
            EventEnum::Request(evt) => evt.timestamp().timestamp_millis(),
            EventEnum::Meta(evt) => evt.timestamp().timestamp_millis(),
        }
    }

    /// 获取事件源
    pub fn source(&self) -> String {
        match self {
            EventEnum::Message(evt) => {
                match evt.source() {
                    crate::events::EventSource::User => "user".to_string(),
                    crate::events::EventSource::System => "system".to_string(),
                    crate::events::EventSource::Api => "api".to_string(),
                    crate::events::EventSource::Worker(s) => format!("worker:{}", s),
                    crate::events::EventSource::Unknown => "unknown".to_string(),
                }
            }
            EventEnum::Notice(evt) => {
                match evt.source() {
                    crate::events::EventSource::User => "user".to_string(),
                    crate::events::EventSource::System => "system".to_string(),
                    crate::events::EventSource::Api => "api".to_string(),
                    crate::events::EventSource::Worker(s) => format!("worker:{}", s),
                    crate::events::EventSource::Unknown => "unknown".to_string(),
                }
            }
            EventEnum::Request(evt) => {
                match evt.source() {
                    crate::events::EventSource::User => "user".to_string(),
                    crate::events::EventSource::System => "system".to_string(),
                    crate::events::EventSource::Api => "api".to_string(),
                    crate::events::EventSource::Worker(s) => format!("worker:{}", s),
                    crate::events::EventSource::Unknown => "unknown".to_string(),
                }
            }
            EventEnum::Meta(evt) => {
                match evt.source() {
                    crate::events::EventSource::User => "user".to_string(),
                    crate::events::EventSource::System => "system".to_string(),
                    crate::events::EventSource::Api => "api".to_string(),
                    crate::events::EventSource::Worker(s) => format!("worker:{}", s),
                    crate::events::EventSource::Unknown => "unknown".to_string(),
                }
            }
        }
    }

    /// 获取用户 ID
    pub fn user_id(&self) -> Option<&str> {
        match self {
            EventEnum::Message(evt) => evt.user_id(),
            EventEnum::Notice(evt) => evt.user_id(),
            EventEnum::Request(evt) => evt.user_id(),
            EventEnum::Meta(evt) => evt.user_id(),
        }
    }

    /// 获取群组 ID
    pub fn group_id(&self) -> Option<&str> {
        match self {
            EventEnum::Message(evt) => evt.group_id(),
            EventEnum::Notice(evt) => evt.group_id(),
            EventEnum::Request(evt) => evt.group_id(),
            EventEnum::Meta(evt) => evt.group_id(),
        }
    }

    /// 获取机器人自身 ID
    pub fn self_id(&self) -> Option<&str> {
        match self {
            EventEnum::Message(evt) => evt.self_id(),
            EventEnum::Notice(evt) => evt.self_id(),
            EventEnum::Request(evt) => evt.self_id(),
            EventEnum::Meta(evt) => evt.self_id(),
        }
    }

    /// 获取关联 ID
    pub fn correlation_id(&self) -> Option<&str> {
        match self {
            EventEnum::Message(evt) => evt.correlation_id(),
            EventEnum::Notice(evt) => evt.correlation_id(),
            EventEnum::Request(evt) => evt.correlation_id(),
            EventEnum::Meta(evt) => evt.correlation_id(),
        }
    }

    /// 判断是否为消息事件
    pub fn is_message(&self) -> bool {
        matches!(self, EventEnum::Message(_))
    }

    /// 判断是否为通知事件
    pub fn is_notice(&self) -> bool {
        matches!(self, EventEnum::Notice(_))
    }

    /// 判断是否为请求事件
    pub fn is_request(&self) -> bool {
        matches!(self, EventEnum::Request(_))
    }

    /// 判断是否为元事件
    pub fn is_meta(&self) -> bool {
        matches!(self, EventEnum::Meta(_))
    }

    /// 转换为消息事件
    pub fn as_message(&self) -> Option<&MessageEvent> {
        match self {
            EventEnum::Message(evt) => Some(evt),
            _ => None,
        }
    }

    /// 转换为通知事件
    pub fn as_notice(&self) -> Option<&NoticeEvent> {
        match self {
            EventEnum::Notice(evt) => Some(evt),
            _ => None,
        }
    }

    /// 转换为请求事件
    pub fn as_request(&self) -> Option<&RequestEvent> {
        match self {
            EventEnum::Request(evt) => Some(evt),
            _ => None,
        }
    }

    /// 转换为元事件
    pub fn as_meta(&self) -> Option<&MetaEvent> {
        match self {
            EventEnum::Meta(evt) => Some(evt),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_enum_message() {
        use crate::events::traits::EventMetadata;

        let message_event = MessageEvent::Text {
            text: "Hello".to_string(),
            metadata: EventMetadata::default(),
        };

        let event_enum = EventEnum::Message(message_event);

        assert!(event_enum.is_message());
        assert!(!event_enum.is_notice());
        assert!(event_enum.as_message().is_some());
    }

    #[test]
    fn test_event_enum_serialization() {
        use crate::events::traits::EventMetadata;

        let message_event = MessageEvent::Text {
            text: "Hello".to_string(),
            metadata: EventMetadata::default(),
        };

        let event_enum = EventEnum::Message(message_event);

        let json = serde_json::to_string(&event_enum).unwrap();
        let deserialized: EventEnum = serde_json::from_str(&json).unwrap();

        assert_eq!(event_enum, deserialized);
    }
}
