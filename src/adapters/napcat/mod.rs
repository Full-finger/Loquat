//! NapCat Adapter implementation
//!
//! This adapter communicates with NapCat QQ bot framework via OneBot protocol.

pub mod adapter;
pub mod factory;

pub use adapter::NapCatAdapter;
pub use factory::NapCatAdapterFactory;
