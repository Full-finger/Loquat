//! TargetSite - worker identification for Package targeting
//!
//! TargetSite is "作用靶点" - worker's identification for Package matching.
//!
//! Four-dimensional classification:
//! - Domain: Material type (text, image, audio, video, event, etc.)
//! - Motif: Structural feature (command, mention, url, hashtag, etc.)
//! - State: Functional state (intent_weather, spam_suspected, needs_api, etc.)
//! - Context: Contextual information (user_vip, group_night_mode, admin, etc.)

use serde::{Serialize, Deserialize};
use std::fmt::Debug;

/// Domain tag - material type classification
/// Represents what the content IS (material type)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainTag {
    /// Text content
    Text,
    /// Image content
    Image,
    /// Audio content
    Audio,
    /// Video content
    Video,
    /// File content
    File,
    /// Event/notice
    Event,
    /// Generic event
    Generic,
    /// Custom domain tag
    Custom(String),
}

impl DomainTag {
    /// Get the tag as string
    pub fn tag_string(&self) -> String {
        match self {
            Self::Text => "Text".to_string(),
            Self::Image => "Image".to_string(),
            Self::Audio => "Audio".to_string(),
            Self::Video => "Video".to_string(),
            Self::File => "File".to_string(),
            Self::Event => "Event".to_string(),
            Self::Generic => "Generic".to_string(),
            Self::Custom(s) => format!("Custom({})", s),
        }
    }
}

/// Motif tag - structural feature classification
/// Represents what the content LOOKS LIKE (structural pattern)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotifTag {
    /// Command pattern (e.g., /ping, /weather)
    Command,
    /// User mention (@user)
    Mention,
    /// URL/link
    Url,
    /// Hashtag (#tag)
    Hashtag,
    /// Question mark detected
    Question,
    /// Exclamation mark detected
    Exclamation,
    /// Number detected
    Number,
    /// Custom motif tag
    Custom(String),
}

impl MotifTag {
    /// Get the tag as string
    pub fn tag_string(&self) -> String {
        match self {
            Self::Command => "Command".to_string(),
            Self::Mention => "Mention".to_string(),
            Self::Url => "Url".to_string(),
            Self::Hashtag => "Hashtag".to_string(),
            Self::Question => "Question".to_string(),
            Self::Exclamation => "Exclamation".to_string(),
            Self::Number => "Number".to_string(),
            Self::Custom(s) => format!("Custom({})", s),
        }
    }
}

/// State tag - functional state classification
/// Represents the FUNCTIONAL STATE of the message (intent, status)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateTag {
    /// Weather intent detected
    IntentWeather,
    /// Question intent detected
    IntentQuestion,
    /// Spam suspected
    SpamSuspected,
    /// VIP user
    UserVip,
    /// Banned user
    UserBanned,
    /// Admin user
    UserAdmin,
    /// API call needed
    NeedsApiCall,
    /// Response ready
    ResponseReady,
    /// Error state
    Error,
    /// Custom state tag
    Custom(String),
}

impl StateTag {
    /// Get the tag as string
    pub fn tag_string(&self) -> String {
        match self {
            Self::IntentWeather => "IntentWeather".to_string(),
            Self::IntentQuestion => "IntentQuestion".to_string(),
            Self::SpamSuspected => "SpamSuspected".to_string(),
            Self::UserVip => "UserVip".to_string(),
            Self::UserBanned => "UserBanned".to_string(),
            Self::UserAdmin => "UserAdmin".to_string(),
            Self::NeedsApiCall => "NeedsApiCall".to_string(),
            Self::ResponseReady => "ResponseReady".to_string(),
            Self::Error => "Error".to_string(),
            Self::Custom(s) => format!("Custom({})", s),
        }
    }
}

/// Context tag - contextual information classification
/// Represents CONTEXTUAL metadata (environment, scope)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextTag {
    /// Group context
    Group,
    /// Direct message context
    Direct,
    /// Channel context
    Channel,
    /// Night mode active
    NightMode,
    /// Quiet mode active
    QuietMode,
    /// Reply to a message
    Reply,
    /// Forward message
    Forward,
    /// Custom context tag
    Custom(String),
}

impl ContextTag {
    /// Get the tag as string
    pub fn tag_string(&self) -> String {
        match self {
            Self::Group => "Group".to_string(),
            Self::Direct => "Direct".to_string(),
            Self::Channel => "Channel".to_string(),
            Self::NightMode => "NightMode".to_string(),
            Self::QuietMode => "QuietMode".to_string(),
            Self::Reply => "Reply".to_string(),
            Self::Forward => "Forward".to_string(),
            Self::Custom(s) => format!("Custom({})", s),
        }
    }
}

/// TargetSite - four-dimensional target site for matching
/// Replaces the old single-dimension SiteType with semantic clarity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetSite {
    /// Domain dimension - material type
    Domain(DomainTag),
    /// Motif dimension - structural feature
    Motif(MotifTag),
    /// State dimension - functional state
    State(StateTag),
    /// Context dimension - contextual information
    Context(ContextTag),
}

impl TargetSite {
    // ========================================================================
    // Domain constructors
    // ========================================================================
    
    /// Create a domain target site
    pub fn domain(tag: DomainTag) -> Self {
        Self::Domain(tag)
    }
    
    /// Create a text domain target
    pub fn domain_text() -> Self {
        Self::Domain(DomainTag::Text)
    }
    
    /// Create an image domain target
    pub fn domain_image() -> Self {
        Self::Domain(DomainTag::Image)
    }
    
    /// Create an audio domain target
    pub fn domain_audio() -> Self {
        Self::Domain(DomainTag::Audio)
    }
    
    /// Create a video domain target
    pub fn domain_video() -> Self {
        Self::Domain(DomainTag::Video)
    }
    
    /// Create a file domain target
    pub fn domain_file() -> Self {
        Self::Domain(DomainTag::File)
    }
    
    /// Create an event domain target
    pub fn domain_event() -> Self {
        Self::Domain(DomainTag::Event)
    }
    
    /// Create a custom domain target
    pub fn domain_custom(tag: &str) -> Self {
        Self::Domain(DomainTag::Custom(tag.to_string()))
    }
    
    // ========================================================================
    // Motif constructors
    // ========================================================================
    
    /// Create a motif target site
    pub fn motif(tag: MotifTag) -> Self {
        Self::Motif(tag)
    }
    
    /// Create a command motif target
    pub fn motif_command() -> Self {
        Self::Motif(MotifTag::Command)
    }
    
    /// Create a mention motif target
    pub fn motif_mention() -> Self {
        Self::Motif(MotifTag::Mention)
    }
    
    /// Create a URL motif target
    pub fn motif_url() -> Self {
        Self::Motif(MotifTag::Url)
    }
    
    /// Create a hashtag motif target
    pub fn motif_hashtag() -> Self {
        Self::Motif(MotifTag::Hashtag)
    }
    
    /// Create a question motif target
    pub fn motif_question() -> Self {
        Self::Motif(MotifTag::Question)
    }
    
    /// Create a custom motif target
    pub fn motif_custom(tag: &str) -> Self {
        Self::Motif(MotifTag::Custom(tag.to_string()))
    }
    
    // ========================================================================
    // State constructors
    // ========================================================================
    
    /// Create a state target site
    pub fn state(tag: StateTag) -> Self {
        Self::State(tag)
    }
    
    /// Create an intent weather state target
    pub fn state_intent_weather() -> Self {
        Self::State(StateTag::IntentWeather)
    }
    
    /// Create an intent question state target
    pub fn state_intent_question() -> Self {
        Self::State(StateTag::IntentQuestion)
    }
    
    /// Create a spam suspected state target
    pub fn state_spam_suspected() -> Self {
        Self::State(StateTag::SpamSuspected)
    }
    
    /// Create a user VIP state target
    pub fn state_user_vip() -> Self {
        Self::State(StateTag::UserVip)
    }
    
    /// Create a user banned state target
    pub fn state_user_banned() -> Self {
        Self::State(StateTag::UserBanned)
    }
    
    /// Create a needs API call state target
    pub fn state_needs_api() -> Self {
        Self::State(StateTag::NeedsApiCall)
    }
    
    /// Create a response ready state target
    pub fn state_response_ready() -> Self {
        Self::State(StateTag::ResponseReady)
    }
    
    /// Create a custom state target
    pub fn state_custom(tag: &str) -> Self {
        Self::State(StateTag::Custom(tag.to_string()))
    }
    
    // ========================================================================
    // Context constructors
    // ========================================================================
    
    /// Create a context target site
    pub fn context(tag: ContextTag) -> Self {
        Self::Context(tag)
    }
    
    /// Create a group context target
    pub fn context_group() -> Self {
        Self::Context(ContextTag::Group)
    }
    
    /// Create a direct message context target
    pub fn context_direct() -> Self {
        Self::Context(ContextTag::Direct)
    }
    
    /// Create a channel context target
    pub fn context_channel() -> Self {
        Self::Context(ContextTag::Channel)
    }
    
    /// Create a night mode context target
    pub fn context_night_mode() -> Self {
        Self::Context(ContextTag::NightMode)
    }
    
    /// Create a quiet mode context target
    pub fn context_quiet_mode() -> Self {
        Self::Context(ContextTag::QuietMode)
    }
    
    /// Create a reply context target
    pub fn context_reply() -> Self {
        Self::Context(ContextTag::Reply)
    }
    
    /// Create a custom context target
    pub fn context_custom(tag: &str) -> Self {
        Self::Context(ContextTag::Custom(tag.to_string()))
    }
    
    // ========================================================================
    // Helper methods
    // ========================================================================
    
    /// Get the dimension name of this target site
    pub fn dimension(&self) -> &'static str {
        match self {
            Self::Domain(_) => "domain",
            Self::Motif(_) => "motif",
            Self::State(_) => "state",
            Self::Context(_) => "context",
        }
    }
    
    /// Get the tag value as string
    pub fn tag_string(&self) -> String {
        match self {
            Self::Domain(tag) => format!("{:?}", tag),
            Self::Motif(tag) => format!("{:?}", tag),
            Self::State(tag) => format!("{:?}", tag),
            Self::Context(tag) => format!("{:?}", tag),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_constructors() {
        let text = TargetSite::domain_text();
        let image = TargetSite::domain_image();
        let custom = TargetSite::domain_custom("video");
        
        assert_eq!(text.dimension(), "domain");
        assert_eq!(text.tag_string(), "Text");
        
        assert_eq!(image.dimension(), "domain");
        assert_eq!(image.tag_string(), "Image");
        
        assert_eq!(custom.dimension(), "domain");
        assert_eq!(custom.tag_string(), "Custom(\"video\")");
    }

    #[test]
    fn test_motif_constructors() {
        let command = TargetSite::motif_command();
        let mention = TargetSite::motif_mention();
        let custom = TargetSite::motif_custom("emoji");
        
        assert_eq!(command.dimension(), "motif");
        assert_eq!(command.tag_string(), "Command");
        
        assert_eq!(mention.dimension(), "motif");
        assert_eq!(mention.tag_string(), "Mention");
        
        assert_eq!(custom.dimension(), "motif");
        assert_eq!(custom.tag_string(), "Custom(\"emoji\")");
    }

    #[test]
    fn test_state_constructors() {
        let weather = TargetSite::state_intent_weather();
        let vip = TargetSite::state_user_vip();
        let custom = TargetSite::state_custom("processing");
        
        assert_eq!(weather.dimension(), "state");
        assert_eq!(weather.tag_string(), "IntentWeather");
        
        assert_eq!(vip.dimension(), "state");
        assert_eq!(vip.tag_string(), "UserVip");
        
        assert_eq!(custom.dimension(), "state");
        assert_eq!(custom.tag_string(), "Custom(\"processing\")");
    }

    #[test]
    fn test_context_constructors() {
        let group = TargetSite::context_group();
        let night = TargetSite::context_night_mode();
        let custom = TargetSite::context_custom("emergency");
        
        assert_eq!(group.dimension(), "context");
        assert_eq!(group.tag_string(), "Group");
        
        assert_eq!(night.dimension(), "context");
        assert_eq!(night.tag_string(), "NightMode");
        
        assert_eq!(custom.dimension(), "context");
        assert_eq!(custom.tag_string(), "Custom(\"emergency\")");
    }

    #[test]
    fn test_equality() {
        let site1 = TargetSite::domain_text();
        let site2 = TargetSite::domain_text();
        let site3 = TargetSite::domain_image();
        
        assert_eq!(site1, site2);
        assert_ne!(site1, site3);
    }

    #[test]
    fn test_serialization() {
        let site = TargetSite::state_intent_weather();
        
        // Test serialization
        let serialized = serde_json::to_string(&site).unwrap();
        
        // Test deserialization
        let deserialized: TargetSite = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(site, deserialized);
    }
}
