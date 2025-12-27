//! CLI (Command Line Interface) module
//! 
//! This module provides command-line interface tools for the Loquat framework,
//! including the plugin template generator.

pub mod plugin_generator;

pub use plugin_generator::{PluginCli, PluginLanguage, PluginTemplateConfig, PluginTemplateGenerator};
