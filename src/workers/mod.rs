//! Workers module for Loquat framework
//!
//! Workers are processing units registered to pools by plugins.
//! They handle Packages asynchronously and can split/merge packages.

pub mod traits;
pub mod result;
pub mod registration;

pub use traits::*;
pub use result::*;
pub use registration::*;
