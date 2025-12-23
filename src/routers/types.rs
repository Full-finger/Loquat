//! Router types

use serde::{Deserialize, Serialize};

/// Routing target for adapters
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RouteTarget {
    /// Direct to a specific adapter
    Adapter(String),
    /// Broadcast to all adapters
    Broadcast,
    /// No specific target
    None,
}

/// Package routing state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteState {
    /// Whether the package has been initialized (marked with init flag)
    pub initialized: bool,
    
    /// Target adapter for this package
    pub adapter_target: RouteTarget,
    
    /// Channel type (determined from routing logic)
    pub channel_type: Option<String>,
}

impl Default for RouteState {
    fn default() -> Self {
        Self {
            initialized: false,
            adapter_target: RouteTarget::None,
            channel_type: None,
        }
    }
}

impl RouteState {
    /// Create a new route state
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Mark as initialized
    pub fn with_initialized(mut self) -> Self {
        self.initialized = true;
        self
    }
    
    /// Set adapter target
    pub fn with_adapter_target(mut self, target: RouteTarget) -> Self {
        self.adapter_target = target;
        self
    }
    
    /// Set channel type
    pub fn with_channel_type(mut self, channel_type: &str) -> Self {
        self.channel_type = Some(channel_type.to_string());
        self
    }
}

/// Routing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    /// Default adapter if no specific match found
    pub default_adapter: Option<String>,
    
    /// Adapter mapping for group channels
    pub group_adapter: Option<String>,
    
    /// Adapter mapping for private channels
    pub private_adapter: Option<String>,
    
    /// Adapter mapping for channel (DM) channels
    pub channel_adapter: Option<String>,
    
    /// Enable auto-initialization (mark packages with init flag)
    pub auto_initialize: bool,
    
    /// Enable automatic routing based on IDs
    pub auto_route: bool,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            default_adapter: None,
            group_adapter: None,
            private_adapter: None,
            channel_adapter: None,
            auto_initialize: true,
            auto_route: true,
        }
    }
}

impl RouterConfig {
    /// Create a new router config
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set default adapter
    pub fn with_default_adapter(mut self, adapter: &str) -> Self {
        self.default_adapter = Some(adapter.to_string());
        self
    }
    
    /// Set group adapter
    pub fn with_group_adapter(mut self, adapter: &str) -> Self {
        self.group_adapter = Some(adapter.to_string());
        self
    }
    
    /// Set private adapter
    pub fn with_private_adapter(mut self, adapter: &str) -> Self {
        self.private_adapter = Some(adapter.to_string());
        self
    }
    
    /// Set channel adapter
    pub fn with_channel_adapter(mut self, adapter: &str) -> Self {
        self.channel_adapter = Some(adapter.to_string());
        self
    }
    
    /// Set auto initialize
    pub fn with_auto_initialize(mut self, enabled: bool) -> Self {
        self.auto_initialize = enabled;
        self
    }
    
    /// Set auto route
    pub fn with_auto_route(mut self, enabled: bool) -> Self {
        self.auto_route = enabled;
        self
    }
}

/// Routing result
#[derive(Debug, Clone)]
pub struct RouteResult {
    /// The route state to apply
    pub state: RouteState,
    
    /// Whether routing was successful
    pub success: bool,
}

impl RouteResult {
    /// Create a new route result
    pub fn new(state: RouteState, success: bool) -> Self {
        Self { state, success }
    }
    
    /// Create a successful route result
    pub fn success(state: RouteState) -> Self {
        Self::new(state, true)
    }
    
    /// Create a failed route result
    pub fn failure(state: RouteState) -> Self {
        Self::new(state, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_state_default() {
        let state = RouteState::default();
        assert!(!state.initialized);
        assert_eq!(state.adapter_target, RouteTarget::None);
        assert!(state.channel_type.is_none());
    }

    #[test]
    fn test_route_state_builder() {
        let state = RouteState::new()
            .with_initialized()
            .with_adapter_target(RouteTarget::Adapter("test_adapter".to_string()))
            .with_channel_type("group");
        
        assert!(state.initialized);
        assert_eq!(state.adapter_target, RouteTarget::Adapter("test_adapter".to_string()));
        assert_eq!(state.channel_type, Some("group".to_string()));
    }

    #[test]
    fn test_router_config_default() {
        let config = RouterConfig::default();
        assert!(config.auto_initialize);
        assert!(config.auto_route);
        assert!(config.default_adapter.is_none());
    }

    #[test]
    fn test_router_config_builder() {
        let config = RouterConfig::new()
            .with_default_adapter("default")
            .with_group_adapter("group_adapter")
            .with_auto_initialize(false);
        
        assert_eq!(config.default_adapter, Some("default".to_string()));
        assert_eq!(config.group_adapter, Some("group_adapter".to_string()));
        assert!(!config.auto_initialize);
    }

    #[test]
    fn test_route_result() {
        let state = RouteState::new().with_initialized();
        let success = RouteResult::success(state.clone());
        let failure = RouteResult::failure(state.clone());
        
        assert!(success.success);
        assert!(!failure.success);
    }

    #[test]
    fn test_route_target_serialization() {
        let target = RouteTarget::Adapter("test".to_string());
        let json = serde_json::to_string(&target).unwrap();
        let deserialized: RouteTarget = serde_json::from_str(&json).unwrap();
        assert_eq!(target, deserialized);
    }
}
