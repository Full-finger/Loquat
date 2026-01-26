//! Create new adapter or plugin

use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use crate::utils::{
    file_ops, code_parser, templates, helpers::{print_step, print_success, print_info, print_command_suggestion}
};

/// Create a new adapter
pub fn create_adapter(name: &str) -> Result<()> {
    print_step(&format!("Creating adapter '{}'", name));
    
    // Validate adapter name
    validate_name(name)?;
    
    // Check if adapter already exists
    if code_parser::adapter_source_exists(name) {
        return Err(anyhow::anyhow!("Adapter '{}' already exists in src/adapters/{}", name, name));
    }
    
    if code_parser::adapter_config_exists(name) {
        return Err(anyhow::anyhow!("Adapter config '{}' already exists in adapters/{}.json", name, name));
    }
    
    // Create directory structure
    let adapter_dir = format!("src/adapters/{}", name);
    file_ops::create_directory(&adapter_dir)?;
    print_success(&format!("Created directory: {}", adapter_dir));
    
    // Generate and write files
    let files = templates::adapter_files(name);
    for (path, content) in files {
        file_ops::write_file(&path, &content)?;
        print_success(&format!("Created file: {}", path));
    }
    
    // Update mod.rs
    code_parser::add_module_to_mod("src/adapters/mod.rs", name)?;
    print_success("Added module declaration to src/adapters/mod.rs");
    
    // Update main.rs
    code_parser::add_factory_to_main("src/main.rs", name)?;
    print_success("Added factory registration to src/main.rs");
    
    // Create config file
    let config = templates::adapter_config(name);
    file_ops::write_file(&format!("adapters/{}.json", name), &config)?;
    print_success(&format!("Created config file: adapters/{}.json", name));
    
    // Print summary
    println!();
    println!("{}", "═════════════════════════════════════════════════════════".cyan());
    print_success(&format!("Adapter '{}' created successfully!", name));
    println!("{}", "═════════════════════════════════════════════════════════".cyan());
    println!();
    print_info("Next steps:");
    println!();
    println!("  1. Implement your adapter:");
    println!("     Edit src/adapters/{}/adapter.rs", name);
    println!();
    println!("  2. Configure your adapter:");
    println!("     Edit adapters/{}.json", name);
    println!();
    println!("  3. Build the project:");
    print_command_suggestion("cargo build");
    println!();
    println!("  4. Run Loquat:");
    print_command_suggestion("cargo run");
    println!();
    print_info("Or use loquat-tool to run:");
    print_command_suggestion("loquat-tool run");
    println!();
    
    Ok(())
}

/// Create a new plugin
pub fn create_plugin(name: &str, plugin_type: &str) -> Result<()> {
    print_step(&format!("Creating {} plugin '{}'", plugin_type, name));
    
    // Validate plugin name
    validate_name(name)?;
    
    // Get project root first
    let project_root = file_ops::get_project_root()?;
    
    // Validate plugin type
    let plugin_type = match plugin_type.to_lowercase().as_str() {
        "rust" | "native" => {
            print_info("Creating Rust (native) plugin");
            templates::PluginType::Rust
        }
        "python" => {
            print_info("Creating Python plugin");
            templates::PluginType::Python
        }
        "javascript" | "js" => {
            print_info("Creating JavaScript plugin");
            templates::PluginType::JavaScript
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid plugin type '{}'. Supported types: rust, python, javascript",
                plugin_type
            ));
        }
    };
    
    // Check if plugin already exists
    let plugin_dir: PathBuf = project_root.join("plugins").join(name);
    if plugin_dir.exists() {
        return Err(anyhow::anyhow!("Plugin '{}' already exists", name));
    }
    
    // Generate files based on plugin type
    let files = match plugin_type {
        templates::PluginType::Rust => {
            // Create plugin directory structure for Rust
            let src_dir = plugin_dir.join("src");
            file_ops::create_directory_recursive(&src_dir)?;
            print_success(&format!("Created directory: {}", plugin_dir.display()));
            templates::plugin_files(name)
        }
        templates::PluginType::Python => {
            // Create plugin directory for Python
            file_ops::create_directory_recursive(&plugin_dir)?;
            print_success(&format!("Created directory: {}", plugin_dir.display()));
            templates::python_plugin_files(name)
        }
        templates::PluginType::JavaScript => {
            return Err(anyhow::anyhow!("JavaScript plugin support is not yet implemented"));
        }
    };
    
    // Generate and write files
    for (relative_path, content) in files {
        let full_path = plugin_dir.join(relative_path);
        let full_path_str = full_path.to_string_lossy().to_string();
        file_ops::write_file(&full_path_str, &content)?;
        print_success(&format!("Created file: {}", full_path_str));
    }
    
    // Print summary
    println!();
    println!("{}", "═════════════════════════════════════════════════════════".cyan());
    print_success(&format!("Plugin '{}' created successfully!", name));
    println!("{}", "═════════════════════════════════════════════════════════".cyan());
    println!();
    print_info("Next steps:");
    println!();
    
    match plugin_type {
        templates::PluginType::Rust => {
            println!("  1. Implement your plugin:");
            println!("     Edit plugins/{}/src/lib.rs", name);
            println!();
            println!("  2. Configure your plugin:");
            println!("     Edit plugins/{}/config.json", name);
            println!();
            println!("  3. Build the plugin:");
            print_command_suggestion(&format!("cd plugins/{} && cargo build --release", name));
        }
        templates::PluginType::Python => {
            println!("  1. Install Python dependencies:");
            print_command_suggestion(&format!("cd plugins/{} && pip install -r requirements.txt", name));
            println!();
            println!("  2. Implement your plugin:");
            println!("     Edit plugins/{}/plugin.py", name);
            println!();
            println!("  3. Configure your plugin:");
            println!("     Edit plugins/{}/config.json", name);
        }
        templates::PluginType::JavaScript => {
            // This should never be reached due to the error above
            unreachable!();
        }
    }
    
    println!();
    println!("  4. Run Loquat:");
    print_command_suggestion("cargo run");
    println!();
    
    Ok(())
}

/// Validate name (must be snake_case)
fn validate_name(name: &str) -> Result<()> {
    // Check if name is empty
    if name.is_empty() {
        return Err(anyhow::anyhow!("Name cannot be empty"));
    }
    
    // Check if name starts with a letter
    if !name.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false) {
        return Err(anyhow::anyhow!("Name must start with a letter"));
    }
    
    // Check if name contains only lowercase letters, numbers, and underscores
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(anyhow::anyhow!("Name must contain only lowercase letters, numbers, and underscores"));
    }
    
    // Check if name is a reserved keyword
    let reserved = ["mod", "fn", "struct", "enum", "impl", "trait", "type", "const", "static", "let", "mut", "ref", "unsafe", "extern", "crate", "super", "self"];
    if reserved.contains(&name) {
        return Err(anyhow::anyhow!("'{}' is a reserved keyword", name));
    }
    
    Ok(())
}
