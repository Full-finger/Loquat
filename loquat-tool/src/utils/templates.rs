//! Template generators for adapters and plugins

use super::helpers::{to_struct_name};

/// Plugin type enum
pub enum PluginType {
    Rust,
    Python,
    JavaScript,
}

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

/// Generate plugin files (returns relative paths from plugin directory)
pub fn plugin_files(name: &str) -> Vec<(&'static str, String)> {
    let struct_name = to_struct_name(name);
    
    vec![
        (
            "src/lib.rs",
            plugin_lib_impl(name, &struct_name),
        ),
        (
            "Cargo.toml",
            plugin_cargo_toml(name),
        ),
        (
            "config.json",
            plugin_config(name),
        ),
        (
            "README.md",
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
  "plugin_type": "rust",
  "enabled": true,
  "dependencies": [],
  "permissions": []
}}
"#, Name=name)
}

fn plugin_readme(name: &str) -> String {
    format!(r#"# {name} Plugin

A plugin for Loquat framework.

## Installation

Copy this plugin to your Loquat `plugins/` directory.

## Configuration

Edit `plugins/{name}/config.json` to configure this plugin.

## Usage

The plugin will automatically load when Loquat starts.

## Commands

- `hello` - Say hello from this plugin

## Development

Edit `src/lib.rs` to modify plugin behavior.

Build with:
```bash
cargo build --release
```
"#, name=name)
}

/// Generate Python plugin files (returns relative paths from plugin directory)
pub fn python_plugin_files(name: &str) -> Vec<(&'static str, String)> {
    let module_name = name.replace("-", "_");
    let class_name: String = name.replace("-", "_").split('_').map(|s| {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().chain(chars).collect(),
        }
    }).collect::<Vec<_>>().join("");
    
    vec![
        (
            "__init__.py",
            python_init_impl(name, &class_name),
        ),
        (
            "plugin.py",
            python_plugin_impl(name, &class_name),
        ),
        (
            "requirements.txt",
            python_requirements(name),
        ),
        (
            "pyproject.toml",
            python_pyproject(name, &module_name),
        ),
        (
            "config.json",
            python_config(name),
        ),
        (
            "README.md",
            python_readme(name),
        ),
    ]
}

fn python_init_impl(name: &str, class_name: &str) -> String {
    format!(r#"# {name} Python Plugin Package

# This package provides a Loquat plugin implementation in Python.

from .plugin import {Class}Plugin

__all__ = ["{Class}Plugin"]
"#, name=name, Class=class_name)
}

fn python_plugin_impl(name: &str, class_name: &str) -> String {
    format!(r#"# {class_name} Plugin Implementation

# This module provides a Python implementation of a Loquat plugin.

from typing import Optional, Dict, Any
import json


class {class_name}Plugin:
    """A Python plugin for Loquat framework."""
    
    def __init__(self):
        """Initialize plugin."""
        self._name = "{name_val}"
        self._version = "0.1.0"
        self._description = "{name_val} plugin for Loquat framework"
        self._config: Dict[str, Any] = {{}}
        self._initialized = False
    
    # Required Plugin Methods
    # ========================
    
    @property
    def name(self) -> str:
        """Get plugin name."""
        return self._name
    
    @property
    def version(self) -> str:
        """Get plugin version."""
        return self._version
    
    @property
    def description(self) -> str:
        """Get plugin description."""
        return self._description
    
    async def init(self) -> None:
        """Initialize plugin.
        
        This is called when plugin is loaded.
        """
        print(f"Initializing {{self._name}} plugin...")
        self._initialized = True
    
    async def load(self) -> None:
        """Load plugin.
        
        This is called after init() and allows plugin to set up resources.
        """
        print(f"Loading {{self._name}} plugin...")
    
    async def unload(self) -> None:
        """Unload plugin.
        
        This is called when plugin is being unloaded or reloaded.
        """
        print(f"Unloading {{self._name}} plugin...")
        self._initialized = False
    
    async def reload(self) -> None:
        """Reload plugin.
        
        This is called to hot-reload plugin.
        """
        await self.unload()
        await self.init()
        await self.load()
    
    @property
    def is_ready(self) -> bool:
        """Check if plugin is ready to handle events."""
        return self._initialized
    
    async def update_config(self, config: Dict[str, Any]) -> None:
        """Update plugin configuration.
        
        Args:
            config: New configuration dictionary
        """
        self._config.update(config)
        print(f"Configuration updated for {{self._name}}")
    
    @property
    def health_status(self) -> str:
        """Get plugin health status.
        
        Returns:
            One of: "healthy", "degraded", "unhealthy"
        """
        return "healthy"
    
    # Event and Command Handlers
    # ==========================
    
    async def on_event(self, event: Dict[str, Any]) -> None:
        """Handle an event from Loquat.
        
        Args:
            event: Event data as a dictionary
        """
        print(f"{{self._name}} received event: {{json.dumps(event, indent=2)}}")
        
        # Add your event handling logic here
        event_type = event.get("type", "unknown")
        
        if event_type == "message":
            await self._handle_message(event)
        elif event_type == "notice":
            await self._handle_notice(event)
    
    async def on_command(self, command: str, args: Optional[Dict[str, Any]] = None) -> Optional[str]:
        """Handle a command sent to plugin.
        
        Args:
            command: Command name
            args: Command arguments (optional)
            
        Returns:
            Response string if command was handled, None otherwise
        """
        print(f"{{self._name}} received command: {{command}}")
        
        if command == "hello":
            return f"Hello from {{self._name}} plugin!"
        
        # Return None if command was not handled
        return None
    
    # Plugin-Specific Methods
    # =======================
    
    async def _handle_message(self, event: Dict[str, Any]) -> None:
        """Handle message events.
        
        Args:
            event: Message event data
        """
        # Add your message handling logic here
        pass
    
    async def _handle_notice(self, event: Dict[str, Any]) -> None:
        """Handle notice events.
        
        Args:
            event: Notice event data
        """
        # Add your notice handling logic here
        pass
    
    # Utility Methods
    # ===============
    
    def get_config(self) -> Dict[str, Any]:
        """Get current plugin configuration."""
        return self._config.copy()
    
    def set_config(self, config: Dict[str, Any]) -> None:
        """Set plugin configuration.
        
        Args:
            config: New configuration dictionary
        """
        self._config = config.copy()


# Plugin Factory
# ==============

def create_plugin() -> {class_name}Plugin:
    """Create a new instance of plugin.
    
    This function is called by Loquat to create plugin instances.
    
    Returns:
        A new plugin instance
    """
    return {class_name}Plugin()


# Export plugin factory
__all__ = ["create_plugin", "{class_name}Plugin"]
"#, class_name=class_name, name_val=name)
}

fn python_requirements(name: &str) -> String {
    format!(r#"# {name} Plugin Dependencies

# Core dependencies
# Add required Python packages here
# Example:
# requests>=2.31.0
# pydantic>=2.0.0

# Loquat Python SDK (when available)
# loquat-python>=0.1.0
"#, name=name)
}

fn python_pyproject(name: &str, module_name: &str) -> String {
    format!(r#"[build-system]
requires = ["setuptools>=61.0", "wheel"]
build-backend = "setuptools.build_meta"

[project]
name = "{name}-plugin"
version = "0.1.0"
description = "{name} plugin for Loquat framework"
readme = "README.md"
requires-python = ">=3.8"
license = {{text = "MIT"}}
authors = [
    {{name = "Your Name", email = "your.email@example.com"}},
]

[project.optional-dependencies]
dev = [
    "pytest>=7.0.0",
    "pytest-asyncio>=0.21.0",
    "black>=23.0.0",
    "mypy>=1.0.0",
]

[tool.setuptools.packages.find]
where = ["."]
include = ["{module_name}*"]

[tool.black]
line-length = 88
target-version = ['py38']

[tool.mypy]
python_version = "3.8"
warn_return_any = true
warn_unused_configs = true
"#, name=name, module_name=module_name)
}

fn python_config(name: &str) -> String {
    let description = format!("{} plugin for Loquat framework", name);
    let json_obj = serde_json::json! {{
        "name": name,
        "version": "0.1.0",
        "description": description,
        "plugin_type": "python",
        "enabled": true,
        "entry_point": "plugin.py",
        "dependencies": [],
        "permissions": [],
        "config": serde_json::json! {{}},
    }};
    serde_json::to_string_pretty(&json_obj).unwrap()
}

fn python_readme(name: &str) -> String {
    format!(r#"# {} Plugin (Python)

A Python plugin for Loquat framework.

## Installation

Copy this plugin to your Loquat `plugins/` directory.

## Requirements

- Python 3.8 or higher
- Dependencies listed in `requirements.txt`

Install dependencies:
```bash
cd plugins/{}
pip install -r requirements.txt
```

## Configuration

Edit `plugins/{}/config.json` to configure this plugin.

## Usage

The plugin will automatically load when Loquat starts.

## Commands

- `hello` - Say hello from this plugin

## Development

Edit `plugin.py` to modify plugin behavior.

### Plugin Structure

- `__init__.py` - Package initialization
- `plugin.py` - Main plugin implementation
- `requirements.txt` - Python dependencies
- `pyproject.toml` - Project configuration
- `config.json` - Loquat plugin configuration

### Testing

Run tests with:
```bash
cd plugins/{}
pytest
```

### Code Style

Format code with Black:
```bash
black plugin.py
```

Type check with mypy:
```bash
mypy plugin.py
```
"#, name, name, name, name)
}
