//! Channel type definitions

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Channel type classification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChannelType {
    /// 群聊
    Group { group_id: String },
    
    /// 私聊
    Private { user_id: String },
    
    /// 频道
    Channel { channel_id: String },
}

impl ChannelType {
    /// Get unique identifier for this channel
    pub fn id(&self) -> &str {
        match self {
            Self::Group { group_id } => group_id,
            Self::Private { user_id } => user_id,
            Self::Channel { channel_id } => channel_id,
        }
    }
    
    /// Create a group channel
    pub fn group(group_id: &str) -> Self {
        Self::Group {
            group_id: group_id.to_string(),
        }
    }
    
    /// Create a private channel
    pub fn private(user_id: &str) -> Self {
        Self::Private {
            user_id: user_id.to_string(),
        }
    }
    
    /// Create a channel
    pub fn channel(channel_id: &str) -> Self {
        Self::Channel {
            channel_id: channel_id.to_string(),
        }
    }
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Group { group_id } => write!(f, "group:{}", group_id),
            Self::Private { user_id } => write!(f, "private:{}", user_id),
            Self::Channel { channel_id } => write!(f, "channel:{}", channel_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_type_group() {
        let ct = ChannelType::group("123456");
        assert_eq!(ct.id(), "123456");
        assert_eq!(ct.to_string(), "group:123456");
    }

    #[test]
    fn test_channel_type_private() {
        let ct = ChannelType::private("user123");
        assert_eq!(ct.id(), "user123");
        assert_eq!(ct.to_string(), "private:user123");
    }

    #[test]
    fn test_channel_type_channel() {
        let ct = ChannelType::channel("channel456");
        assert_eq!(ct.id(), "channel456");
        assert_eq!(ct.to_string(), "channel:channel456");
    }
}
