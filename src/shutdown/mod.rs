//! Graceful shutdown coordination
//!
//! Provides coordinated shutdown of all components with timeout handling
//! and resource cleanup guarantees.

pub mod coordinator;
pub mod stages;

pub use coordinator::{ShutdownCoordinator, ShutdownStatus};
pub use stages::{ShutdownStage, ShutdownOrder, ShutdownStageResult};
