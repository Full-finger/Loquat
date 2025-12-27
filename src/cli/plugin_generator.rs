//! Plugin template generator for Loquat framework
//!
//! Generates plugin templates for different languages (Rust, Python, JavaScript)

use crate::errors::{PluginError, Result};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::SystemTime;

/// Plugin template configuration
#[derive(Debug, Clone)]
pub struct PluginTemplateConfig {
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin author
    pub author: String,
    /// Plugin description
    pub description: String,
    /// Plugin type (Rust, Python, JavaScript)
    pub plugin_type: PluginLanguage,
    /// Output directory
    pub output_dir: PathBuf,
    /// Include example code
    pub include_example: bool,
}

/// Supported plugin languages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginLanguage {
    Rust,
    Python,
    JavaScript,
}

impl PluginLanguage {
    /// Get language from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "rust" | "rs" => Some(PluginLanguage::Rust),
            "python" | "py" => Some(PluginLanguage::Python),
            "javascript" | "js" | "typescript" | "ts" => Some(PluginLanguage::JavaScript),
            _ => None,
        }
    }

    /// Get file extension
    pub fn extension(&self) -> &str {
        match self {
            PluginLanguage::Rust => "rs",
            PluginLanguage::Python => "py",
            PluginLanguage::JavaScript => "ts",
        }
    }

    /// Get file extension for compiled library
    pub fn lib_extension(&self) -> &str {
        match self {
            PluginLanguage::Rust => {
                #[cfg(target_os = "windows")]
                return "dll";
                #[cfg(target_os = "macos")]
                return "dylib";
                #[cfg(target_os = "linux")]
                return "so";
            }
            PluginLanguage::Python => "py",
            PluginLanguage::JavaScript => "js",
        }
    }
}

impl Default for PluginTemplateConfig {
    fn default() -> Self {
        Self {
            name: "my_plugin".to_string(),
            version: "0.1.0".to_string(),
            author: "Your Name".to_string(),
            description: "A Loquat plugin".to_string(),
            plugin_type: PluginLanguage::Rust,
            output_dir: PathBuf::from("./plugins"),
            include_example: true,
        }
    }
}

/// Plugin template generator
pub struct PluginTemplateGenerator {
    config: PluginTemplateConfig,
}

impl PluginTemplateGenerator {
    /// Create a new plugin template generator
    pub fn new(config: PluginTemplateConfig) -> Self {
        Self { config }
    }

    /// Generate plugin template
    pub fn generate(&self) -> Result<PathBuf> {
        let plugin_dir = self.config.output_dir.join(&self.config.name);
        
        // Create plugin directory
        fs::create_dir_all(&plugin_dir)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to create plugin directory: {}", e)))?;

        // Generate files based on plugin type
        match self.config.plugin_type {
            PluginLanguage::Rust => self.generate_rust_plugin(&plugin_dir)?,
            PluginLanguage::Python => self.generate_python_plugin(&plugin_dir)?,
            PluginLanguage::JavaScript => self.generate_javascript_plugin(&plugin_dir)?,
        }

        println!("✓ Plugin template generated successfully!");
        println!("  Location: {}", plugin_dir.display());
        println!("  Type: {:?}", self.config.plugin_type);
        
        Ok(plugin_dir)
    }

    /// Generate Rust plugin template
    fn generate_rust_plugin(&self, plugin_dir: &PathBuf) -> Result<()> {
        // Generate Cargo.toml
        let cargo_toml = self.generate_rust_cargo_toml();
        self.write_file(plugin_dir, "Cargo.toml", &cargo_toml)?;

        // Generate src/lib.rs
        let lib_rs = self.generate_rust_lib();
        let src_dir = plugin_dir.join("src");
        fs::create_dir_all(&src_dir)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to create src directory: {}", e)))?;
        self.write_file(&src_dir, "lib.rs", &lib_rs)?;

        // Generate README.md
        let readme = self.generate_readme();
        self.write_file(plugin_dir, "README.md", &readme)?;

        if self.config.include_example {
            // Generate example config
            let config = self.generate_plugin_config();
            self.write_file(plugin_dir, "config.json", &config)?;
        }

        Ok(())
    }

    /// Generate Python plugin template
    fn generate_python_plugin(&self, plugin_dir: &PathBuf) -> Result<()> {
        // Generate main.py
        let main_py = self.generate_python_main();
        self.write_file(plugin_dir, "main.py", &main_py)?;

        // Generate requirements.txt
        let requirements = "loquat-py-sdk>=0.1.0\n".to_string();
        self.write_file(plugin_dir, "requirements.txt", &requirements)?;

        // Generate README.md
        let readme = self.generate_readme();
        self.write_file(plugin_dir, "README.md", &readme)?;

        if self.config.include_example {
            // Generate config.json
            let config = self.generate_plugin_config();
            self.write_file(plugin_dir, "config.json", &config)?;
        }

        Ok(())
    }

    /// Generate JavaScript/TypeScript plugin template
    fn generate_javascript_plugin(&self, plugin_dir: &PathBuf) -> Result<()> {
        // Generate package.json
        let package_json = self.generate_js_package_json();
        self.write_file(plugin_dir, "package.json", &package_json)?;

        // Generate src/index.ts
        let index_ts = self.generate_js_main();
        let src_dir = plugin_dir.join("src");
        fs::create_dir_all(&src_dir)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to create src directory: {}", e)))?;
        self.write_file(&src_dir, "index.ts", &index_ts)?;

        // Generate tsconfig.json
        let tsconfig = self.generate_tsconfig();
        self.write_file(plugin_dir, "tsconfig.json", &tsconfig)?;

        // Generate README.md
        let readme = self.generate_readme();
        self.write_file(plugin_dir, "README.md", &readme)?;

        if self.config.include_example {
            // Generate config.json
            let config = self.generate_plugin_config();
            self.write_file(plugin_dir, "config.json", &config)?;
        }

        Ok(())
    }

    /// Write file to disk
    fn write_file(&self, dir: &PathBuf, filename: &str, content: &str) -> Result<()> {
        let file_path = dir.join(filename);
        fs::write(&file_path, content)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to write file {}: {}", filename, e)))?;
        println!("  ✓ Generated: {}", filename);
        Ok(())
    }

    /// Generate Rust Cargo.toml
    fn generate_rust_cargo_toml(&self) -> String {
        format!(
            r#"[package]
name = "{}"
version = "{}"
edition = "2021"

[lib]
name = "{}"
crate-type = ["cdylib"]

[dependencies]
loquat = "{{ version = \"0.1.0\", path = \"../..\" }}"
async-trait = "0.1"
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
tokio = {{ version = "1.0", features = ["full"] }}
"#,
            self.config.name,
            self.config.version,
            self.config.name.replace("-", "_")
        )
    }

    /// Generate Rust src/lib.rs
    fn generate_rust_lib(&self) -> String {
        format!(
            r#"//! {} - A Loquat plugin

use async_trait::async_trait;
use loquat::plugins::{{Plugin, PluginHealth, PluginType, traits::Plugin}};
use loquat::errors::Result;
use serde::Deserialize;

/// Plugin configuration
#[derive(Debug, Deserialize)]
pub struct Config {{
    // Add your plugin configuration here
    pub enabled: bool,
}}

impl Default for Config {{
    fn default() -> Self {{
        Self {{
            enabled: true,
        }}
    }}
}}

/// Main plugin struct
pub struct {} {{
    name: String,
    version: String,
    description: String,
    author: String,
    config: Config,
}}

impl {} {{
    /// Create a new plugin instance
    pub fn new() -> Self {{
        Self {{
            name: "{}".to_string(),
            version: "{}".to_string(),
            description: "{}".to_string(),
            author: "{}".to_string(),
            config: Config::default(),
        }}
    }}
}}

#[async_trait]
impl Plugin for {} {{
    fn name(&self) -> &str {{
        &self.name
    }}

    fn version(&self) -> &str {{
        &self.version
    }}

    fn plugin_type(&self) -> PluginType {{
        PluginType::Native
    }}

    fn description(&self) -> Option<&str> {{
        Some(&self.description)
    }}

    fn author(&self) -> Option<&str> {{
        Some(&self.author)
    }}

    async fn init(&mut self) -> Result<()> {{
        // Initialize your plugin here
        println!("{{}} v{{}} initialized!", self.name, self.version);
        Ok(())
    }}

    async fn load(&mut self) -> Result<()> {{
        // Load your plugin resources here
        println!("{{}} loaded!", self.name);
        Ok(())
    }}

    async fn unload(&mut self) -> Result<()> {{
        // Cleanup your plugin resources here
        println!("{{}} unloaded!", self.name);
        Ok(())
    }}

    fn health_status(&self) -> PluginHealth {{
        // Return plugin health status
        PluginHealth::Healthy
    }}

    async fn update_config(&mut self, config: serde_json::Value) -> Result<()> {{
        // Update plugin configuration
        if let Ok(new_config) = serde_json::from_value::<Config>(config) {{
            self.config = new_config;
            println!("{{}} config updated!", self.name);
        }}
        Ok(())
    }}
}}

/// Plugin constructor (called by the loader)
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {{
    let plugin = {}::new();
    Box::into_raw(Box::new(plugin))
}}

/// Plugin destructor (called by the loader)
#[no_mangle]
pub extern "C" fn destroy_plugin(plugin: *mut dyn Plugin) {{
    if !plugin.is_null() {{
        unsafe {{
            let _ = Box::from_raw(plugin);
        }}
    }}
}}
"#,
            self.config.name,
            self.struct_name(),
            self.struct_name(),
            self.config.name,
            self.config.version,
            self.config.description,
            self.config.author,
            self.struct_name(),
            self.struct_name()
        )
    }

    /// Generate Python main.py
    fn generate_python_main(&self) -> String {
        format!(
            r#"#!/usr/bin/env python3
"""
{} - A Loquat plugin
"""

from typing import Optional, Dict, Any
import asyncio


class {}:
    """Main plugin class"""
    
    def __init__(self):
        self.name = "{}"
        self.version = "{}"
        self.description = "{}"
        self.author = "{}"
        self.enabled = True
    
    async def init(self) -> bool:
        """Initialize the plugin"""
        print(f"{{self.name}} v{{self.version}} initialized!")
        return True
    
    async def load(self) -> bool:
        """Load the plugin"""
        print(f"{{self.name}} loaded!")
        return True
    
    async def unload(self) -> bool:
        """Unload the plugin"""
        print(f"{{self.name}} unloaded!")
        return True
    
    async def reload(self) -> bool:
        """Reload the plugin"""
        await self.unload()
        return await self.load()
    
    def health_status(self) -> str:
        """Get plugin health status"""
        return "healthy"
    
    def update_config(self, config: Dict[str, Any]) -> bool:
        """Update plugin configuration"""
        print(f"{{self.name}} config updated!")
        return True
    
    def get_info(self) -> Dict[str, Any]:
        """Get plugin information"""
        return {{
            "name": self.name,
            "version": self.version,
            "description": self.description,
            "author": self.author,
            "enabled": self.enabled,
        }}


# Plugin entry point
def create_plugin() -> {}:
    """Create a plugin instance"""
    return {}()


if __name__ == "__main__":
    # Test the plugin
    plugin = create_plugin()
    asyncio.run(plugin.init())
    asyncio.run(plugin.load())
"#,
            self.config.name,
            self.struct_name(),
            self.config.name,
            self.config.version,
            self.config.description,
            self.config.author,
            self.struct_name(),
            self.struct_name()
        )
    }

    /// Generate JavaScript/TypeScript src/index.ts
    fn generate_js_main(&self) -> String {
        format!(
            r#"/**
 * {} - A Loquat plugin
 */

export interface Config {{
  // Add your plugin configuration here
  enabled: boolean;
}}

export interface PluginInfo {{
  name: string;
  version: string;
  description: string;
  author: string;
  pluginType: string;
}}

/**
 * Main plugin class
 */
export class {} {{
  private name: string;
  private version: string;
  private description: string;
  private author: string;
  private config: Config;

  constructor() {{
    this.name = "{}";
    this.version = "{}";
    this.description = "{}";
    this.author = "{}";
    this.config = {{ enabled: true }};
  }}

  /**
   * Initialize the plugin
   */
  async init(): Promise<boolean> {{
    console.log(`${{this.name}} v${{this.version}} initialized!`);
    return true;
  }}

  /**
   * Load the plugin
   */
  async load(): Promise<boolean> {{
    console.log(`${{this.name}} loaded!`);
    return true;
  }}

  /**
   * Unload the plugin
   */
  async unload(): Promise<boolean> {{
    console.log(`${{this.name}} unloaded!`);
    return true;
  }}

  /**
   * Reload the plugin
   */
  async reload(): Promise<boolean> {{
    await this.unload();
    return await this.load();
  }}

  /**
   * Get plugin health status
   */
  healthStatus(): string {{
    return "healthy";
  }}

  /**
   * Update plugin configuration
   */
  updateConfig(config: Config): boolean {{
    this.config = {{ ...this.config, ...config }};
    console.log(`${{this.name}} config updated!`);
    return true;
  }}

  /**
   * Get plugin information
   */
  getInfo(): PluginInfo {{
    return {{
      name: this.name,
      version: this.version,
      description: this.description,
      author: this.author,
      pluginType: "javascript",
    }};
  }}
}}

/**
 * Create a plugin instance
 */
export function createPlugin(): {} {{
  return new {}();
}}
"#,
            self.config.name,
            self.struct_name(),
            self.config.name,
            self.config.version,
            self.config.description,
            self.config.author,
            self.struct_name(),
            self.struct_name()
        )
    }

    /// Generate JavaScript package.json
    fn generate_js_package_json(&self) -> String {
        format!(
            r#"{{
  "name": "{}",
  "version": "{}",
  "description": "{}",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "scripts": {{
    "build": "tsc",
    "watch": "tsc --watch",
    "dev": "tsc --watch"
  }},
  "keywords": ["loquat", "plugin"],
  "author": "{}",
  "license": "MIT",
  "devDependencies": {{
    "typescript": "^5.0.0",
    "@types/node": "^20.0.0"
  }},
  "dependencies": {{
    "loquat-js-sdk": "^0.1.0"
  }}
}}
"#,
            self.config.name, self.config.version, self.config.description, self.config.author
        )
    }

    /// Generate TypeScript tsconfig.json
    fn generate_tsconfig(&self) -> String {
        r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "lib": ["ES2020"],
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}
"#
        .to_string()
    }

    /// Generate plugin config.json
    fn generate_plugin_config(&self) -> String {
        format!(
            r#"{{
  "name": "{}",
  "version": "{}",
  "enabled": true,
  "config": {{
    "enabled": true
  }}
}}
"#,
            self.config.name, self.config.version
        )
    }

    /// Generate README.md
    fn generate_readme(&self) -> String {
        format!(
            r#"# {}

{}

## Installation

### For Rust Plugins
```bash
cargo build --release
# Copy the compiled library to your plugins directory
cp target/release/lib{}.dll ./plugins/  # Windows
cp target/release/lib{}.so ./plugins/     # Linux
cp target/release/lib{}.dylib ./plugins/ # macOS
```

### For Python Plugins
```bash
pip install -r requirements.txt
# Copy the plugin file to your plugins directory
cp main.py ./plugins/
```

### For JavaScript/TypeScript Plugins
```bash
npm install
npm run build
# Copy the compiled file to your plugins directory
cp dist/index.js ./plugins/
```

## Configuration

Add the following to your `config.toml`:

```toml
[plugins.{}]
enabled = true
auto_load = true
```

## Usage

The plugin will be automatically loaded by the Loquat framework.

## Author

{}
"#,
            self.config.name,
            self.config.description,
            self.config.name,
            self.config.name,
            self.config.name,
            self.config.name,
            self.config.author
        )
    }

    /// Convert plugin name to struct name (PascalCase)
    fn struct_name(&self) -> String {
        self.config
            .name
            .split('_')
            .map(|s| {
                let mut chars = s.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect()
    }
}

/// Command-line interface for plugin generation
pub struct PluginCli {
    config: PluginTemplateConfig,
}

impl PluginCli {
    /// Create a new plugin CLI
    pub fn new() -> Self {
        Self {
            config: PluginTemplateConfig::default(),
        }
    }

    /// Parse command-line arguments and generate plugin
    pub fn run_from_args(&mut self, args: Vec<String>) -> Result<PathBuf> {
        let mut name = None;
        let mut plugin_type = None;
        let mut author = None;
        let mut description = None;
        let mut version = None;
        let mut output_dir = PathBuf::from(".");

        for i in 0..args.len() {
            match args[i].as_str() {
                "--name" | "-n" => {
                    if i + 1 < args.len() {
                        name = Some(args[i + 1].clone());
                    }
                }
                "--type" | "-t" => {
                    if i + 1 < args.len() {
                        plugin_type = PluginLanguage::from_str(&args[i + 1]);
                        if plugin_type.is_none() {
                            eprintln!("Error: Invalid plugin type '{}'", args[i + 1]);
                            eprintln!("Valid types: rust, python, javascript");
                            std::process::exit(1);
                        }
                    }
                }
                "--author" | "-a" => {
                    if i + 1 < args.len() {
                        author = Some(args[i + 1].clone());
                    }
                }
                "--description" | "-d" => {
                    if i + 1 < args.len() {
                        description = Some(args[i + 1].clone());
                    }
                }
                "--version" | "-v" => {
                    if i + 1 < args.len() {
                        version = Some(args[i + 1].clone());
                    }
                }
                "--output" | "-o" => {
                    if i + 1 < args.len() {
                        output_dir = PathBuf::from(&args[i + 1]);
                    }
                }
                "--help" | "-h" => {
                    self.print_help();
                    std::process::exit(0);
                }
                _ => {
                    // Treat as plugin name if no flag
                    if !args[i].starts_with('-') && name.is_none() {
                        name = Some(args[i].clone());
                    }
                }
            }
        }

        // Apply parsed arguments
        if let Some(n) = name {
            self.config.name = n;
        }
        if let Some(t) = plugin_type {
            self.config.plugin_type = t;
        }
        if let Some(a) = author {
            self.config.author = a;
        }
        if let Some(d) = description {
            self.config.description = d;
        }
        if let Some(v) = version {
            self.config.version = v;
        }
        self.config.output_dir = output_dir;

        // Validate
        if self.config.name.is_empty() {
            eprintln!("Error: Plugin name is required");
            eprintln!("Use --name or --help for more information");
            std::process::exit(1);
        }

        // Generate plugin
        let generator = PluginTemplateGenerator::new(self.config.clone());
        generator.generate()
    }

    /// Run interactive mode
    pub fn run_interactive(&mut self) -> Result<PathBuf> {
        // Print banner
        println!();
        println!("╔══════════════════════════════════════════════════════════╗");
        println!("║              Loquat Plugin Creator                        ║");
        println!("╚══════════════════════════════════════════════════════════╝");
        println!();
        println!("欢迎使用 Loquat 插件生成器！");
        println!("我将引导您创建一个新的插件项目。");
        println!();

        // Input plugin name
        self.config.name = self.prompt_input("请输入插件名称", &self.config.name, true)?;
        println!();

        // Select plugin type
        self.config.plugin_type = self.prompt_plugin_type()?;
        println!();

        // Input author (optional)
        self.config.author = self.prompt_input("请输入插件作者 (可选，按Enter跳过)", &self.config.author, false)?;
        println!();

        // Input description (optional)
        self.config.description = self.prompt_input("请输入插件描述 (可选，按Enter跳过)", &self.config.description, false)?;
        println!();

        // Input version (optional with default)
        self.config.version = self.prompt_input("请输入插件版本 (默认 0.1.0，按Enter使用默认值)", &self.config.version, false)?;
        println!();

        // Input output directory (optional with default)
        let default_output = self.config.output_dir.to_string_lossy().to_string();
        let output_str = self.prompt_input("请输入输出目录 (默认 ./plugins，按Enter使用默认值)", &default_output, false)?;
        if !output_str.is_empty() {
            self.config.output_dir = PathBuf::from(&output_str);
        }
        println!();

        // Generate plugin
        println!("正在生成插件...");
        println!();
        let generator = PluginTemplateGenerator::new(self.config.clone());
        generator.generate()
    }

    /// Prompt user for input
    fn prompt_input(&self, prompt: &str, default: &str, required: bool) -> Result<String> {
        loop {
            print!("{} [{}]: ", prompt, default);
            io::stdout().flush().map_err(|e| PluginError::LoadFailed(format!("Failed to flush stdout: {}", e)))?;

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(|e| PluginError::LoadFailed(format!("Failed to read input: {}", e)))?;

            let input = input.trim().to_string();

            if input.is_empty() {
                if required {
                    println!("此项为必填项，请重新输入。");
                    continue;
                }
                return Ok(default.to_string());
            }

            return Ok(input);
        }
    }

    /// Prompt user to select plugin type
    fn prompt_plugin_type(&self) -> Result<PluginLanguage> {
        loop {
            println!("请选择插件编程语言:");
            println!("  1. Rust");
            println!("  2. Python");
            println!("  3. JavaScript/TypeScript");
            print!("请输入选项 [1-3]: ");
            io::stdout().flush().map_err(|e| PluginError::LoadFailed(format!("Failed to flush stdout: {}", e)))?;

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(|e| PluginError::LoadFailed(format!("Failed to read input: {}", e)))?;

            match input.trim() {
                "1" => return Ok(PluginLanguage::Rust),
                "2" => return Ok(PluginLanguage::Python),
                "3" => return Ok(PluginLanguage::JavaScript),
                _ => {
                    println!("无效的选项，请重新输入。");
                    println!();
                }
            }
        }
    }

        /// Print help message
    fn print_help(&self) {
        println!("Loquat Plugin Generator");
        println!();
        println!("Usage: loquat plugin create [options] [name]");
        println!();
        println!("Options:");
        println!("  -n, --name <name>          Plugin name (required)");
        println!("  -t, --type <type>          Plugin type: rust, python, javascript (default: rust)");
        println!("  -a, --author <author>      Plugin author");
        println!("  -d, --description <desc>   Plugin description");
        println!("  -v, --version <version>     Plugin version (default: 0.1.0)");
        println!("  -o, --output <dir>         Output directory (default: ./plugins)");
        println!("  -h, --help                 Print this help message");
        println!();
        println!("Examples:");
        println!("  loquat plugin create my_plugin");
        println!("  loquat plugin create -n my_plugin -t python");
        println!("  loquat plugin create --name my_plugin --type javascript --author \"John Doe\"");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_language_from_str() {
        assert_eq!(PluginLanguage::from_str("rust"), Some(PluginLanguage::Rust));
        assert_eq!(PluginLanguage::from_str("python"), Some(PluginLanguage::Python));
        assert_eq!(PluginLanguage::from_str("javascript"), Some(PluginLanguage::JavaScript));
        assert_eq!(PluginLanguage::from_str("js"), Some(PluginLanguage::JavaScript));
        assert_eq!(PluginLanguage::from_str("unknown"), None);
    }

    #[test]
    fn test_struct_name_conversion() {
        let config = PluginTemplateConfig {
            name: "my_plugin".to_string(),
            ..Default::default()
        };
        assert_eq!(PluginTemplateGenerator::new(config).struct_name(), "MyPlugin");
    }

    #[test]
    fn test_rust_template_generation() {
        let mut config = PluginTemplateConfig::default();
        config.name = "test_plugin".to_string();
        config.plugin_type = PluginLanguage::Rust;
        
        let generator = PluginTemplateGenerator::new(config);
        let cargo_toml = generator.generate_rust_cargo_toml();
        
        assert!(cargo_toml.contains("name = \"test_plugin\""));
        assert!(cargo_toml.contains("crate-type = [\"cdylib\"]"));
    }
}
