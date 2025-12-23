//! Request events for Loquat event system
//!
//! Request events include friend requests, group invites, etc.

use crate::events::traits::{Event, EventMetadata, EventSource};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::fmt::Debug;

/// Request event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RequestEvent {
    /// Friend request
    FriendRequest {
        /// Requester user ID
        from_user_id: String,
        /// Comment/remark
        comment: Option<String>,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Group invite
    GroupInvite {
        /// Inviter user ID
        inviter_id: String,
        /// Group ID
        group_id: String,
        /// Invite message
        message: Option<String>,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Group join request
    GroupJoinRequest {
        /// User ID requesting to join
        user_id: String,
        /// Group ID
        group_id: String,
        /// Join reason
        reason: Option<String>,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Request approval
    RequestApprove {
        /// Request type
        request_type: String,
        /// Request ID
        request_id: String,
        /// Approver user ID
        approver_id: Option<String>,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Request rejection
    RequestReject {
        /// Request type
        request_type: String,
        /// Request ID
        request_id: String,
        /// Rejector user ID
        rejector_id: Option<String>,
        /// Reject reason
        reason: Option<String>,
        /// Event metadata
        metadata: EventMetadata,
    },
    
    /// Request cancel
    RequestCancel {
        /// Request type
        request_type: String,
        /// Request ID
        request_id: String,
        /// Canceled by
        canceled_by: String,
        /// Cancel reason
        reason: Option<String>,
        /// Event metadata
        metadata: EventMetadata,
    },
}

impl Event for RequestEvent {
    fn event_id(&self) -> &str {
        match self {
            RequestEvent::FriendRequest { metadata, .. } => &metadata.event_id,
            RequestEvent::GroupInvite { metadata, .. } => &metadata.event_id,
            RequestEvent::GroupJoinRequest { metadata, .. } => &metadata.event_id,
            RequestEvent::RequestApprove { metadata, .. } => &metadata.event_id,
            RequestEvent::RequestReject { metadata, .. } => &metadata.event_id,
            RequestEvent::RequestCancel { metadata, .. } => &metadata.event_id,
        }
    }
    
    fn event_type(&self) -> &str {
        match self {
            RequestEvent::FriendRequest { .. } => "request.friend",
            RequestEvent::GroupInvite { .. } => "request.group.invite",
            RequestEvent::GroupJoinRequest { .. } => "request.group.join",
            RequestEvent::RequestApprove { .. } => "request.approve",
            RequestEvent::RequestReject { .. } => "request.reject",
            RequestEvent::RequestCancel { .. } => "request.cancel",
        }
    }
    
    fn timestamp(&self) -> DateTime<Utc> {
        match self {
            RequestEvent::FriendRequest { metadata, .. } => metadata.timestamp,
            RequestEvent::GroupInvite { metadata, .. } => metadata.timestamp,
            RequestEvent::GroupJoinRequest { metadata, .. } => metadata.timestamp,
            RequestEvent::RequestApprove { metadata, .. } => metadata.timestamp,
            RequestEvent::RequestReject { metadata, .. } => metadata.timestamp,
            RequestEvent::RequestCancel { metadata, .. } => metadata.timestamp,
        }
    }
    
    fn source(&self) -> EventSource {
        match self {
            RequestEvent::FriendRequest { metadata, .. } => metadata.source.clone(),
            RequestEvent::GroupInvite { metadata, .. } => metadata.source.clone(),
            RequestEvent::GroupJoinRequest { metadata, .. } => metadata.source.clone(),
            RequestEvent::RequestApprove { metadata, .. } => metadata.source.clone(),
            RequestEvent::RequestReject { metadata, .. } => metadata.source.clone(),
            RequestEvent::RequestCancel { metadata, .. } => metadata.source.clone(),
        }
    }
    
    fn user_id(&self) -> Option<&str> {
        match self {
            RequestEvent::FriendRequest { from_user_id, .. } => Some(from_user_id),
            RequestEvent::GroupInvite { inviter_id, .. } => Some(inviter_id),
            RequestEvent::GroupJoinRequest { user_id, .. } => Some(user_id),
            RequestEvent::RequestApprove { .. } => None,
            RequestEvent::RequestReject { .. } => None,
            RequestEvent::RequestCancel { .. } => None,
        }
    }
    
    fn group_id(&self) -> Option<&str> {
        match self {
            RequestEvent::GroupInvite { group_id, .. } => Some(group_id),
            RequestEvent::GroupJoinRequest { group_id, .. } => Some(group_id),
            RequestEvent::RequestApprove { .. } => None,
            RequestEvent::RequestReject { .. } => None,
            RequestEvent::RequestCancel { .. } => None,
            RequestEvent::FriendRequest { .. } => None,
        }
    }
    
    fn self_id(&self) -> Option<&str> {
        match self {
            RequestEvent::FriendRequest { metadata, .. } => metadata.self_id.as_deref(),
            RequestEvent::GroupInvite { metadata, .. } => metadata.self_id.as_deref(),
            RequestEvent::GroupJoinRequest { metadata, .. } => metadata.self_id.as_deref(),
            RequestEvent::RequestApprove { metadata, .. } => metadata.self_id.as_deref(),
            RequestEvent::RequestReject { metadata, .. } => metadata.self_id.as_deref(),
            RequestEvent::RequestCancel { metadata, .. } => metadata.self_id.as_deref(),
        }
    }
    
    fn correlation_id(&self) -> Option<&str> {
        match self {
            RequestEvent::FriendRequest { metadata, .. } => metadata.correlation_id.as_deref(),
            RequestEvent::GroupInvite { metadata, .. } => metadata.correlation_id.as_deref(),
            RequestEvent::GroupJoinRequest { metadata, .. } => metadata.correlation_id.as_deref(),
            RequestEvent::RequestApprove { metadata, .. } => metadata.correlation_id.as_deref(),
            RequestEvent::RequestReject { metadata, .. } => metadata.correlation_id.as_deref(),
            RequestEvent::RequestCancel { metadata, .. } => metadata.correlation_id.as_deref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_event_friend() {
        let metadata = EventMetadata::new("request.friend")
            .with_user_id("user123");
        
        let event = RequestEvent::FriendRequest {
            from_user_id: "user123".to_string(),
            comment: Some("Let's be friends".to_string()),
            metadata: metadata.clone(),
        };
        
        assert_eq!(event.event_type(), "request.friend");
        assert_eq!(event.user_id(), Some("user123"));
        assert_eq!(event.group_id(), None);
    }
    
    #[test]
    fn test_request_event_group_invite() {
        let metadata = EventMetadata::new("request.group.invite")
            .with_user_id("inviter123")
            .with_group_id("group456");
        
        let event = RequestEvent::GroupInvite {
            inviter_id: "inviter123".to_string(),
            group_id: "group456".to_string(),
            message: Some("Please join our group".to_string()),
            metadata: metadata.clone(),
        };
        
        assert_eq!(event.event_type(), "request.group.invite");
        assert_eq!(event.user_id(), Some("inviter123"));
        assert_eq!(event.group_id(), Some("group456"));
    }
    
    #[test]
    fn test_request_event_approve() {
        let metadata = EventMetadata::new("request.approve")
            .with_self_id("bot123");
        
        let event = RequestEvent::RequestApprove {
            request_type: "friend".to_string(),
            request_id: "req-001".to_string(),
            approver_id: Some("admin123".to_string()),
            metadata: metadata.clone(),
        };
        
        assert_eq!(event.event_type(), "request.approve");
        assert_eq!(event.self_id(), Some("bot123"));
    }
}
