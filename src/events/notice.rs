//! Notice events for Loquat event system
//!
//! Notice events include group member changes, system notifications, etc.

use crate::events::traits::{Event, EventMetadata, EventSource};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::fmt::Debug;

/// Notice event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NoticeEvent {
    /// Group member join
    GroupMemberJoin {
        /// User ID who joined
        user_id: String,
        /// Group ID
        group_id: String,
        /// User information
        user_info: Option<UserInfo>,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Group member leave
    GroupMemberLeave {
        /// User ID who left
        user_id: String,
        /// Group ID
        group_id: String,
        /// Leave reason
        reason: Option<String>,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Group member kick
    GroupMemberKick {
        /// User ID who was kicked
        user_id: String,
        /// Group ID
        group_id: String,
        /// Operator user ID who kicked
        operator_id: String,
        /// Kick reason
        reason: Option<String>,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Group member ban
    GroupMemberBan {
        /// User ID who was banned
        user_id: String,
        /// Group ID
        group_id: String,
        /// Operator user ID who banned
        operator_id: String,
        /// Ban duration in seconds (0 for permanent)
        duration: Option<u64>,
        /// Ban reason
        reason: Option<String>,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Group member mute
    GroupMemberMute {
        /// User ID who was muted
        user_id: String,
        /// Group ID
        group_id: String,
        /// Operator user ID who muted
        operator_id: String,
        /// Mute duration in seconds (0 for permanent)
        duration: Option<u64>,
        /// Mute reason
        reason: Option<String>,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Group name change
    GroupNameChange {
        /// Old group name
        old_name: String,
        /// New group name
        new_name: String,
        /// Operator user ID who changed
        operator_id: String,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Friend add
    FriendAdd {
        /// User ID added as friend
        user_id: String,
        /// User information
        user_info: Option<UserInfo>,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Friend delete
    FriendDelete {
        /// User ID removed from friends
        user_id: String,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Group invitation
    GroupInvite {
        /// Group ID
        group_id: String,
        /// Inviter user ID
        inviter_id: String,
        /// Invitee user ID
        invitee_id: Option<String>,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Group disband
    GroupDisband {
        /// Group ID
        group_id: String,
        /// Operator user ID who disbanded
        operator_id: String,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Friend request notice
    FriendRequestNotice {
        /// From user ID
        from_user_id: String,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// System notice
    SystemNotice {
        /// Notice type
        notice_type: String,
        /// Notice content
        content: String,
        /// Event metadata
        metadata: EventMetadata,
    },
}

/// User information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserInfo {
    /// User nickname
    pub nickname: Option<String>,
    
    /// User avatar URL
    pub avatar: Option<String>,
    
    /// User card
    pub card: Option<String>,
    
    /// User sex
    pub sex: Option<String>,
    
    /// User age
    pub age: Option<u32>,
}

impl Event for NoticeEvent {
    fn event_id(&self) -> &str {
        match self {
            NoticeEvent::GroupMemberJoin { metadata, .. } => &metadata.event_id,
            NoticeEvent::GroupMemberLeave { metadata, .. } => &metadata.event_id,
            NoticeEvent::GroupMemberKick { metadata, .. } => &metadata.event_id,
            NoticeEvent::GroupMemberBan { metadata, .. } => &metadata.event_id,
            NoticeEvent::GroupMemberMute { metadata, .. } => &metadata.event_id,
            NoticeEvent::GroupNameChange { metadata, .. } => &metadata.event_id,
            NoticeEvent::FriendAdd { metadata, .. } => &metadata.event_id,
            NoticeEvent::FriendDelete { metadata, .. } => &metadata.event_id,
            NoticeEvent::GroupInvite { metadata, .. } => &metadata.event_id,
            NoticeEvent::GroupDisband { metadata, .. } => &metadata.event_id,
            NoticeEvent::FriendRequestNotice { metadata, .. } => &metadata.event_id,
            NoticeEvent::SystemNotice { metadata, .. } => &metadata.event_id,
        }
    }
    
    fn event_type(&self) -> &str {
        match self {
            NoticeEvent::GroupMemberJoin { .. } => "notice.group.member.join",
            NoticeEvent::GroupMemberLeave { .. } => "notice.group.member.leave",
            NoticeEvent::GroupMemberKick { .. } => "notice.group.member.kick",
            NoticeEvent::GroupMemberBan { .. } => "notice.group.member.ban",
            NoticeEvent::GroupMemberMute { .. } => "notice.group.member.mute",
            NoticeEvent::GroupNameChange { .. } => "notice.group.name.change",
            NoticeEvent::FriendAdd { .. } => "notice.friend.add",
            NoticeEvent::FriendDelete { .. } => "notice.friend.delete",
            NoticeEvent::GroupInvite { .. } => "notice.group.invite",
            NoticeEvent::GroupDisband { .. } => "notice.group.disband",
            NoticeEvent::FriendRequestNotice { .. } => "notice.friend.request",
            NoticeEvent::SystemNotice { .. } => "notice.system",
        }
    }
    
    fn timestamp(&self) -> DateTime<Utc> {
        match self {
            NoticeEvent::GroupMemberJoin { metadata, .. } => metadata.timestamp,
            NoticeEvent::GroupMemberLeave { metadata, .. } => metadata.timestamp,
            NoticeEvent::GroupMemberKick { metadata, .. } => metadata.timestamp,
            NoticeEvent::GroupMemberBan { metadata, .. } => metadata.timestamp,
            NoticeEvent::GroupMemberMute { metadata, .. } => metadata.timestamp,
            NoticeEvent::GroupNameChange { metadata, .. } => metadata.timestamp,
            NoticeEvent::FriendAdd { metadata, .. } => metadata.timestamp,
            NoticeEvent::FriendDelete { metadata, .. } => metadata.timestamp,
            NoticeEvent::GroupInvite { metadata, .. } => metadata.timestamp,
            NoticeEvent::GroupDisband { metadata, .. } => metadata.timestamp,
            NoticeEvent::FriendRequestNotice { metadata, .. } => metadata.timestamp,
            NoticeEvent::SystemNotice { metadata, .. } => metadata.timestamp,
        }
    }
    
    fn source(&self) -> EventSource {
        match self {
            NoticeEvent::GroupMemberJoin { metadata, .. } => metadata.source.clone(),
            NoticeEvent::GroupMemberLeave { metadata, .. } => metadata.source.clone(),
            NoticeEvent::GroupMemberKick { metadata, .. } => metadata.source.clone(),
            NoticeEvent::GroupMemberBan { metadata, .. } => metadata.source.clone(),
            NoticeEvent::GroupMemberMute { metadata, .. } => metadata.source.clone(),
            NoticeEvent::GroupNameChange { metadata, .. } => metadata.source.clone(),
            NoticeEvent::FriendAdd { metadata, .. } => metadata.source.clone(),
            NoticeEvent::FriendDelete { metadata, .. } => metadata.source.clone(),
            NoticeEvent::GroupInvite { metadata, .. } => metadata.source.clone(),
            NoticeEvent::GroupDisband { metadata, .. } => metadata.source.clone(),
            NoticeEvent::FriendRequestNotice { metadata, .. } => metadata.source.clone(),
            NoticeEvent::SystemNotice { metadata, .. } => metadata.source.clone(),
        }
    }
    
    fn user_id(&self) -> Option<&str> {
        match self {
            NoticeEvent::GroupMemberJoin { user_id, .. } => Some(user_id),
            NoticeEvent::GroupMemberLeave { user_id, .. } => Some(user_id),
            NoticeEvent::GroupMemberKick { user_id, .. } => Some(user_id),
            NoticeEvent::GroupMemberBan { user_id, .. } => Some(user_id),
            NoticeEvent::GroupMemberMute { user_id, .. } => Some(user_id),
            NoticeEvent::GroupNameChange { .. } => None,
            NoticeEvent::FriendAdd { user_id, .. } => Some(user_id),
            NoticeEvent::FriendDelete { user_id, .. } => Some(user_id),
            NoticeEvent::GroupInvite { .. } => None,
            NoticeEvent::GroupDisband { .. } => None,
            NoticeEvent::FriendRequestNotice { from_user_id, .. } => Some(from_user_id),
            NoticeEvent::SystemNotice { .. } => None,
        }
    }
    
    fn group_id(&self) -> Option<&str> {
        match self {
            NoticeEvent::GroupMemberJoin { group_id, .. } => Some(group_id),
            NoticeEvent::GroupMemberLeave { group_id, .. } => Some(group_id),
            NoticeEvent::GroupMemberKick { group_id, .. } => Some(group_id),
            NoticeEvent::GroupMemberBan { group_id, .. } => Some(group_id),
            NoticeEvent::GroupMemberMute { group_id, .. } => Some(group_id),
            NoticeEvent::GroupNameChange { .. } => None,
            NoticeEvent::FriendAdd { .. } => None,
            NoticeEvent::FriendDelete { .. } => None,
            NoticeEvent::GroupInvite { group_id, .. } => Some(group_id),
            NoticeEvent::GroupDisband { group_id, .. } => Some(group_id),
            NoticeEvent::FriendRequestNotice { .. } => None,
            NoticeEvent::SystemNotice { .. } => None,
        }
    }
    
    fn self_id(&self) -> Option<&str> {
        match self {
            NoticeEvent::GroupMemberJoin { metadata, .. } => metadata.self_id.as_deref(),
            NoticeEvent::GroupMemberLeave { metadata, .. } => metadata.self_id.as_deref(),
            NoticeEvent::GroupMemberKick { metadata, .. } => metadata.self_id.as_deref(),
            NoticeEvent::GroupMemberBan { metadata, .. } => metadata.self_id.as_deref(),
            NoticeEvent::GroupMemberMute { metadata, .. } => metadata.self_id.as_deref(),
            NoticeEvent::GroupNameChange { metadata, .. } => metadata.self_id.as_deref(),
            NoticeEvent::FriendAdd { metadata, .. } => metadata.self_id.as_deref(),
            NoticeEvent::FriendDelete { metadata, .. } => metadata.self_id.as_deref(),
            NoticeEvent::GroupInvite { metadata, .. } => metadata.self_id.as_deref(),
            NoticeEvent::GroupDisband { metadata, .. } => metadata.self_id.as_deref(),
            NoticeEvent::FriendRequestNotice { metadata, .. } => metadata.self_id.as_deref(),
            NoticeEvent::SystemNotice { metadata, .. } => metadata.self_id.as_deref(),
        }
    }
    
    fn correlation_id(&self) -> Option<&str> {
        match self {
            NoticeEvent::GroupMemberJoin { metadata, .. } => metadata.correlation_id.as_deref(),
            NoticeEvent::GroupMemberLeave { metadata, .. } => metadata.correlation_id.as_deref(),
            NoticeEvent::GroupMemberKick { metadata, .. } => metadata.correlation_id.as_deref(),
            NoticeEvent::GroupMemberBan { metadata, .. } => metadata.correlation_id.as_deref(),
            NoticeEvent::GroupMemberMute { metadata, .. } => metadata.correlation_id.as_deref(),
            NoticeEvent::GroupNameChange { metadata, .. } => metadata.correlation_id.as_deref(),
            NoticeEvent::FriendAdd { metadata, .. } => metadata.correlation_id.as_deref(),
            NoticeEvent::FriendDelete { metadata, .. } => metadata.correlation_id.as_deref(),
            NoticeEvent::GroupInvite { metadata, .. } => metadata.correlation_id.as_deref(),
            NoticeEvent::GroupDisband { metadata, .. } => metadata.correlation_id.as_deref(),
            NoticeEvent::FriendRequestNotice { metadata, .. } => metadata.correlation_id.as_deref(),
            NoticeEvent::SystemNotice { metadata, .. } => metadata.correlation_id.as_deref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notice_event_group_member_join() {
        let metadata = EventMetadata::new("notice.group.member.join")
            .with_user_id("user123")
            .with_group_id("group456");
        
        let event = NoticeEvent::GroupMemberJoin {
            user_id: "user123".to_string(),
            group_id: "group456".to_string(),
            user_info: None,
            metadata: metadata.clone(),
        };
        
        assert_eq!(event.event_type(), "notice.group.member.join");
        assert_eq!(event.user_id(), Some("user123"));
        assert_eq!(event.group_id(), Some("group456"));
    }
    
    #[test]
    fn test_notice_event_friend_add() {
        let metadata = EventMetadata::new("notice.friend.add")
            .with_user_id("user123");
        
        let event = NoticeEvent::FriendAdd {
            user_id: "user123".to_string(),
            user_info: None,
            metadata: metadata.clone(),
        };
        
        assert_eq!(event.event_type(), "notice.friend.add");
        assert_eq!(event.user_id(), Some("user123"));
        assert_eq!(event.group_id(), None);
    }
    
    #[test]
    fn test_notice_event_system() {
        let metadata = EventMetadata::new("notice.system")
            .with_source(EventSource::System);
        
        let event = NoticeEvent::SystemNotice {
            notice_type: "maintenance".to_string(),
            content: "System will be down for maintenance".to_string(),
            metadata: metadata.clone(),
        };
        
        assert_eq!(event.event_type(), "notice.system");
        assert_eq!(event.source(), EventSource::System);
    }
}
