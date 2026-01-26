//! Code parsing and modification utilities

use regex::Regex;
use std::fs;
use anyhow::{Context, Result};
use super::helpers::to_factory_name;

/// Add module declaration to mod.rs file
pub fn add_module_to_mod(mod_file: &str, module_name: &str) -> Result<()> {
    let content = fs::read_to_string(mod_file)
        .with_context(|| format!("Failed to read {}", mod_file))?;
    
    // Check if module already exists
    let existing = Regex::new(&format!(r#"pub\s+mod\s+{};"#, regex::escape(module_name)))?;
    if existing.is_match(&content) {
        return Err(anyhow::anyhow!("Module '{}' already exists in {}", module_name, mod_file));
    }
    
    // Add module declaration
    let new_content = format!(
        "{}\npub mod {};",
        content.trim_end(),
        module_name
    );
    
    fs::write(mod_file, new_content)?;
    Ok(())
}

/// Remove module declaration from mod.rs file
pub fn remove_module_from_mod(mod_file: &str, module_name: &str) -> Result<()> {
    let content = fs::read_to_string(mod_file)
        .with_context(|| format!("Failed to read {}", mod_file))?;
    
    // Remove module declaration
    let re = Regex::new(&format!(r#"pub\s+mod\s+{};"#, regex::escape(module_name)))?;
    let content = re.replace(&content, "").to_string();
    
    // Remove use statement
    let re = Regex::new(&format!(r#"pub\s+use\s+{}::\*;"#, regex::escape(module_name)))?;
    let content = re.replace(&content, "").to_string();
    
    // Clean up empty lines
    let content = cleanup_empty_lines(&content);
    
    fs::write(mod_file, content)?;
    Ok(())
}

/// Add factory to main.rs file
pub fn add_factory_to_main(main_file: &str, adapter_name: &str) -> Result<()> {
    let content = fs::read_to_string(main_file)
        .with_context(|| format!("Failed to read {}", main_file))?;
    let factory_name = to_factory_name(adapter_name);
    
    // Check if factory already imported
    let import_re = Regex::new(&format!(
        r#"use\s+loquat::adapters::{};"#,
        regex::escape(&factory_name)
    ))?;
    
    let content = if !import_re.is_match(&content) {
        // Look for existing adapters use statement
        let existing_import_re = Regex::new(r"use\s+loquat::adapters::\{([^}]*)\}")?;
        let content = if let Some(captures) = existing_import_re.captures(&content) {
            let existing = captures.get(1).unwrap().as_str();
            let new_import = format!("use loquat::adapters::{{{}, {}}};", existing, factory_name);
            existing_import_re.replace(&content, &new_import).to_string()
        } else {
            // Find use loquat::adapters; and replace with use loquat::adapters::{...}
            let re = Regex::new(r"use\s+loquat::adapters;")?;
            re.replace(&content, &format!("use loquat::adapters::{{{}}};", factory_name)).to_string()
        };
        content
    } else {
        content
    };
    
    // Add factory registration
    let registration_re = Regex::new(r"adapter_manager\.register_factory\([^)]*\)\?;")?;
    let registration = format!("adapter_manager.register_factory(Box::new({}))?;", factory_name);
    
    let content = if !registration_re.is_match(&content) {
        // Find the place where adapters are registered
        // Look for the pattern of existing adapter registrations
        let content = if content.contains("adapter_manager.register_factory") {
            // Append after the last registration
            format!("{}\n        {}", content.trim_end(), registration)
        } else {
            // First registration
            content + &format!("\n        {}", registration)
        };
        content
    } else {
        content
    };
    
    fs::write(main_file, content)?;
    Ok(())
}

/// Remove factory from main.rs file
pub fn remove_factory_from_main(main_file: &str, adapter_name: &str) -> Result<()> {
    let content = fs::read_to_string(main_file)
        .with_context(|| format!("Failed to read {}", main_file))?;
    let factory_name = to_factory_name(adapter_name);
    
    // Remove use statement
    let re = Regex::new(&format!(r#"use\s+loquat::adapters::{};"#, regex::escape(&factory_name)))?;
    let content = re.replace(&content, "").to_string();
    
    // Also try to remove from use loquat::adapters::{...} format
    let braces_re = Regex::new(&format!(r#"use\s+loquat::adapters::\{{([^}}]*)\s*{}([^}}]*)\}};"#, regex::escape(&factory_name)))?;
    let content = braces_re.replace(&content, "use loquat::adapters::{$1$2};").to_string();
    
    // Clean up trailing commas in use statement
    let comma_re = Regex::new(r",\s*\}")?;
    let content = comma_re.replace(&content, "}").to_string();
    
    // Remove factory registration
    let reg_re = Regex::new(&format!(
        r#"adapter_manager\.register_factory\(Box::new\({}\)\)\?;"#,
        regex::escape(&factory_name)
    ))?;
    let content = reg_re.replace(&content, "").to_string();
    
    // Also handle multiline registration
    let multiline_re = Regex::new(&format!(
        r"adapter_manager\.register_factory\(\s*Box::new\({}\)\s*\)\?;",
        regex::escape(&factory_name)
    ))?;
    let content = multiline_re.replace(&content, "").to_string();
    
    // Clean up empty lines
    let content = cleanup_empty_lines(&content);
    
    fs::write(main_file, content)?;
    Ok(())
}

/// Clean up excessive empty lines
fn cleanup_empty_lines(content: &str) -> String {
    let lines: Vec<&str> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    
    lines.join("\n")
}

/// Parse adapter config to get name
pub fn parse_adapter_config(config_path: &str) -> Result<String> {
    let content = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config: {}", config_path))?;
    
    let json: serde_json::Value = serde_json::from_str(&content)?;
    
    json.get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("No 'name' field found in config"))
}

/// Get list of adapters from adapters directory
pub fn list_adapters() -> Result<Vec<String>> {
    let adapters_dir = "adapters";
    if !std::path::Path::new(adapters_dir).exists() {
        return Ok(vec![]);
    }
    
    let entries = fs::read_dir(adapters_dir)
        .with_context(|| format!("Failed to read adapters directory"))?;
    
    let mut adapters = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "json" {
                    if let Some(name) = path.file_stem() {
                        adapters.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    
    adapters.sort();
    Ok(adapters)
}

/// Get list of plugins from plugins directory
pub fn list_plugins() -> Result<Vec<String>> {
    // Get project root
    let project_root = super::file_ops::get_project_root()?;
    let plugins_dir = project_root.join("plugins");
    
    if !plugins_dir.exists() {
        return Ok(vec![]);
    }
    
    let entries = fs::read_dir(&plugins_dir)
        .with_context(|| format!("Failed to read plugins directory"))?;
    
    let mut plugins = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            if let Some(name) = path.file_name() {
                plugins.push(name.to_string_lossy().to_string());
            }
        }
    }
    
    plugins.sort();
    Ok(plugins)
}

/// Check if adapter directory exists in src/adapters
pub fn adapter_source_exists(name: &str) -> bool {
    let path = format!("src/adapters/{}", name);
    std::path::Path::new(&path).is_dir()
}

/// Check if adapter config exists
pub fn adapter_config_exists(name: &str) -> bool {
    let path = format!("adapters/{}.json", name);
    std::path::Path::new(&path).is_file()
}
