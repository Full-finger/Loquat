//! ConversionWorker - configuration-driven tag transformation (v2.0)
//!
//! ConversionWorker allows tag transformation through YAML configuration
//! without writing code. It's the "meta worker" from the design document.

use crate::events::Package;
use crate::workers::{Matcher, Worker, WorkerResult, WorkerType};
use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Conversion rule for tag transformation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionRule {
    /// Rule name (for debugging)
    pub name: String,
    
    /// Matcher conditions (when to apply this rule)
    pub conditions: ConversionConditions,
    
    /// Actions to take (what tags to add/remove)
    pub actions: ConversionActions,
}

/// Conditions for applying conversion rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionConditions {
    /// Match if package has all these tags
    #[serde(default)]
    pub has_tags: Vec<String>,
    
    /// Match if package has this payload type
    #[serde(default)]
    pub has_payload_type: Option<String>,
    
    /// Match if payload text contains pattern
    #[serde(default)]
    pub text_contains: Option<String>,
    
    /// Match if payload text starts with prefix
    #[serde(default)]
    pub text_starts_with: Option<String>,
    
    /// Match if payload text ends with suffix
    #[serde(default)]
    pub text_ends_with: Option<String>,
    
    /// Match if payload text matches regex
    #[serde(default)]
    pub text_matches: Option<String>,
    
    /// Match if package has been processed by these workers
    #[serde(default)]
    pub has_trace: Vec<String>,
}

impl ConversionConditions {
    /// Check if all conditions are satisfied
    pub fn matches(&self, package: &Package) -> bool {
        // Check tags
        if !self.has_tags.is_empty() {
            let package_tags: Vec<String> = package.target_sites
                .iter()
                .filter_map(|t| match &t.site_type {
                    crate::events::SiteType::Tag(tag) => Some(tag.clone()),
                    _ => None,
                })
                .collect();
            
            if !self.has_tags.iter().all(|tag| package_tags.contains(tag)) {
                return false;
            }
        }
        
        // Check payload type
        if let Some(expected_type) = &self.has_payload_type {
            if !package.has_payload_type(expected_type) {
                return false;
            }
        }
        
        // Check text conditions
        if let Some(text_payload) = package.get_payload::<crate::events::payloads::TextPayload>() {
            if let Some(pattern) = &self.text_contains {
                if !text_payload.content.contains(pattern) {
                    return false;
                }
            }
            
            if let Some(prefix) = &self.text_starts_with {
                if !text_payload.content.starts_with(prefix) {
                    return false;
                }
            }
            
            if let Some(suffix) = &self.text_ends_with {
                if !text_payload.content.ends_with(suffix) {
                    return false;
                }
            }
            
            if let Some(pattern) = &self.text_matches {
                if let Ok(regex) = Regex::new(pattern) {
                    if !regex.is_match(&text_payload.content) {
                        return false;
                    }
                }
            }
        } else if self.text_contains.is_some() || self.text_starts_with.is_some() 
            || self.text_ends_with.is_some() || self.text_matches.is_some() {
            // Text conditions require TextPayload
            return false;
        }
        
        // Check trace
        if !self.has_trace.is_empty() {
            if !self.has_trace.iter().any(|worker| package.trace.contains(worker)) {
                return false;
            }
        }
        
        true
    }
}

/// Actions to take when conditions are met
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionActions {
    /// Tags to add to package
    #[serde(default)]
    pub add_tags: Vec<String>,
    
    /// Tags to remove from package
    #[serde(default)]
    pub remove_tags: Vec<String>,
    
    /// Set new payload (JSON string representation)
    #[serde(default)]
    pub set_payload: Option<serde_json::Value>,
}

/// Configuration file for ConversionWorker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionConfig {
    /// List of conversion rules
    pub rules: Vec<ConversionRule>,
}

impl ConversionConfig {
    /// Load from YAML string
    pub fn from_yaml(yaml_str: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml_str)
    }
    
    /// Load from YAML file
    pub fn from_yaml_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Self::from_yaml(&content).map_err(|e| e.into())
    }
    
    /// Convert to YAML string
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

/// ConversionWorker - applies conversion rules to packages
#[derive(Debug, Clone)]
pub struct ConversionWorker {
    /// Worker name
    name: String,
    
    /// Conversion rules to apply
    rules: Arc<Vec<ConversionRule>>,
}

impl ConversionWorker {
    /// Create a new ConversionWorker
    pub fn new(name: &str, config: ConversionConfig) -> Self {
        Self {
            name: name.to_string(),
            rules: Arc::new(config.rules),
        }
    }
    
    /// Create from YAML config
    pub fn from_yaml(name: &str, yaml_str: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config = ConversionConfig::from_yaml(yaml_str)?;
        Ok(Self::new(name, config))
    }
    
    /// Create from YAML file
    pub fn from_yaml_file(name: &str, path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config = ConversionConfig::from_yaml_file(path)?;
        Ok(Self::new(name, config))
    }
}

#[async_trait]
impl Worker for ConversionWorker {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn worker_type(&self) -> WorkerType {
        WorkerType::Process
    }
    
    fn matches_package(&self, package: &Package) -> bool {
        // ConversionWorker matches all packages
        // Internal rules filter which conversions to apply
        true
    }
    
    async fn handle_batch(&self, mut packages: Vec<Package>) -> WorkerResult {
        for package in &mut packages {
            for rule in self.rules.iter() {
                if rule.conditions.matches(package) {
                    // Add tags
                    for tag in &rule.actions.add_tags {
                        package.target_sites.push(crate::events::TargetSite::tag(tag));
                    }
                    
                    // Remove tags
                    for tag in &rule.actions.remove_tags {
                        package.target_sites.retain(|t| match &t.site_type {
                            crate::events::SiteType::Tag(existing_tag) => existing_tag != tag,
                            _ => true,
                        });
                    }
                    
                    // Set payload if specified
                    if let Some(payload_json) = &rule.actions.set_payload {
                        // For now, just log the payload change
                        // In a full implementation, this would convert JSON to appropriate payload type
                        tracing::debug!("ConversionWorker: would set payload to {:?}", payload_json);
                    }
                    
                    tracing::debug!("ConversionWorker: applied rule '{}' to package '{}'", 
                        rule.name, package.package_id);
                }
            }
        }
        
        WorkerResult::Modify(packages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::payloads::TextPayload;
    use crate::events::TargetSite;

    #[test]
    fn test_conversion_conditions_tags() {
        let conditions = ConversionConditions {
            has_tags: vec!["text".to_string(), "command".to_string()],
            has_payload_type: None,
            text_contains: None,
            text_starts_with: None,
            text_ends_with: None,
            text_matches: None,
            has_trace: Vec::new(),
        };
        
        let matching_package = Package::new()
            .with_target_site(TargetSite::tag("text"))
            .with_target_site(TargetSite::tag("command"));
        
        let non_matching_package = Package::new()
            .with_target_site(TargetSite::tag("image"));
        
        assert!(conditions.matches(&matching_package));
        assert!(!conditions.matches(&non_matching_package));
    }

    #[test]
    fn test_conversion_conditions_text() {
        let conditions = ConversionConditions {
            has_tags: Vec::new(),
            has_payload_type: None,
            text_contains: Some("world".to_string()),
            text_starts_with: None,
            text_ends_with: None,
            text_matches: None,
            has_trace: Vec::new(),
        };
        
        let matching_package = Package::new()
            .with_payload(TextPayload::new("Hello world"));
        
        let non_matching_package = Package::new()
            .with_payload(TextPayload::new("Hello there"));
        
        assert!(conditions.matches(&matching_package));
        assert!(!conditions.matches(&non_matching_package));
    }

    #[test]
    fn test_conversion_config_yaml() {
        let yaml = r#"
rules:
  - name: "detect_command"
    conditions:
      has_tags: ["text"]
      text_starts_with: "/"
    actions:
      add_tags: ["command"]
"#;
        
        let config = ConversionConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.rules.len(), 1);
        assert_eq!(config.rules[0].name, "detect_command");
    }

    #[test]
    fn test_conversion_worker() {
        let yaml = r#"
rules:
  - name: "add_command_tag"
    conditions:
      text_starts_with: "/"
    actions:
      add_tags: ["command"]
"#;
        
        let worker = ConversionWorker::from_yaml("test_worker", yaml).unwrap();
        
        let mut package = Package::new()
            .with_payload(TextPayload::new("/ping"));
        
        // Simulate worker processing
        let result = tokio_test::block_on(worker.handle_batch(vec![package]));
        
        if let WorkerResult::Modify(packages) = result {
            assert_eq!(packages.len(), 1);
            assert!(packages[0].target_sites.iter()
                .any(|t| matches!(&t.site_type, crate::events::SiteType::Tag(tag) if tag == "command")));
        } else {
            panic!("Expected Modify result");
        }
    }
}
