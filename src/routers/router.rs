//! Standard router implementation

use crate::events::EventEnum;
use crate::logging::traits::{LogLevel, LogContext};
use crate::routers::traits::Router;
use crate::routers::types::{RouteResult, RouterConfig, RouteState, RouteTarget};
use async_trait::async_trait;
use std::sync::Arc;

/// Standard router - routes packages based on event IDs
pub struct StandardRouter {
    router_id: String,
    config: RouterConfig,
    logger: Arc<dyn crate::logging::Logger>,
}

impl StandardRouter {
    /// Create a new standard router
    pub fn new(logger: Arc<dyn crate::logging::Logger>) -> Self {
        Self {
            router_id: "standard_router".to_string(),
            config: RouterConfig::new(),
            logger,
        }
    }

    /// Create a router with custom ID
    pub fn with_id(router_id: String, logger: Arc<dyn crate::logging::Logger>) -> Self {
        Self {
            router_id,
            config: RouterConfig::new(),
            logger,
        }
    }

    /// Create a router with custom config
    pub fn with_config(config: RouterConfig, logger: Arc<dyn crate::logging::Logger>) -> Self {
        Self {
            router_id: "standard_router".to_string(),
            config,
            logger,
        }
    }

    /// Extract channel type from package based on event IDs
    fn extract_channel_type(&self, package: &crate::events::Package) -> Option<String> {
        // Check all events in all groups in all blocks
        for block in &package.blocks {
            for group in &block.groups {
                for event in &group.events {
                    // Priority: channel_id > group_id > user_id
                    if let Some(channel_id) = event.channel_id() {
                        return Some(format!("channel:{}", channel_id));
                    }
                    if let Some(group_id) = event.group_id() {
                        return Some(format!("group:{}", group_id));
                    }
                    if let Some(user_id) = event.user_id() {
                        return Some(format!("private:{}", user_id));
                    }
                }
            }
        }
        None
    }

    /// Determine route target based on channel type and config
    fn determine_target(&self, channel_type: &str) -> RouteTarget {
        if channel_type.starts_with("group:") {
            if let Some(adapter) = &self.config.group_adapter {
                return RouteTarget::Adapter(adapter.clone());
            }
        } else if channel_type.starts_with("private:") {
            if let Some(adapter) = &self.config.private_adapter {
                return RouteTarget::Adapter(adapter.clone());
            }
        } else if channel_type.starts_with("channel:") {
            if let Some(adapter) = &self.config.channel_adapter {
                return RouteTarget::Adapter(adapter.clone());
            }
        }

        // Use default adapter if configured
        if let Some(adapter) = &self.config.default_adapter {
            RouteTarget::Adapter(adapter.clone())
        } else {
            RouteTarget::None
        }
    }
}

impl std::fmt::Debug for StandardRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StandardRouter")
            .field("router_id", &self.router_id)
            .field("config", &self.config)
            .finish()
    }
}

#[async_trait]
impl Router for StandardRouter {
    fn router_id(&self) -> &str {
        &self.router_id
    }

    fn config(&self) -> &RouterConfig {
        &self.config
    }

    fn set_config(&mut self, config: RouterConfig) -> crate::errors::Result<()> {
        self.config = config;
        Ok(())
    }

    async fn route_package(&self, package: &crate::events::Package) -> RouteResult {
        // Start with default state
        let mut state = RouteState::new();

        // If auto-initialization is enabled, mark as initialized
        if self.config.auto_initialize {
            state = state.with_initialized();
        }

        // If auto-routing is disabled, return as-is
        if !self.config.auto_route {
            return RouteResult::success(state);
        }

        // Extract channel type from events
        let channel_type = self.extract_channel_type(package);

        if let Some(ct) = channel_type {
            // Determine target based on channel type
            let target = self.determine_target(&ct);
            let target_for_log = target.clone(); // Clone for logging
            state = state.with_adapter_target(target).with_channel_type(&ct);

            // Log routing decision
            let message = format!(
                "Routed package {} to {:?} (channel: {})",
                package.package_id, target_for_log, ct
            );
            let context = LogContext::new().with_component("Router");
            self.logger.log(LogLevel::Debug, &message, &context);

            RouteResult::success(state)
        } else {
            // No channel information found, use default
            if let Some(adapter) = &self.config.default_adapter {
                state = state.with_adapter_target(RouteTarget::Adapter(adapter.clone()));
            }

            let message = format!(
                "No channel info for package {}, using default route: {:?}",
                package.package_id, state.adapter_target
            );
            let context = LogContext::new().with_component("Router");
            self.logger.log(LogLevel::Warn, &message, &context);

            RouteResult::success(state)
        }
    }
}

// Extension trait to extract channel_id from EventEnum
trait ChannelIdExt {
    fn channel_id(&self) -> Option<String>;
}

impl ChannelIdExt for EventEnum {
    fn channel_id(&self) -> Option<String> {
        // Extract from metadata extra field
        match self {
            EventEnum::Message(evt) => {
                if let Ok(extra) = serde_json::from_value::<serde_json::Value>(serde_json::json!(evt)) {
                    if let Some(obj) = extra.as_object() {
                        if let Some(channel_id) = obj.get("channel_id").and_then(|v| v.as_str()) {
                            return Some(channel_id.to_string());
                        }
                    }
                }
                None
            }
            EventEnum::Notice(evt) => {
                if let Ok(extra) = serde_json::from_value::<serde_json::Value>(serde_json::json!(evt)) {
                    if let Some(obj) = extra.as_object() {
                        if let Some(channel_id) = obj.get("channel_id").and_then(|v| v.as_str()) {
                            return Some(channel_id.to_string());
                        }
                    }
                }
                None
            }
            EventEnum::Request(evt) => {
                if let Ok(extra) = serde_json::from_value::<serde_json::Value>(serde_json::json!(evt)) {
                    if let Some(obj) = extra.as_object() {
                        if let Some(channel_id) = obj.get("channel_id").and_then(|v| v.as_str()) {
                            return Some(channel_id.to_string());
                        }
                    }
                }
                None
            }
            EventEnum::Meta(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::*;
    use crate::logging::StructuredLogger;
    use crate::logging::formatters::JsonFormatter;
    use crate::logging::writers::ConsoleWriter;

    fn create_test_logger() -> Arc<dyn crate::logging::Logger> {
        let formatter = Arc::new(JsonFormatter::new());
        let writer = Arc::new(ConsoleWriter::new());
        Arc::new(StructuredLogger::new(formatter, writer))
    }

    fn create_test_router() -> StandardRouter {
        let config = RouterConfig::new()
            .with_group_adapter("group_adapter")
            .with_private_adapter("private_adapter");
        StandardRouter::with_config(config, create_test_logger())
    }

    #[test]
    fn test_router_creation() {
        let logger = create_test_logger();
        let router = StandardRouter::new(logger);
        assert_eq!(router.router_id(), "standard_router");
        assert!(router.is_enabled());
    }

    #[test]
    fn test_router_with_id() {
        let logger = create_test_logger();
        let router = StandardRouter::with_id("custom_router".to_string(), logger);
        assert_eq!(router.router_id(), "custom_router");
    }

    #[test]
    fn test_router_set_config() {
        let logger = create_test_logger();
        let mut router = StandardRouter::new(logger);

        let new_config = RouterConfig::new()
            .with_default_adapter("new_default")
            .with_auto_initialize(false);
        assert!(router.set_config(new_config).is_ok());
        assert!(!router.config().auto_initialize);
    }

    #[test]
    fn test_extract_channel_type_group() {
        let router = create_test_router();

        let group_event = EventEnum::Message(message::MessageEvent::Text {
            text: "Test".to_string(),
            metadata: traits::EventMetadata::new("message")
                .with_user_id("user1")
                .with_group_id("group123"),
        });

        let group = Group::new("test_group").with_event(group_event);
        let block = Block::new(BlockType::Message).with_group(group);
        let package = Package::new().with_block(block);

        let channel_type = router.extract_channel_type(&package);
        assert_eq!(channel_type, Some("group:group123".to_string()));
    }

    #[test]
    fn test_extract_channel_type_private() {
        let router = create_test_router();

        let private_event = EventEnum::Message(message::MessageEvent::Text {
            text: "Test".to_string(),
            metadata: traits::EventMetadata::new("message")
                .with_user_id("user1"),
        });

        let group = Group::new("test_group").with_event(private_event);
        let block = Block::new(BlockType::Message).with_group(group);
        let package = Package::new().with_block(block);

        let channel_type = router.extract_channel_type(&package);
        assert_eq!(channel_type, Some("private:user1".to_string()));
    }

    #[test]
    fn test_determine_target_group() {
        let router = create_test_router();
        let target = router.determine_target("group:123");
        assert_eq!(target, RouteTarget::Adapter("group_adapter".to_string()));
    }

    #[test]
    fn test_determine_target_private() {
        let router = create_test_router();
        let target = router.determine_target("private:123");
        assert_eq!(target, RouteTarget::Adapter("private_adapter".to_string()));
    }

    #[test]
    fn test_determine_target_default() {
        let logger = create_test_logger();
        let config = RouterConfig::new().with_default_adapter("default_adapter");
        let router = StandardRouter::with_config(config, logger);

        let target = router.determine_target("unknown:123");
        assert_eq!(target, RouteTarget::Adapter("default_adapter".to_string()));
    }

    #[test]
    fn test_determine_target_none() {
        let logger = create_test_logger();
        let config = RouterConfig::new();
        let router = StandardRouter::with_config(config, logger);

        let target = router.determine_target("unknown:123");
        assert_eq!(target, RouteTarget::None);
    }

    #[tokio::test]
    async fn test_route_package_group() {
        let router = create_test_router();

        let group_event = EventEnum::Message(message::MessageEvent::Text {
            text: "Test".to_string(),
            metadata: traits::EventMetadata::new("message")
                .with_user_id("user1")
                .with_group_id("group123"),
        });

        let group = Group::new("test_group").with_event(group_event);
        let block = Block::new(BlockType::Message).with_group(group);
        let package = Package::new().with_block(block);

        let result = router.route_package(&package).await;
        assert!(result.success);
        assert!(result.state.initialized);
        assert_eq!(
            result.state.adapter_target,
            RouteTarget::Adapter("group_adapter".to_string())
        );
        assert_eq!(result.state.channel_type, Some("group:group123".to_string()));
    }

    #[tokio::test]
    async fn test_route_package_no_auto_route() {
        let logger = create_test_logger();
        let config = RouterConfig::new().with_auto_route(false);
        let router = StandardRouter::with_config(config, logger);

        let event = EventEnum::Message(message::MessageEvent::Text {
            text: "Test".to_string(),
            metadata: traits::EventMetadata::new("message")
                .with_user_id("user1")
                .with_group_id("group123"),
        });

        let group = Group::new("test_group").with_event(event);
        let block = Block::new(BlockType::Message).with_group(group);
        let package = Package::new().with_block(block);

        let result = router.route_package(&package).await;
        assert!(result.success);
        assert!(result.state.initialized);
        assert_eq!(result.state.adapter_target, RouteTarget::None);
        assert!(result.state.channel_type.is_none());
    }

    #[tokio::test]
    async fn test_route_batch() {
        let router = create_test_router();

        let event1 = EventEnum::Message(message::MessageEvent::Text {
            text: "Test".to_string(),
            metadata: traits::EventMetadata::new("message")
                .with_user_id("user1")
                .with_group_id("group123"),
        });

        let event2 = EventEnum::Message(message::MessageEvent::Text {
            text: "Test".to_string(),
            metadata: traits::EventMetadata::new("message")
                .with_user_id("user2"),
        });

        let group = Group::new("test_group").with_event(event1).with_event(event2);
        let block = Block::new(BlockType::Message).with_group(group);
        let package = Package::new().with_block(block);

        let results = router.route_batch(&[package.clone()]).await;
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
    }
}
