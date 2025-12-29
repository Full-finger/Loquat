//! Plugins command

use super::Command;
use crate::repl::context::ReplContext;
use crate::errors::Result;
use colored::Colorize;

pub struct PluginsCommand;

#[async_trait::async_trait]
impl Command for PluginsCommand {
    fn name(&self) -> &str {
        "plugins"
    }
    
    fn aliases(&self) -> Vec<&str> {
        vec!["pl", "plugin"]
    }
    
    fn description(&self) -> &str {
        "Plugin management"
    }
    
    fn usage(&self) -> &str {
        "plugins [list|info|load|unload|reload] [name]"
    }
    
    fn detailed_help(&self) -> &str {
        r#"Manage plugins in the Loquat framework.

Subcommands:
  plugins list [filter]        List all plugins (optionally filter by name)
  plugins info <name>         Show detailed information about a plugin
  plugins load <name>          Load a specific plugin
  plugins unload <name>        Unload a specific plugin
  plugins reload <name>        Reload a specific plugin

Examples:
  plugins                      List all plugins
  plugins list                 Same as above
  plugins info console_adapter  Show info about console_adapter
  plugins reload example_plugin Reload a specific plugin

Aliases: pl, plugin
"#
    }
    
    async fn execute(&self, args: &[String], ctx: &ReplContext) -> Result<()> {
        let plugin_manager = ctx.plugin_manager.as_ref()
            .ok_or_else(|| crate::errors::Error::Internal("Plugin system is not enabled".to_string()))?;
        
        if args.is_empty() {
            // Default: list all plugins
            self.list_plugins(None, plugin_manager).await?;
            return Ok(());
        }
        
        match args[0].as_str() {
            "list" | "ls" => {
                let filter = if args.len() > 1 { Some(args[1].as_str()) } else { None };
                self.list_plugins(filter, plugin_manager).await?;
            }
            "info" => {
                if args.len() < 2 {
                    println!();
                    println!("{}", "Error: Missing plugin name".red());
                    println!("Usage: plugins info <name>");
                    println!();
                    return Ok(());
                }
                self.show_plugin_info(&args[1], plugin_manager).await?;
            }
            "load" => {
                if args.len() < 2 {
                    println!();
                    println!("{}", "Error: Missing plugin name".red());
                    println!("Usage: plugins load <name>");
                    println!();
                    return Ok(());
                }
                self.load_plugin(&args[1], plugin_manager, ctx).await?;
            }
            "unload" => {
                if args.len() < 2 {
                    println!();
                    println!("{}", "Error: Missing plugin name".red());
                    println!("Usage: plugins unload <name>");
                    println!();
                    return Ok(());
                }
                self.unload_plugin(&args[1], plugin_manager, ctx).await?;
            }
            "reload" => {
                if args.len() < 2 {
                    println!();
                    println!("{}", "Error: Missing plugin name".red());
                    println!("Usage: plugins reload <name>");
                    println!();
                    return Ok(());
                }
                self.reload_plugin(&args[1], plugin_manager, ctx).await?;
            }
            _ => {
                // Treat as filter or show help
                self.list_plugins(Some(&args[0]), plugin_manager).await?;
            }
        }
        
        Ok(())
    }
    
    fn complete(&self, args: &[String], ctx: &ReplContext) -> Vec<String> {
        let plugin_manager = match &ctx.plugin_manager {
            Some(pm) => pm,
            None => return Vec::new(),
        };
        
        // Note: We can't use async in complete, so we'll return subcommands only
        let subcommands: Vec<String> = vec!["list", "ls", "info", "load", "unload", "reload"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let plugins: Vec<String> = Vec::new();
        
        if args.is_empty() {
            subcommands
        } else if args.len() == 1 {
            let prefix = &args[0];
            let prefix_str = prefix.as_str();
            subcommands
                .into_iter()
                .filter(|s| s.starts_with(prefix_str))
                .collect()
        } else {
            match args[0].as_str() {
                "info" | "load" | "unload" | "reload" if args.len() == 2 => {
                    // No plugin suggestions in complete
                    Vec::new()
                }
                _ => Vec::new()
            }
        }
    }
}

impl PluginsCommand {
    async fn list_plugins(&self, filter: Option<&str>, plugin_manager: &crate::plugins::PluginManager) -> Result<()> {
        let plugins = plugin_manager.list_plugin_infos();
        
        println!();
        
        let filtered_plugins: Vec<_> = if let Some(filter) = filter {
            plugins.iter()
                .filter(|p| p.metadata.name.contains(filter))
                .collect()
        } else {
            plugins.iter().collect()
        };
        
        if filtered_plugins.is_empty() {
            println!("{}", "No plugins found".yellow());
            println!();
            return Ok(());
        }
        
        // Print header
        let header = format!("{:<20} {:<15} {:<15} {:<20}", 
            "Name", "Type", "Status", "Version");
        println!("{}", header.bold().underline());
        
        for plugin in &filtered_plugins {
            let status_text = format!("{:?}", plugin.status);
            let status_colored = match plugin.status {
                crate::plugins::types::PluginStatus::Loaded => status_text.green(),
                crate::plugins::types::PluginStatus::Error { .. } => status_text.red(),
                _ => status_text.yellow(),
            };
            
            println!("{:<20} {:<15} {:<15} {:<20}",
                plugin.metadata.name.cyan(),
                format!("{:?}", plugin.metadata.plugin_type),
                status_colored,
                plugin.metadata.version,
            );
        }
        
        println!();
        println!("Total: {} plugin(s)", filtered_plugins.len());
        println!();
        
        Ok(())
    }
    
    async fn show_plugin_info(&self, name: &str, plugin_manager: &crate::plugins::PluginManager) -> Result<()> {
        let plugins = plugin_manager.list_plugin_infos();
        
        if let Some(plugin) = plugins.iter().find(|p| p.metadata.name == name) {
            println!();
            println!("{}", "Plugin Information".bold().underline());
            println!();
            println!("  Name:        {}", plugin.metadata.name.cyan());
            println!("  Version:     {}", plugin.metadata.version);
            println!("  Type:        {:?}", plugin.metadata.plugin_type);
            println!("  Author:      {}", plugin.metadata.author.as_ref().map(|s| s.as_str()).unwrap_or("Unknown"));
            println!("  Description: {}", plugin.metadata.description.as_ref().map(|s| s.as_str()).unwrap_or("No description"));
            
            let status_text = format!("{:?}", plugin.status);
            let status_colored = match plugin.status {
                crate::plugins::types::PluginStatus::Loaded => status_text.green(),
                crate::plugins::types::PluginStatus::Error { .. } => status_text.red(),
                _ => status_text.yellow(),
            };
            println!("  Status:      {}", status_colored);
            println!();
        } else {
            println!();
            println!("{}", format!("Plugin '{}' not found", name).red());
            println!();
        }
        
        Ok(())
    }
    
    async fn load_plugin(&self, name: &str, plugin_manager: &crate::plugins::PluginManager, ctx: &ReplContext) -> Result<()> {
        println!();
        println!("{}", format!("Loading plugin '{}'...", name).cyan());
        
        // Note: This is a placeholder - actual implementation depends on PluginManager API
        println!("{}", "Note: Plugin loading is currently automatic on startup".yellow());
        println!("Use 'reload plugins' to reload all plugins");
        println!();
        
        Ok(())
    }
    
    async fn unload_plugin(&self, name: &str, plugin_manager: &crate::plugins::PluginManager, ctx: &ReplContext) -> Result<()> {
        println!();
        println!("{}", format!("Unloading plugin '{}'...", name).cyan());
        
        // Note: This is a placeholder - actual implementation depends on PluginManager API
        println!("{}", "Note: Plugin unloading is not yet implemented".yellow());
        println!();
        
        Ok(())
    }
    
    async fn reload_plugin(&self, name: &str, plugin_manager: &crate::plugins::PluginManager, ctx: &ReplContext) -> Result<()> {
        println!();
        println!("{}", format!("Reloading plugin '{}'...", name).cyan());
        
        match plugin_manager.auto_load_plugins().await {
            Ok(results) => {
                let reloaded = results.iter().filter(|r| r.success).count();
                let failed = results.len() - reloaded;
                
                if reloaded > 0 {
                    println!("{}", format!("✓ Plugin reloaded successfully ({} plugins total)", reloaded).green());
                } else {
                    println!("{}", "✗ No plugins were reloaded".yellow());
                }
                
                if failed > 0 {
                    println!("{}", format!("✗ {} plugins failed to reload", failed).red());
                }
            }
            Err(e) => {
                println!("{}", format!("✗ Failed to reload plugin: {}", e).red());
            }
        }
        
        println!();
        Ok(())
    }
}
