//! Remove command handler for adapters and plugins
//!
//! Provides CLI interface to remove adapters and plugins with safety checks

use std::io::{self, Write};

/// CLI handler for remove commands
pub struct RemoveCli;

impl RemoveCli {
    /// Run remove command from command-line arguments
    pub fn run_from_args(args: Vec<String>) -> Result<(), String> {
        if args.len() < 2 {
            return Err("Invalid remove command. Usage: remove <adapter|plugin> <name> [options]".to_string());
        }

        let target_type = &args[1];
        let target_name = if args.len() > 2 { &args[2] } else { "" };

        // Parse flags
        let mut force = false;
        let mut confirm = false;
        let mut clean_logs = false;
        let mut clean_db = false;
        let mut remove_all = false;

        let mut i = 3;
        while i < args.len() {
            match args[i].as_str() {
                "--force" => force = true,
                "--confirm" => confirm = true,
                "--clean-logs" => clean_logs = true,
                "--clean-db" => clean_db = true,
                "--all" => {
                    remove_all = true;
                    // --all requires specific handling for adapter/plugin
                }
                _ => {
                    // Treat as name if not a flag
                    if !args[i].starts_with("--") && target_name.is_empty() {
                        // This shouldn't happen with current parsing logic
                    }
                }
            }
            i += 1;
        }

        // Handle --all flag
        if remove_all {
            if target_name != "" && target_name != "--all" {
                return Err("--all cannot be used with a specific name".to_string());
            }

            // Require confirmation for --all
            if !confirm {
                Self::print_danger_warning(target_type);
                if !Self::prompt_confirmation() {
                    println!("Operation cancelled.");
                    return Ok(());
                }
            }

            return match target_type.as_str() {
                "adapters" => Self::remove_all_adapters(force, clean_logs, clean_db),
                "plugins" => Self::remove_all_plugins(force, clean_logs, clean_db),
                _ => Err(format!("Unknown target type for --all: {}. Use 'adapters' or 'plugins'.", target_type)),
            };
        }

        // Handle single item removal
        if target_name.is_empty() {
            return Err("Target name is required. Usage: remove <adapter|plugin> <name>".to_string());
        }

        // Check if it's a built-in adapter
        let is_built_in = match target_type.as_str() {
            "adapter" => Self::is_built_in_adapter(target_name),
            "plugin" => false,
            _ => return Err(format!("Unknown target type: {}. Use 'adapter' or 'plugin'.", target_type)),
        };

        // Require --force for built-in adapters
        if is_built_in && !force {
            Self::print_builtin_warning(target_name);
            println!("Use --force to delete built-in adapters.");
            return Ok(()); // Not an error, just don't delete
        }

        match target_type.as_str() {
            "adapter" => Self::remove_adapter(target_name, clean_logs, clean_db),
            "plugin" => Self::remove_plugin(target_name, clean_logs, clean_db),
            _ => Err(format!("Unknown target type: {}. Use 'adapter' or 'plugin'.", target_type)),
        }
    }

    /// Remove a single adapter
    fn remove_adapter(name: &str, clean_logs: bool, clean_db: bool) -> Result<(), String> {
        println!();
        println!("Removing adapter: {}", name);

        // Check if adapter exists
        let adapter_dir = format!("src/adapters/{}", name);
        let adapter_config = format!("adapters/{}.json", name);

        if !std::path::Path::new(&adapter_dir).exists()
            && !std::path::Path::new(&adapter_config).exists()
        {
            return Err(format!("Adapter '{}' not found.", name));
        }

        // Show what will be removed
        println!("The following will be removed:");
        if std::path::Path::new(&adapter_dir).exists() {
            println!("  - Source code: {}", adapter_dir);
        }
        if std::path::Path::new(&adapter_config).exists() {
            println!("  - Configuration: {}", adapter_config);
        }
        if clean_logs {
            println!("  - Related log files (if any)");
        }
        if clean_db {
            println!("  - Database records (if any)");
        }

        // Confirm removal
        if !Self::prompt_confirmation() {
            println!("Operation cancelled.");
            return Ok(());
        }

        // Remove source directory
        if std::path::Path::new(&adapter_dir).exists() {
            std::fs::remove_dir_all(&adapter_dir)
                .map_err(|e| format!("Failed to remove directory '{}': {}", adapter_dir, e))?;
            println!("✓ Removed: {}", adapter_dir);
        }

        // Remove config file
        if std::path::Path::new(&adapter_config).exists() {
            std::fs::remove_file(&adapter_config)
                .map_err(|e| format!("Failed to remove config '{}': {}", adapter_config, e))?;
            println!("✓ Removed: {}", adapter_config);
        }

        // Clean logs if requested
        if clean_logs {
            Self::clean_adapter_logs(name)?;
        }

        // Clean database if requested
        if clean_db {
            Self::clean_adapter_db(name)?;
        }

        // Note: Manual cleanup of module declarations and factory registration is required
        println!();
        println!("⚠️  MANUAL CLEANUP REQUIRED:");
        println!("   1. Edit src/adapters/mod.rs");
        println!("   2. Remove: pub mod {};", name);
        println!("   3. Remove: pub use {}::*;", name);
        println!("   4. Edit src/main.rs");
        println!("   5. Remove factory registration for {}", name);
        println!();

        println!("✓ Adapter '{}' removed successfully.", name);
        Ok(())
    }

    /// Remove a single plugin
    fn remove_plugin(name: &str, clean_logs: bool, clean_db: bool) -> Result<(), String> {
        println!();
        println!("Removing plugin: {}", name);

        // Check if plugin exists
        let plugin_dir = format!("plugins/{}", name);

        if !std::path::Path::new(&plugin_dir).exists() {
            return Err(format!("Plugin '{}' not found.", name));
        }

        // Show what will be removed
        println!("The following will be removed:");
        println!("  - Plugin directory: {}", plugin_dir);
        if clean_logs {
            println!("  - Related log files (if any)");
        }
        if clean_db {
            println!("  - Database records (if any)");
        }

        // Confirm removal
        if !Self::prompt_confirmation() {
            println!("Operation cancelled.");
            return Ok(());
        }

        // Remove plugin directory
        std::fs::remove_dir_all(&plugin_dir)
            .map_err(|e| format!("Failed to remove directory '{}': {}", plugin_dir, e))?;
        println!("✓ Removed: {}", plugin_dir);

        // Clean logs if requested
        if clean_logs {
            Self::clean_plugin_logs(name)?;
        }

        // Clean database if requested
        if clean_db {
            Self::clean_plugin_db(name)?;
        }

        println!("✓ Plugin '{}' removed successfully.", name);
        Ok(())
    }

    /// Remove all adapters
    fn remove_all_adapters(force: bool, clean_logs: bool, clean_db: bool) -> Result<(), String> {
        println!();
        println!("Removing ALL adapters...");

        // List adapters
        let adapters_dir = std::path::Path::new("src/adapters");
        let mut adapter_names = Vec::new();

        if let Ok(entries) = std::fs::read_dir(adapters_dir) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if entry.path().is_dir() && name != "core" && name != "actor" && name != "utils" {
                        adapter_names.push(name);
                    }
                }
            }
        }

        if adapter_names.is_empty() {
            println!("No adapters found to remove.");
            return Ok(());
        }

        println!("Found {} adapter(s):", adapter_names.len());
        for name in &adapter_names {
            println!("  - {}", name);
        }
        println!();

        // Check for built-in adapters
        let built_ins: Vec<_> = adapter_names.iter()
            .filter(|n| Self::is_built_in_adapter(n))
            .cloned()
            .collect();

        if !built_ins.is_empty() && !force {
            println!("⚠️  The following built-in adapters require --force:");
            for name in &built_ins {
                println!("  - {}", name);
            }
            println!();
            println!("Use --force to remove built-in adapters.");
            println!();
            if !Self::prompt_confirmation() {
                println!("Operation cancelled.");
                return Ok(());
            }
        }

        // Remove each adapter
        let mut removed_count = 0;
        let mut failed_count = 0;
        for name in &adapter_names {
            match Self::remove_adapter(name, clean_logs, clean_db) {
                Ok(_) => removed_count += 1,
                Err(e) => {
                    eprintln!("Failed to remove '{}': {}", name, e);
                    failed_count += 1;
                }
            }
        }

        println!();
        println!("Summary:");
        println!("  Removed: {}", removed_count);
        if failed_count > 0 {
            println!("  Failed: {}", failed_count);
        }

        Ok(())
    }

    /// Remove all plugins
    fn remove_all_plugins(force: bool, clean_logs: bool, clean_db: bool) -> Result<(), String> {
        println!();
        println!("Removing ALL plugins...");

        // List plugins
        let plugins_dir = std::path::Path::new("plugins");
        let mut plugin_names = Vec::new();

        if let Ok(entries) = std::fs::read_dir(plugins_dir) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if entry.path().is_dir() {
                        plugin_names.push(name);
                    }
                }
            }
        }

        if plugin_names.is_empty() {
            println!("No plugins found to remove.");
            return Ok(());
        }

        println!("Found {} plugin(s):", plugin_names.len());
        for name in &plugin_names {
            println!("  - {}", name);
        }
        println!();

        // Confirm removal
        if !Self::prompt_confirmation() {
            println!("Operation cancelled.");
            return Ok(());
        }

        // Remove each plugin
        let mut removed_count = 0;
        let mut failed_count = 0;
        for name in &plugin_names {
            match Self::remove_plugin(name, clean_logs, clean_db) {
                Ok(_) => removed_count += 1,
                Err(e) => {
                    eprintln!("Failed to remove '{}': {}", name, e);
                    failed_count += 1;
                }
            }
        }

        println!();
        println!("Summary:");
        println!("  Removed: {}", removed_count);
        if failed_count > 0 {
            println!("  Failed: {}", failed_count);
        }

        Ok(())
    }

    /// Check if adapter is built-in
    fn is_built_in_adapter(name: &str) -> bool {
        matches!(name, "console" | "echo" | "mock_test" | "napcat")
    }

    /// Print warning for built-in adapter
    fn print_builtin_warning(name: &str) {
        println!();
        println!("⚠️  WARNING: '{}' is a built-in adapter.", name);
        println!("   Built-in adapters are part of the framework.");
        println!("   Deleting them may affect framework functionality.");
        println!();
    }

    /// Print danger warning for --all operations
    fn print_danger_warning(target_type: &str) {
        println!();
        println!("⚠️  ══════════════════════════════════════════════╗");
        println!("⚠️  │            DANGER: MASS DELETION               │");
        println!("⚠️  ══════════════════════════════════════════════╝");
        println!();
        println!("You are about to delete ALL {}!", target_type);
        println!("This operation cannot be undone!");
        println!();
    }

    /// Prompt user for confirmation
    fn prompt_confirmation() -> bool {
        print!("Are you sure? [y/N]: ");
        io::stdout().flush().unwrap_or(());

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap_or(0);

        let input = input.trim().to_lowercase();
        matches!(input.as_str(), "y" | "yes")
    }

    /// Clean adapter logs (stub implementation)
    fn clean_adapter_logs(_name: &str) -> Result<(), String> {
        // In a real implementation, you would search for and delete log files
        // related to this adapter
        println!("✓ Cleaned adapter logs (stub)");
        Ok(())
    }

    /// Clean adapter database records (stub implementation)
    fn clean_adapter_db(_name: &str) -> Result<(), String> {
        // In a real implementation, you would delete database records
        // related to this adapter
        println!("✓ Cleaned adapter database records (stub)");
        Ok(())
    }

    /// Clean plugin logs (stub implementation)
    fn clean_plugin_logs(_name: &str) -> Result<(), String> {
        // In a real implementation, you would search for and delete log files
        // related to this plugin
        println!("✓ Cleaned plugin logs (stub)");
        Ok(())
    }

    /// Clean plugin database records (stub implementation)
    fn clean_plugin_db(_name: &str) -> Result<(), String> {
        // In a real implementation, you would delete database records
        // related to this plugin
        println!("✓ Cleaned plugin database records (stub)");
        Ok(())
    }
}
