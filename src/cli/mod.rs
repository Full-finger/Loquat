//! CLI (Command Line Interface) module
//! 
//! This module provides command-line interface tools for Loquat framework,
//! including plugin template generator and remove command.

pub mod plugin_generator;
pub mod remove;

pub use plugin_generator::{PluginCli, PluginLanguage, PluginTemplateConfig, PluginTemplateGenerator};
pub use remove::RemoveCli;
