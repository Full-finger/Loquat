//! Template generators for adapters and plugins

use super::helpers::{to_struct_name};

/// Generate adapter files
pub fn adapter_files(name: &str) -> Vec<(String, String)> {
    let struct_name = to_struct_name(name);
    
    vec![
        (
            format!("src/adapters/{}/mod.rs", name),
            format!(r#"//! {} adapter module

pub mod adapter;
pub mod factory;

pub use adapter::*;
pub use factory::*;
"#, name),
        ),
        (
            format!("src/adapters/{}/adapter.rs", name),
            adapter_adapter_impl(name, &struct_name),
        ),
        (
            format!("src/adapters/{}/factory.rs", name),
            adapter_factory_impl(name, &struct_name),
        ),
    ]
}

fn adapter_adapter_impl(name: &str, struct_name: &str) -> String {
    format!(r#"//! {Struct} adapter implementation

use loquat::adapters::core::traits::Adapter;
use loquat::events::Event;
use loquat::errors::Result;
use tokio::sync::mpsc;

pub struct {Struct} {{
    event_sender: Option<mpsc::UnboundedSender<Event>>,
    // Add your adapter-specific fields here
    config: serde_json::Value,
}}

impl {Struct} {{
    pub fn new(config: serde_json::Value) -> Self {{
        Self {{
            event_sender: None,
            config,
        }}
    }}
}}

#[async_trait::async_trait]
impl Adapter for {Struct} {{
    fn name(&self) -> &str {{
        "{Name}"
    }}
    
    async fn initialize(&mut self) -> Result<()> {{
        // Initialize your adapter here
        // For example: connect to external services, setup resources, etc.
        println!("Initializing {Name} adapter...");
        Ok(())
    }}
    
    async fn start(&mut self, sender: mpsc::UnboundedSender<Event>) -> Result<()> {{
        self.event_sender = Some(sender.clone());
        
        // Start your adapter here
        // For example: start listening for events, spawn tasks, etc.
        println!("{Name} adapter started");
        
        Ok(())
    }}
    
    async fn stop(&mut self) -> Result<()> {{
        // Stop your adapter here
        // For example: close connections, cleanup resources, etc.
        println!("{Name} adapter stopped");
        Ok(())
    }}
    
    async fn send_event(&mut self, event: Event) -> Result<()> {{
        // Handle outgoing events
        // For example: send messages to external services, etc.
        println!("{{}}: Sending event: {{:?}}", self.name(), event);
        Ok(())
    }}
    
    async fn handle_command(&mut self, command: String) -> Result<String> {{
        // Handle adapter-specific commands
        format!("{{}} handled command: {{}}", self.name(), command)
    }}
    
    fn get_config(&self) -> serde_json::Value {{
        self.config.clone()
    }}
    
    fn set_config(&mut self, config: serde_json::Value) -> Result<()> {{
        self.config = config;
        Ok(())
    }}
}}
"#, Struct=struct_name, Name=name)
}

fn adapter_factory_impl(name: &str, struct_name: &str) -> String {
    format!(r#"//! {Struct} adapter factory

use loquat::adapters::core::traits::{{
    Adapter, AdapterFactory
}};
use loquat::errors::Result;
use super::adapter::{Struct};

pub struct {Struct}Factory;

impl {Struct}Factory {{
    pub fn new() -> Self {{
        Self
    }}
}}

impl AdapterFactory for {Struct}Factory {{
    fn name(&self) -> &str {{
        "{Name}"
    }}
    
    fn create(&self, config: serde_json::Value) -> Result<Box<dyn Adapter>> {{
        Ok(Box::new({Struct}::new(config)))
    }}
    
    fn default_config(&self) -> serde_json::Value {{
        serde_json::json! {{
            "enabled": true,
            // Add your default configuration here
        }}
    }}
}}
"#, Struct=struct_name, Name=name)
}

/// Generate adapter config file
pub fn adapter_config(name: &str) -> String {
    format!(r#"{{
  "name": "{Name}",
  "version": "0.1.0",
  "description": "{Name} adapter for Loquat framework",
  "enabled": true,
  "config": {{
    // Add your adapter configuration here
  }}
}}
"#, Name=name)
}

/// Generate plugin files
pub fn plugin_files(name: &str) -> Vec<(String, String)> {
    let struct_name = to_struct_name(name);
    
    vec![
        (
            format!("plugins/{}/src/lib.rs", name),
            plugin_lib_impl(name, &struct_name),
        ),
        (
            format!("plugins/{}/Cargo.toml", name),
            plugin_cargo_toml(name),
        ),
        (
            format!("plugins/{}/config.json", name),
            plugin_config(name),
        ),
        (
            format!("plugins/{}/README.md", name),
            plugin_readme(name),
        ),
    ]
}

fn plugin_lib_impl(name: &str, struct_name: &str) -> String {
    format!(r#"//! {Struct} plugin

use loquat::plugins::traits::Plugin;
use loquat::events::Event;
use loquat::errors::Result;

pub struct {Struct};

impl {Struct} {{
    pub fn new() -> Self {{
        Self
    }}
}}

impl Plugin for {Struct} {{
    fn name(&self) -> &str {{
        "{Name}"
    }}
    
    fn version(&self) -> &str {{
        "0.1.0"
    }}
    
    fn description(&self) -> &str {{
        "{Name} plugin for Loquat framework"
    }}
    
    async fn on_load(&mut self) -> Result<()> {{
        println!("Loading {Name} plugin...");
        Ok(())
    }}
    
    async fn on_unload(&mut self) -> Result<()> {{
        println!("Unloading {Name} plugin...");
        Ok(())
    }}
    
    async fn on_event(&mut self, event: &Event) -> Result<()> {{
        // Handle events here
        println!("{{}} received event: {{:?}}", self.name(), event);
        Ok(())
    }}
    
    async fn on_command(&mut self, command: &str) -> Result<Option<String>> {{
        // Handle commands here
        if command == "hello" {{
            Ok(Some("Hello from {Name} plugin!".to_string()))
        }} else {{
            Ok(None)
        }}
    }}
}}
"#, Struct=struct_name, Name=name)
}

fn plugin_cargo_toml(name: &str) -> String {
    format!(r#"[package]
name = "{Name}-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
loquat = {{ path = "../../" }}
async-trait = "0.1"
"#, Name=name.replace("-", "_"))
}

fn plugin_config(name: &str) -> String {
    format!(r#"{{
  "name": "{Name}",
  "version": "0.1.0",
  "description": "{Name} plugin for Loquat framework",
  "enabled": true,
  "dependencies": [],
  "permissions": []
}}
"#, Name=name)
}

fn plugin_readme(name: &str) -> String {
    format!(r#"# {name} Plugin

A plugin for the Loquat framework.

## Installation

Copy this plugin to your Loquat `plugins/` directory.

## Configuration

Edit `plugins/{}/config.json` to configure this plugin.

## Usage

The plugin will automatically load when Loquat starts.

## Commands

- `hello` - Say hello from this plugin

## Development

Edit `src/lib.rs` to modify the plugin behavior.

Build with:
```bash
cargo build --release
```
"#, name)
}
