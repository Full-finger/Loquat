//! Workers module for Loquat framework
//!
//! Workers are processing units registered to pools by plugins.
//! They handle Packages asynchronously and can split/merge packages.

pub mod traits;
pub mod result;
pub mod registration;
pub mod matcher;
pub mod conversion;
pub mod command_parser;
pub mod ping_pong;

pub use traits::*;
pub use result::*;
pub use registration::*;
pub use matcher::*;
pub use conversion::*;
pub use command_parser::*;
pub use ping_pong::*;
