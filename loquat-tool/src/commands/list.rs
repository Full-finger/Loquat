//! List adapters or plugins

use anyhow::Result;
use colored::Colorize;
use crate::utils::{
    code_parser, helpers::{print_step, print_info}
};

/// List all adapters
pub fn list_adapters() -> Result<()> {
    print_step("Listing adapters");
    
    let adapters = code_parser::list_adapters()?;
    
    if adapters.is_empty() {
        println!();
        print_info("No adapters found");
        println!();
        return Ok(());
    }
    
    println!();
    println!("{}", "Installed Adapters:".cyan().bold());
    println!();
    
    for (i, adapter) in adapters.iter().enumerate() {
        let source_exists = code_parser::adapter_source_exists(adapter);
        let config_exists = code_parser::adapter_config_exists(adapter);
        
        let status = if source_exists && config_exists {
            "✓".green().to_string()
        } else if source_exists {
            "⚠ Source only".yellow().to_string()
        } else if config_exists {
            "⚠ Config only".yellow().to_string()
        } else {
            "✗".red().to_string()
        };
        
        println!("  {}. {} {}", i + 1, adapter.cyan(), status);
        
        // Show adapter details if config exists
        if config_exists {
            let config_path = format!("adapters/{}.json", adapter);
            if let Ok(name) = code_parser::parse_adapter_config(&config_path) {
                println!("     Name: {}", name.bright_white());
            }
        }
    }
    
    println!();
    println!("Total: {} adapter(s)", adapters.len());
    println!();
    
    Ok(())
}

/// List all plugins
pub fn list_plugins() -> Result<()> {
    print_step("Listing plugins");
    
    let plugins = code_parser::list_plugins()?;
    
    if plugins.is_empty() {
        println!();
        print_info("No plugins found");
        println!();
        return Ok(());
    }
    
    println!();
    println!("{}", "Installed Plugins:".cyan().bold());
    println!();
    
    for (i, plugin) in plugins.iter().enumerate() {
        println!("  {}. {}", i + 1, plugin.cyan());
        
        // Try to read plugin config
        let config_path = format!("plugins/{}/config.json", plugin);
        if std::path::Path::new(&config_path).exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
                        println!("     Name: {}", name.bright_white());
                    }
                    if let Some(version) = json.get("version").and_then(|v| v.as_str()) {
                        println!("     Version: {}", version.bright_white());
                    }
                    if let Some(description) = json.get("description").and_then(|v| v.as_str()) {
                        println!("     Description: {}", description.dimmed());
                    }
                }
            }
        }
    }
    
    println!();
    println!("Total: {} plugin(s)", plugins.len());
    println!();
    
    Ok(())
}
