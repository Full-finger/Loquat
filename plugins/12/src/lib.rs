//! 12 - A Loquat plugin

use async_trait::async_trait;
use loquat::plugins::{Plugin, PluginHealth, PluginType, traits::Plugin};
use loquat::errors::Result;
use serde::Deserialize;

/// Plugin configuration
#[derive(Debug, Deserialize)]
pub struct Config {
    // Add your plugin configuration here
    pub enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
        }
    }
}

/// Main plugin struct
pub struct 12 {
    name: String,
    version: String,
    description: String,
    author: String,
    config: Config,
}

impl 12 {
    /// Create a new plugin instance
    pub fn new() -> Self {
        Self {
            name: "12".to_string(),
            version: "0.1.0".to_string(),
            description: "A Loquat plugin".to_string(),
            author: "Your Name".to_string(),
            config: Config::default(),
        }
    }
}

#[async_trait]
impl Plugin for 12 {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Native
    }

    fn description(&self) -> Option<&str> {
        Some(&self.description)
    }

    fn author(&self) -> Option<&str> {
        Some(&self.author)
    }

    async fn init(&mut self) -> Result<()> {
        // Initialize your plugin here
        println!("{} v{} initialized!", self.name, self.version);
        Ok(())
    }

    async fn load(&mut self) -> Result<()> {
        // Load your plugin resources here
        println!("{} loaded!", self.name);
        Ok(())
    }

    async fn unload(&mut self) -> Result<()> {
        // Cleanup your plugin resources here
        println!("{} unloaded!", self.name);
        Ok(())
    }

    fn health_status(&self) -> PluginHealth {
        // Return plugin health status
        PluginHealth::Healthy
    }

    async fn update_config(&mut self, config: serde_json::Value) -> Result<()> {
        // Update plugin configuration
        if let Ok(new_config) = serde_json::from_value::<Config>(config) {
            self.config = new_config;
            println!("{} config updated!", self.name);
        }
        Ok(())
    }
}

/// Plugin constructor (called by the loader)
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {
    let plugin = 12::new();
    Box::into_raw(Box::new(plugin))
}

/// Plugin destructor (called by the loader)
#[no_mangle]
pub extern "C" fn destroy_plugin(plugin: *mut dyn Plugin) {
    if !plugin.is_null() {
        unsafe {
            let _ = Box::from_raw(plugin);
        }
    }
}
