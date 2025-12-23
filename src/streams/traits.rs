//! Stream trait definition

use async_trait::async_trait;
use crate::events::Package;
use std::fmt::Debug;

/// Stream trait - processes packages through 9 pools in sequence
#[async_trait]
pub trait Stream: Send + Sync + Debug {
    /// Get stream ID
    fn stream_id(&self) -> &str;
    
    /// Get associated channel ID
    fn channel_id(&self) -> &str;
    
    /// Process packages through all 9 pools in sequence
    async fn process(&self, packages: Vec<Package>) -> crate::errors::Result<Vec<Package>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MockStream;

    #[async_trait]
    impl Stream for MockStream {
        fn stream_id(&self) -> &str {
            "mock_stream"
        }

        fn channel_id(&self) -> &str {
            "mock_channel"
        }

        async fn process(&self, packages: Vec<Package>) -> crate::errors::Result<Vec<Package>> {
            Ok(packages)
        }
    }

    #[tokio::test]
    async fn test_mock_stream() {
        let stream = MockStream;
        assert_eq!(stream.stream_id(), "mock_stream");
        assert_eq!(stream.channel_id(), "mock_channel");

        let packages = vec![Package::new()];
        let result = stream.process(packages).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }
}
