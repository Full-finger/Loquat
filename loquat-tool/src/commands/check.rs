//! Check project for errors

use anyhow::Result;
use colored::Colorize;
use crate::utils::{
    code_parser, file_ops, helpers::{print_step, print_success, print_warning, print_info}
};

/// Check project for errors
pub fn check_project() -> Result<()> {
    print_step("Checking Loquat project");
    println!();
    
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    
    // Check if we're in a Loquat project
    if !file_ops::is_loquat_project() {
        errors.push("Not in a Loquat project directory".to_string());
    } else {
        print_success("✓ Loquat project detected");
    }
    
    // Check Cargo.toml exists
    if !file_ops::file_exists("Cargo.toml") {
        errors.push("Cargo.toml not found".to_string());
    } else {
        print_success("✓ Cargo.toml exists");
    }
    
    // Check src directory exists
    if !file_ops::directory_exists("src") {
        errors.push("src directory not found".to_string());
    } else {
        print_success("✓ src directory exists");
    }
    
    // Check adapters directory
    if !file_ops::directory_exists("adapters") {
        warnings.push("adapters directory not found (you can create adapters with 'loquat-tool new adapter')".to_string());
    } else {
        print_success("✓ adapters directory exists");
    }
    
    // Check plugins directory
    if !file_ops::directory_exists("plugins") {
        warnings.push("plugins directory not found (you can create plugins with 'loquat-tool new plugin')".to_string());
    } else {
        print_success("✓ plugins directory exists");
    }
    
    // Check config directory
    if !file_ops::directory_exists("config") {
        errors.push("config directory not found".to_string());
    } else {
        print_success("✓ config directory exists");
        
        // Check for default config
        if !file_ops::file_exists("config/default.toml") {
            warnings.push("config/default.toml not found".to_string());
        } else {
            print_success("✓ config/default.toml exists");
        }
    }
    
    println!();
    print_step("Checking adapters");
    println!();
    
    // Check adapter consistency
    let adapters = code_parser::list_adapters()?;
    
    if adapters.is_empty() {
        print_info("No adapters found");
    } else {
        println!("Found {} adapter(s):", adapters.len());
        
        for adapter in &adapters {
            let source_exists = code_parser::adapter_source_exists(adapter);
            let config_exists = code_parser::adapter_config_exists(adapter);
            
            if source_exists && config_exists {
                print_success(&format!("✓ Adapter '{}' is complete", adapter));
            } else if source_exists {
                warnings.push(format!("Adapter '{}' has source but no config file", adapter));
                print_warning(&format!("⚠ Adapter '{}' missing config file", adapter));
            } else if config_exists {
                warnings.push(format!("Adapter '{}' has config but no source", adapter));
                print_warning(&format!("⚠ Adapter '{}' missing source code", adapter));
            }
        }
    }
    
    println!();
    print_step("Checking plugins");
    println!();
    
    // Get project root
    let project_root = file_ops::get_project_root()?;
    
    // Check plugins
    let plugins = code_parser::list_plugins()?;
    
    if plugins.is_empty() {
        print_info("No plugins found");
    } else {
        println!("Found {} plugin(s):", plugins.len());
        
        for plugin in &plugins {
        let plugin_dir = project_root.join("plugins").join(plugin);
        
        if plugin_dir.exists() {
            // Determine plugin type from config if available
            let config_path = plugin_dir.join("config.json");
            let config_path_str = config_path.to_string_lossy().to_string();
            let is_python = if std::path::Path::new(&config_path).exists() {
                if let Ok(content) = std::fs::read_to_string(&config_path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        json.get("plugin_type")
                            .and_then(|v| v.as_str())
                            .map(|s| s == "python")
                            .unwrap_or(false)
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };
            
            let _is_complete = if is_python {
                // Python plugin checks
                let init_path = plugin_dir.join("__init__.py");
                let plugin_path = plugin_dir.join("plugin.py");
                let requirements_path = plugin_dir.join("requirements.txt");
                let pyproject_path = plugin_dir.join("pyproject.toml");
                
                let has_init = init_path.exists();
                let has_plugin = plugin_path.exists();
                let has_requirements = requirements_path.exists();
                let has_pyproject = pyproject_path.exists();
                let has_config = std::path::Path::new(&config_path).exists();
                
                if has_init && has_plugin && has_requirements && has_pyproject && has_config {
                    print_success(&format!("✓ Plugin '{}' is complete (Python)", plugin));
                    true
                } else {
                    warnings.push(format!("Plugin '{}' (Python) is incomplete", plugin));
                    if !has_init {
                        print_warning(&format!("⚠ Plugin '{}' missing __init__.py", plugin));
                    }
                    if !has_plugin {
                        print_warning(&format!("⚠ Plugin '{}' missing plugin.py", plugin));
                    }
                    if !has_requirements {
                        print_warning(&format!("⚠ Plugin '{}' missing requirements.txt", plugin));
                    }
                    if !has_pyproject {
                        print_warning(&format!("⚠ Plugin '{}' missing pyproject.toml", plugin));
                    }
                    if !has_config {
                        print_warning(&format!("⚠ Plugin '{}' missing config.json", plugin));
                    }
                    false
                }
            } else {
                // Rust plugin checks (default)
                let lib_path = plugin_dir.join("src").join("lib.rs");
                let cargo_path = plugin_dir.join("Cargo.toml");
                
                let has_lib = lib_path.exists();
                let has_cargo = cargo_path.exists();
                let has_config = std::path::Path::new(&config_path).exists();
                
                if has_lib && has_cargo && has_config {
                    print_success(&format!("✓ Plugin '{}' is complete (Rust)", plugin));
                    true
                } else {
                    warnings.push(format!("Plugin '{}' (Rust) is incomplete", plugin));
                    if !has_lib {
                        print_warning(&format!("⚠ Plugin '{}' missing src/lib.rs", plugin));
                    }
                    if !has_cargo {
                        print_warning(&format!("⚠ Plugin '{}' missing Cargo.toml", plugin));
                    }
                    if !has_config {
                        print_warning(&format!("⚠ Plugin '{}' missing config.json", plugin));
                    }
                    false
                }
            };
        }
    }
    }
    
    // Print summary
    println!();
    println!("{}", "═════════════════════════════════════════════════════════".cyan());
    print_info("Check Summary");
    println!("{}", "═════════════════════════════════════════════════════════".cyan());
    println!();
    
    if errors.is_empty() {
        print_success(&format!("No errors found!"));
    } else {
        println!("Errors ({}):", errors.len().to_string().red());
        for error in &errors {
            println!("  ✗ {}", error.red());
        }
    }
    
    println!();
    
    if warnings.is_empty() {
        print_success(&format!("No warnings!"));
    } else {
        println!("Warnings ({}):", warnings.len().to_string().yellow());
        for warning in &warnings {
            println!("  ⚠ {}", warning.yellow());
        }
    }
    
    println!();
    
    if errors.is_empty() && warnings.is_empty() {
        print_success("Your Loquat project looks great!");
    } else if errors.is_empty() {
        print_info("Your Loquat project is mostly healthy, but has some warnings.");
    } else {
        print_warning("Your Loquat project has errors that need to be fixed.");
    }
    
    println!();
    
    Ok(())
}
