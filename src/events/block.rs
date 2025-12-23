//! Block structure - contains groups
//!
//! Block contains an array of Group objects.

use crate::events::Group;
use serde::{Serialize, Deserialize};
use std::fmt::Debug;

/// Block type classification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockType {
    /// Default block type
    Default,
    /// Message block
    Message,
    /// Notice block
    Notice,
    /// Request block
    Request,
    /// Meta block
    Meta,
    /// Custom block type
    Custom(String),
}

/// Block - contains an array of Group objects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Groups - array of event groups
    pub groups: Vec<Group>,
    
    /// Block type
    pub block_type: BlockType,
    
    /// Block ID
    pub block_id: String,
}

impl Block {
    /// Create a new block
    pub fn new(block_type: BlockType) -> Self {
        Self {
            groups: Vec::new(),
            block_type,
            block_id: format!("block-{}-{}", 
                chrono::Utc::now().timestamp_millis(),
                uuid::Uuid::new_v4()),
        }
    }
    
    /// Create a default block
    pub fn default_block() -> Self {
        Self::new(BlockType::Default)
    }
    
    /// Create a message block
    pub fn message_block() -> Self {
        Self::new(BlockType::Message)
    }
    
    /// Add a group
    pub fn with_group(mut self, group: Group) -> Self {
        self.groups.push(group);
        self
    }
    
    /// Add multiple groups
    pub fn with_groups(mut self, groups: Vec<Group>) -> Self {
        self.groups.extend(groups);
        self
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new(BlockType::Default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_creation() {
        let block = Block::new(BlockType::Message);
        
        assert!(block.groups.is_empty());
        assert_eq!(block.block_type, BlockType::Message);
        assert!(!block.block_id.is_empty());
    }
    
    #[test]
    fn test_block_builder() {
        let group = Group::new("test_group");
        let block = Block::new(BlockType::Default)
            .with_group(group);
        
        assert_eq!(block.groups.len(), 1);
    }
}
