//! Channel structure

use crate::channels::ChannelType;
use crate::events::Package;
use crate::logging::traits::Logger;
use std::fmt::Debug;

/// Channel - abstract communication channel with a unique stream
#[derive(Debug)]
pub struct Channel {
    /// Channel ID (unique identifier)
    pub channel_id: String,
    
    /// Channel type
    pub channel_type: ChannelType,
    
    /// Stream for processing packages through 9 pools
    pub stream: Box<dyn crate::streams::Stream>,
}

impl Channel {
    /// Create a new channel
    pub fn new(channel_id: String, channel_type: ChannelType, stream: Box<dyn crate::streams::Stream>) -> Self {
        Self {
            channel_id,
            channel_type,
            stream,
        }
    }
    
    /// Get channel ID
    pub fn id(&self) -> &str {
        &self.channel_id
    }
    
    /// Process a batch of packages through channel's stream
    pub async fn process_batch(&self, packages: Vec<Package>) -> crate::errors::Result<Vec<Package>> {
        self.stream.process(packages).await
    }
}
