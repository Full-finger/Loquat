//! Remove adapter or plugin

use anyhow::Result;
use colored::Colorize;
use crate::utils::{
    file_ops, code_parser, helpers::{
        print_step, print_success, print_warning, 
        print_info, print_command_suggestion, prompt_confirmation
    }
};

/// Remove an adapter
pub fn remove_adapter(name: &str, force: bool) -> Result<()> {
    print_step(&format!("Removing adapter '{}'", name));
    
    // Check if adapter exists
    let source_exists = code_parser::adapter_source_exists(name);
    let config_exists = code_parser::adapter_config_exists(name);
    
    if !source_exists && !config_exists {
        print_warning(&format!("Adapter '{}' does not exist", name));
        return Ok(());
    }
    
    // Confirm deletion
    if !force {
        let message = format!("Are you sure you want to remove adapter '{}'?", name);
        if !prompt_confirmation(&message)? {
            println!("Operation cancelled.");
            return Ok(());
        }
    }
    
    // Remove source directory
    let adapter_dir = format!("src/adapters/{}", name);
    if source_exists {
        file_ops::remove_directory(&adapter_dir)?;
        print_success(&format!("Removed directory: {}", adapter_dir));
    }
    
    // Remove config file
    let config_file = format!("adapters/{}.json", name);
    if config_exists {
        file_ops::remove_file(&config_file)?;
        print_success(&format!("Removed config: {}", config_file));
    }
    
    // Clean up mod.rs
    print_info("Auto-cleaning module declarations...");
    code_parser::remove_module_from_mod("src/adapters/mod.rs", name)?;
    print_success("Removed module declaration from src/adapters/mod.rs");
    
    // Clean up main.rs
    print_info("Auto-cleaning factory registration...");
    code_parser::remove_factory_from_main("src/main.rs", name)?;
    print_success("Removed factory registration from src/main.rs");
    
    // Print summary
    println!();
    println!("{}", "═════════════════════════════════════════════════════════".cyan());
    print_success(&format!("Adapter '{}' removed successfully!", name));
    println!("{}", "═════════════════════════════════════════════════════════".cyan());
    println!();
    print_info("Next steps:");
    println!();
    println!("  1. Rebuild the project:");
    print_command_suggestion("cargo build");
    println!();
    println!("  2. Run Loquat:");
    print_command_suggestion("cargo run");
    println!();
    print_info("Or use loquat-tool to run:");
    print_command_suggestion("loquat-tool run");
    println!();
    
    Ok(())
}

/// Remove a plugin
pub fn remove_plugin(name: &str, force: bool) -> Result<()> {
    print_step(&format!("Removing plugin '{}'", name));
    
    // Get project root to locate plugins directory
    let project_root = file_ops::get_project_root()?;
    
    // Check if plugin exists
    let plugin_dir = project_root.join("plugins").join(name);
    let plugin_dir_str = plugin_dir.to_string_lossy().to_string();
    
    if !file_ops::directory_exists(&plugin_dir_str) {
        print_warning(&format!("Plugin '{}' does not exist", name));
        return Ok(());
    }
    
    // Confirm deletion
    if !force {
        let message = format!("Are you sure you want to remove plugin '{}'?", name);
        if !prompt_confirmation(&message)? {
            println!("Operation cancelled.");
            return Ok(());
        }
    }
    
    // Remove plugin directory
    file_ops::remove_directory(&plugin_dir_str)?;
    print_success(&format!("Removed directory: {}", plugin_dir_str));
    
    // Print summary
    println!();
    println!("{}", "═════════════════════════════════════════════════════════".cyan());
    print_success(&format!("Plugin '{}' removed successfully!", name));
    println!("{}", "═════════════════════════════════════════════════════════".cyan());
    println!();
    print_info("Next steps:");
    println!();
    println!("  1. Rebuild the project:");
    print_command_suggestion("cargo build");
    println!();
    println!("  2. Run Loquat:");
    print_command_suggestion("cargo run");
    println!();
    
    Ok(())
}
