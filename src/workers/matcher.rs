//! Matcher system for Worker package matching (v2.0 with four-dimensional support)
//!
//! Matchers determine if a Worker should process a given Package based on:
//! - Domain tags (material type: text, image, audio, etc.)
//! - Motif tags (structural feature: command, mention, url, etc.)
//! - State tags (functional state: intent_weather, spam_suspected, etc.)
//! - Context tags (contextual info: group, night_mode, etc.)
//! - Payload types and content
//! - Trace history

use crate::events::{Package, TargetSite};
use regex::Regex;

/// Matcher - determines if a Worker should process a Package
#[derive(Debug, Clone)]
pub enum Matcher {
    /// Exact match on a specific target site
    Exact(TargetSite),
    
    /// Match if package has specific domain tag
    HasDomain(String),
    
    /// Match if package has specific motif tag
    HasMotif(String),
    
    /// Match if package has specific state tag
    HasState(String),
    
    /// Match if package has specific context tag
    HasContext(String),
    
    /// Match if package has any of these domain tags
    HasAnyDomain(Vec<String>),
    
    /// Match if package has any of these motif tags
    HasAnyMotif(Vec<String>),
    
    /// Match if package has any of these state tags
    HasAnyState(Vec<String>),
    
    /// Match if package has any of these context tags
    HasAnyContext(Vec<String>),
    
    /// Match if package has a specific payload type
    HasPayloadType(String),
    
    /// Match if payload text contains pattern (regex)
    PayloadTextContains(String),
    
    /// Match if payload text starts with prefix
    PayloadTextStartsWith(String),
    
    /// Match if payload text ends with suffix
    PayloadTextEndsWith(String),
    
    /// Match if payload text matches regex pattern
    PayloadTextMatches(Regex),
    
    /// Match if package has been processed by specific worker
    HasTrace(Vec<String>),
    
    /// All matchers must be satisfied (AND)
    AllOf(Vec<Matcher>),
    
    /// Any matcher must be satisfied (OR)
    AnyOf(Vec<Matcher>),
    
    /// Matcher must not be satisfied (NOT)
    Not(Box<Matcher>),
    
    /// Match everything (wildcard)
    Wildcard,
}

impl Matcher {
    /// Check if this matcher matches the given package
    pub fn matches(&self, package: &Package) -> bool {
        match self {
            Matcher::Exact(site) => {
                package.target_sites.iter().any(|t| t == site)
            }
            
            Matcher::HasDomain(tag) => {
                package.target_sites.iter().any(|t| match t {
                    TargetSite::Domain(dt) => dt.tag_string().eq_ignore_ascii_case(tag),
                    _ => false,
                })
            }
            
            Matcher::HasMotif(tag) => {
                package.target_sites.iter().any(|t| match t {
                    TargetSite::Motif(mt) => mt.tag_string().eq_ignore_ascii_case(tag),
                    _ => false,
                })
            }
            
            Matcher::HasState(tag) => {
                package.target_sites.iter().any(|t| match t {
                    TargetSite::State(st) => st.tag_string().eq_ignore_ascii_case(tag),
                    _ => false,
                })
            }
            
            Matcher::HasContext(tag) => {
                package.target_sites.iter().any(|t| match t {
                    TargetSite::Context(ct) => ct.tag_string().eq_ignore_ascii_case(tag),
                    _ => false,
                })
            }
            
            Matcher::HasAnyDomain(tags) => {
                package.target_sites.iter().any(|t| match t {
                    TargetSite::Domain(dt) => {
                        tags.iter().any(|tag| dt.tag_string().eq_ignore_ascii_case(tag))
                    }
                    _ => false,
                })
            }
            
            Matcher::HasAnyMotif(tags) => {
                package.target_sites.iter().any(|t| match t {
                    TargetSite::Motif(mt) => {
                        tags.iter().any(|tag| mt.tag_string().eq_ignore_ascii_case(tag))
                    }
                    _ => false,
                })
            }
            
            Matcher::HasAnyState(tags) => {
                package.target_sites.iter().any(|t| match t {
                    TargetSite::State(st) => {
                        tags.iter().any(|tag| st.tag_string().eq_ignore_ascii_case(tag))
                    }
                    _ => false,
                })
            }
            
            Matcher::HasAnyContext(tags) => {
                package.target_sites.iter().any(|t| match t {
                    TargetSite::Context(ct) => {
                        tags.iter().any(|tag| ct.tag_string().eq_ignore_ascii_case(tag))
                    }
                    _ => false,
                })
            }
            
            Matcher::HasPayloadType(type_name) => {
                package.has_payload_type(type_name)
            }
            
            Matcher::PayloadTextContains(pattern) => {
                if let Some(text_payload) = package.get_payload::<crate::events::payloads::TextPayload>() {
                    text_payload.content.contains(pattern)
                } else {
                    false
                }
            }
            
            Matcher::PayloadTextStartsWith(prefix) => {
                if let Some(text_payload) = package.get_payload::<crate::events::payloads::TextPayload>() {
                    text_payload.content.starts_with(prefix)
                } else {
                    false
                }
            }
            
            Matcher::PayloadTextEndsWith(suffix) => {
                if let Some(text_payload) = package.get_payload::<crate::events::payloads::TextPayload>() {
                    text_payload.content.ends_with(suffix)
                } else {
                    false
                }
            }
            
            Matcher::PayloadTextMatches(regex) => {
                if let Some(text_payload) = package.get_payload::<crate::events::payloads::TextPayload>() {
                    regex.is_match(&text_payload.content)
                } else {
                    false
                }
            }
            
            Matcher::HasTrace(worker_names) => {
                worker_names.iter().any(|name| package.trace.contains(name))
            }
            
            Matcher::AllOf(matchers) => {
                matchers.iter().all(|m| m.matches(package))
            }
            
            Matcher::AnyOf(matchers) => {
                matchers.iter().any(|m| m.matches(package))
            }
            
            Matcher::Not(matcher) => {
                !matcher.matches(package)
            }
            
            Matcher::Wildcard => true,
        }
    }
    
    /// Create a HasDomain matcher
    pub fn has_domain(tag: &str) -> Self {
        Self::HasDomain(tag.to_string())
    }
    
    /// Create a HasMotif matcher
    pub fn has_motif(tag: &str) -> Self {
        Self::HasMotif(tag.to_string())
    }
    
    /// Create a HasState matcher
    pub fn has_state(tag: &str) -> Self {
        Self::HasState(tag.to_string())
    }
    
    /// Create a HasContext matcher
    pub fn has_context(tag: &str) -> Self {
        Self::HasContext(tag.to_string())
    }
    
    /// Create a HasAnyDomain matcher
    pub fn has_any_domain(tags: Vec<&str>) -> Self {
        Self::HasAnyDomain(tags.into_iter().map(|s| s.to_string()).collect())
    }
    
    /// Create a HasAnyMotif matcher
    pub fn has_any_motif(tags: Vec<&str>) -> Self {
        Self::HasAnyMotif(tags.into_iter().map(|s| s.to_string()).collect())
    }
    
    /// Create a HasAnyState matcher
    pub fn has_any_state(tags: Vec<&str>) -> Self {
        Self::HasAnyState(tags.into_iter().map(|s| s.to_string()).collect())
    }
    
    /// Create a HasAnyContext matcher
    pub fn has_any_context(tags: Vec<&str>) -> Self {
        Self::HasAnyContext(tags.into_iter().map(|s| s.to_string()).collect())
    }
    
    /// Create a HasPayloadType matcher
    pub fn has_payload_type(type_name: &str) -> Self {
        Self::HasPayloadType(type_name.to_string())
    }
    
    /// Create a PayloadTextContains matcher
    pub fn text_contains(pattern: &str) -> Self {
        Self::PayloadTextContains(pattern.to_string())
    }
    
    /// Create a PayloadTextStartsWith matcher
    pub fn text_starts_with(prefix: &str) -> Self {
        Self::PayloadTextStartsWith(prefix.to_string())
    }
    
    /// Create a PayloadTextMatches matcher from pattern
    pub fn text_matches(pattern: &str) -> Result<Self, regex::Error> {
        Regex::new(pattern).map(Self::PayloadTextMatches)
    }
    
    /// Combine multiple matchers with AND
    pub fn all_of(matchers: Vec<Matcher>) -> Self {
        Self::AllOf(matchers)
    }
    
    /// Combine multiple matchers with OR
    pub fn any_of(matchers: Vec<Matcher>) -> Self {
        Self::AnyOf(matchers)
    }
    
    /// Negate a matcher
    pub fn not(matcher: Matcher) -> Self {
        Self::Not(Box::new(matcher))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::payloads::TextPayload;

    #[test]
    fn test_exact_matcher() {
        let site = TargetSite::state_custom("test_tag");
        let package = Package::new()
            .with_target_site(site.clone());
        
        let matcher = Matcher::Exact(site);
        assert!(matcher.matches(&package));
    }

    #[test]
    fn test_has_domain_matcher() {
        let package = Package::new()
            .with_target_site(TargetSite::domain_text())
            .with_target_site(TargetSite::motif_command());
        
        let matcher = Matcher::has_motif("Command");
        assert!(matcher.matches(&package));
    }

    #[test]
    fn test_has_payload_type_matcher() {
        let package = Package::new()
            .with_payload(TextPayload::new("Hello world"));
        
        let matcher = Matcher::has_payload_type("text");
        assert!(matcher.matches(&package));
        
        let wrong_matcher = Matcher::has_payload_type("blob");
        assert!(!wrong_matcher.matches(&package));
    }

    #[test]
    fn test_text_contains_matcher() {
        let package = Package::new()
            .with_payload(TextPayload::new("Hello world"));
        
        let matcher = Matcher::text_contains("world");
        assert!(matcher.matches(&package));
    }

    #[test]
    fn test_text_starts_with_matcher() {
        let package = Package::new()
            .with_payload(TextPayload::new("/ping"));
        
        let matcher = Matcher::text_starts_with("/");
        assert!(matcher.matches(&package));
    }

    #[test]
    fn test_text_matches_matcher() {
        let package = Package::new()
            .with_payload(TextPayload::new("ping 123"));
        
        let matcher = Matcher::text_matches(r"ping \d+").unwrap();
        assert!(matcher.matches(&package));
    }

    #[test]
    fn test_all_of_matcher() {
        let package = Package::new()
            .with_target_site(TargetSite::domain_text())
            .with_payload(TextPayload::new("/ping"));
        
        let matcher = Matcher::all_of(vec![
            Matcher::has_domain("Text"),
            Matcher::text_starts_with("/"),
        ]);
        
        assert!(matcher.matches(&package));
    }

    #[test]
    fn test_any_of_matcher() {
        let package1 = Package::new()
            .with_target_site(TargetSite::domain_image());
        
        let package2 = Package::new()
            .with_target_site(TargetSite::domain_text());
        
        let matcher = Matcher::any_of(vec![
            Matcher::has_domain("Image"),
            Matcher::has_domain("Text"),
        ]);
        
        assert!(matcher.matches(&package1));
        assert!(matcher.matches(&package2));
    }

    #[test]
    fn test_not_matcher() {
        let package = Package::new()
            .with_payload(TextPayload::new("/ping"));
        
        let matcher = Matcher::not(Matcher::text_starts_with("/"));
        assert!(!matcher.matches(&package));
    }

    #[test]
    fn test_wildcard_matcher() {
        let package = Package::new();
        let matcher = Matcher::Wildcard;
        assert!(matcher.matches(&package));
    }

    #[test]
    fn test_has_trace_matcher() {
        let package = Package::new()
            .trace_worker("worker1")
            .trace_worker("worker2");
        
        let matcher = Matcher::HasTrace(vec!["worker1".to_string()]);
        assert!(matcher.matches(&package));
    }
}
