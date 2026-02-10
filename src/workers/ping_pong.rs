//! PingPong Worker - responds to /ping commands (v2.0)
//!
//! PingPongWorker demonstrates the complete ping-pong flow:
//! Input: /ping command
//! Output: pong response

use crate::events::Package;
use crate::events::payloads::TextPayload;
use crate::events::TargetSite;
use crate::workers::{Matcher, Worker, WorkerResult, WorkerType};
use async_trait::async_trait;

/// PingPongWorker - responds to ping commands
#[derive(Debug, Clone)]
pub struct PingPongWorker {
    /// Response message
    response: String,
    
    /// Matcher for this worker
    matcher: Matcher,
}

impl PingPongWorker {
    /// Create a new PingPongWorker with default response "pong"
    pub fn new() -> Self {
        Self::with_response("pong")
    }
    
    /// Create a new PingPongWorker with custom response
    pub fn with_response(response: &str) -> Self {
        let matcher = Matcher::all_of(vec![
            Matcher::has_tag("command"),
            Matcher::has_tag("command:ping"),
        ]);
        
        Self {
            response: response.to_string(),
            matcher,
        }
    }
}

impl Default for PingPongWorker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Worker for PingPongWorker {
    fn name(&self) -> &str {
        "ping_pong"
    }
    
    fn worker_type(&self) -> WorkerType {
        WorkerType::Output
    }
    
    fn matcher(&self) -> &Matcher {
        &self.matcher
    }
    
    async fn handle_batch(&self, mut packages: Vec<Package>) -> WorkerResult {
        for package in &mut packages {
            // Create response payload
            let response_payload = TextPayload::new(&self.response);
            
            // Set the response as the new payload
            package.payload = Some(Box::new(response_payload));
            package.payload_type = Some("TextPayload".to_string());
            
            // Add response tag
            package.target_sites.push(TargetSite::tag("response"));
            
            // Trace this worker
            package.trace.push(self.name().to_string());
            
            tracing::info!("PingPongWorker: responded to package '{}' with '{}'", 
                package.package_id, self.response);
        }
        
        WorkerResult::Modify(packages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_pong_creation() {
        let worker = PingPongWorker::new();
        assert_eq!(worker.response, "pong");
        
        let worker = PingPongWorker::with_response("PONG!");
        assert_eq!(worker.response, "PONG!");
    }

    #[test]
    fn test_ping_pong_matcher() {
        let worker = PingPongWorker::new();
        
        let package = Package::new()
            .with_target_site(TargetSite::tag("command"))
            .with_target_site(TargetSite::tag("command:ping"));
        
        assert!(worker.matches_package(&package));
        
        let package = Package::new()
            .with_target_site(TargetSite::tag("command"))
            .with_target_site(TargetSite::tag("command:help"));
        
        assert!(!worker.matches_package(&package));
    }

    #[tokio::test]
    async fn test_ping_pong_handle() {
        let worker = PingPongWorker::new();
        
        let mut package = Package::new()
            .with_payload(TextPayload::new("/ping"))
            .with_target_site(TargetSite::tag("command"))
            .with_target_site(TargetSite::tag("command:ping"));
        
        let result = worker.handle_batch(vec![package]).await;
        
        if let WorkerResult::Modify(packages) = result {
            assert_eq!(packages.len(), 1);
            let pkg = &packages[0];
            
            // Check payload
            if let Some(payload) = pkg.get_payload::<TextPayload>() {
                assert_eq!(payload.content, "pong");
            } else {
                panic!("Expected TextPayload");
            }
            
            // Check tags
            assert!(pkg.target_sites.iter().any(|t| matches!(&t.site_type, 
                crate::events::SiteType::Tag(tag) if tag == "response")));
            
            // Check trace
            assert!(pkg.trace.contains(&"ping_pong".to_string()));
        } else {
            panic!("Expected Modify result");
        }
    }

    #[tokio::test]
    async fn test_ping_pong_custom_response() {
        let worker = PingPongWorker::with_response("PONG!");
        
        let mut package = Package::new()
            .with_payload(TextPayload::new("/ping"))
            .with_target_site(TargetSite::tag("command"))
            .with_target_site(TargetSite::tag("command:ping"));
        
        let result = worker.handle_batch(vec![package]).await;
        
        if let WorkerResult::Modify(packages) = result {
            assert_eq!(packages.len(), 1);
            let pkg = &packages[0];
            
            if let Some(payload) = pkg.get_payload::<TextPayload>() {
                assert_eq!(payload.content, "PONG!");
            } else {
                panic!("Expected TextPayload");
            }
        } else {
            panic!("Expected Modify result");
        }
    }

    #[test]
    fn test_ping_pong_worker_type() {
        let worker = PingPongWorker::new();
        assert_eq!(worker.worker_type(), WorkerType::Output);
    }
}
