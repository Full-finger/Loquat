//! CommandParser Worker - parses text commands (v2.0)
//!
//! CommandParser detects and parses text commands like "/ping hello"
//! and adds appropriate tags to the package.

use crate::events::Package;
use crate::events::payloads::TextPayload;
use crate::events::TargetSite;
use crate::workers::{Matcher, Worker, WorkerResult, WorkerType};
use async_trait::async_trait;

/// CommandParser - parses text commands
#[derive(Debug, Clone)]
pub struct CommandParser {
    /// Command prefix (e.g., "/")
    command_prefix: String,
    
    /// Matcher for this parser
    matcher: Matcher,
    
    /// Whether to add specific command tag (e.g., "command:ping")
    add_command_tag: bool,
}

impl CommandParser {
    /// Create a new CommandParser with default prefix "/"
    pub fn new() -> Self {
        Self::with_prefix("/")
    }
    
    /// Create a new CommandParser with custom prefix
    pub fn with_prefix(prefix: &str) -> Self {
        let matcher = Matcher::all_of(vec![
            Matcher::has_tag("text"),
            Matcher::text_starts_with(prefix),
        ]);
        
        Self {
            command_prefix: prefix.to_string(),
            matcher,
            add_command_tag: true,
        }
    }
    
    /// Create a CommandParser with custom options
    pub fn with_options(prefix: &str, add_command_tag: bool) -> Self {
        let matcher = Matcher::all_of(vec![
            Matcher::has_tag("text"),
            Matcher::text_starts_with(prefix),
        ]);
        
        Self {
            command_prefix: prefix.to_string(),
            matcher,
            add_command_tag,
        }
    }
    
    /// Parse command text and extract command name and arguments
    /// Example: "/ping hello world" -> ("ping", vec!["hello", "world"])
    pub fn parse_command(text: &str, prefix: &str) -> Option<(String, Vec<String>)> {
        if !text.starts_with(prefix) {
            return None;
        }
        
        let text = &text[prefix.len()..];
        let text = text.trim();
        
        if text.is_empty() {
            return None;
        }
        
        // Split into command and arguments
        let parts: Vec<&str> = text.split_whitespace().collect();
        
        if parts.is_empty() {
            return None;
        }
        
        let command = parts[0].to_string();
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
        
        Some((command, args))
    }
}

impl Default for CommandParser {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Worker for CommandParser {
    fn name(&self) -> &str {
        "command_parser"
    }
    
    fn worker_type(&self) -> WorkerType {
        WorkerType::Process
    }
    
    fn matcher(&self) -> &Matcher {
        &self.matcher
    }
    
    async fn handle_batch(&self, mut packages: Vec<Package>) -> WorkerResult {
        for package in &mut packages {
            // Get text payload
            let text_payload = if let Some(payload) = package.get_payload::<TextPayload>() {
                payload.clone()
            } else {
                continue;
            };
            
            // Parse command
            if let Some((command, _args)) = Self::parse_command(&text_payload.content, &self.command_prefix) {
                // Add "command" tag
                package.target_sites.push(TargetSite::tag("command"));
                
                // Add specific command tag if enabled
                if self.add_command_tag {
                    let command_tag = format!("command:{}", command);
                    package.target_sites.push(TargetSite::tag(&command_tag));
                }
                
                // Add command to extra metadata
                if let Ok(extra) = serde_json::to_value(&CommandData {
                    command: command.clone(),
                    prefix: self.command_prefix.clone(),
                }) {
                    package.extra["command"] = extra;
                }
                
                tracing::debug!("CommandParser: parsed command '{}' from package '{}'", 
                    command, package.package_id);
            }
        }
        
        WorkerResult::Modify(packages)
    }
}

/// Command data stored in package extra metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CommandData {
    command: String,
    prefix: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command() {
        let result = CommandParser::parse_command("/ping hello", "/");
        assert_eq!(result, Some(("ping".to_string(), vec!["hello".to_string()])));
        
        let result = CommandParser::parse_command("/ping hello world", "/");
        assert_eq!(result, Some(("ping".to_string(), vec!["hello".to_string(), "world".to_string()])));
        
        let result = CommandParser::parse_command("/ping", "/");
        assert_eq!(result, Some(("ping".to_string(), vec![])));
        
        let result = CommandParser::parse_command("ping hello", "/");
        assert_eq!(result, None);
        
        let result = CommandParser::parse_command("/ ping hello", "/");
        assert_eq!(result, Some(("ping".to_string(), vec!["hello".to_string()])));
    }

    #[test]
    fn test_command_parser_creation() {
        let parser = CommandParser::new();
        assert_eq!(parser.command_prefix, "/");
        assert!(parser.add_command_tag);
        
        let parser = CommandParser::with_prefix("!");
        assert_eq!(parser.command_prefix, "!");
        assert!(parser.add_command_tag);
        
        let parser = CommandParser::with_options(".", false);
        assert_eq!(parser.command_prefix, ".");
        assert!(!parser.add_command_tag);
    }

    #[test]
    fn test_command_parser_matcher() {
        let parser = CommandParser::new();
        
        let package = Package::new()
            .with_payload(TextPayload::new("/ping"))
            .with_target_site(TargetSite::tag("text"));
        
        assert!(parser.matches_package(&package));
        
        let package = Package::new()
            .with_payload(TextPayload::new("ping"))
            .with_target_site(TargetSite::tag("text"));
        
        assert!(!parser.matches_package(&package));
    }

    #[tokio::test]
    async fn test_command_parser_handle() {
        let parser = CommandParser::new();
        
        let mut package = Package::new()
            .with_payload(TextPayload::new("/ping hello"))
            .with_target_site(TargetSite::tag("text"));
        
        let result = parser.handle_batch(vec![package]).await;
        
        if let WorkerResult::Modify(packages) = result {
            assert_eq!(packages.len(), 1);
            let pkg = &packages[0];
            
            // Check tags
            assert!(pkg.target_sites.iter().any(|t| matches!(&t.site_type, 
                crate::events::SiteType::Tag(tag) if tag == "command")));
            assert!(pkg.target_sites.iter().any(|t| matches!(&t.site_type, 
                crate::events::SiteType::Tag(tag) if tag == "command:ping")));
            
            // Check extra metadata
            assert!(pkg.extra.get("command").is_some());
        } else {
            panic!("Expected Modify result");
        }
    }

    #[tokio::test]
    async fn test_command_parser_without_command_tag() {
        let parser = CommandParser::with_options("/", false);
        
        let mut package = Package::new()
            .with_payload(TextPayload::new("/ping"))
            .with_target_site(TargetSite::tag("text"));
        
        let result = parser.handle_batch(vec![package]).await;
        
        if let WorkerResult::Modify(packages) = result {
            assert_eq!(packages.len(), 1);
            let pkg = &packages[0];
            
            // Should have "command" tag
            assert!(pkg.target_sites.iter().any(|t| matches!(&t.site_type, 
                crate::events::SiteType::Tag(tag) if tag == "command")));
            
            // Should NOT have "command:ping" tag
            assert!(!pkg.target_sites.iter().any(|t| matches!(&t.site_type, 
                crate::events::SiteType::Tag(tag) if tag == "command:ping")));
        } else {
            panic!("Expected Modify result");
        }
    }

    #[test]
    fn test_parse_empty_command() {
        let result = CommandParser::parse_command("/", "/");
        assert_eq!(result, None);
        
        let result = CommandParser::parse_command("/  ", "/");
        assert_eq!(result, None);
    }
}
