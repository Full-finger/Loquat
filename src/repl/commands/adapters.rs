//! Adapters command

use super::Command;
use crate::repl::context::ReplContext;
use crate::errors::Result;
use colored::Colorize;

pub struct AdaptersCommand;

#[async_trait::async_trait]
impl Command for AdaptersCommand {
    fn name(&self) -> &str {
        "adapters"
    }
    
    fn aliases(&self) -> Vec<&str> {
        vec!["ad", "adapter"]
    }
    
    fn description(&self) -> &str {
        "Adapter management"
    }
    
    fn usage(&self) -> &str {
        "adapters [list|info|reload] [name]"
    }
    
    fn detailed_help(&self) -> &str {
        r#"Manage adapters in Loquat framework.

Subcommands:
  adapters list [filter]        List all adapters (optionally filter by name)
  adapters info <name>         Show detailed information about an adapter
  adapters reload <name>       Reload a specific adapter

Examples:
  adapters                      List all adapters
  adapters list                 Same as above
  adapters info console         Show info about console adapter
  adapters reload console       Reload a specific adapter

Aliases: ad, adapter
"#
    }
    
    async fn execute(&self, args: &[String], ctx: &ReplContext) -> Result<()> {
        let adapter_manager = ctx.adapter_manager.as_ref()
            .ok_or_else(|| crate::errors::Error::Internal("Adapter system is not enabled".to_string()))?;
        
        if args.is_empty() {
            // Default: list all adapters
            self.list_adapters(None, adapter_manager).await?;
            return Ok(());
        }
        
        match args[0].as_str() {
            "list" | "ls" => {
                let filter = if args.len() > 1 { Some(args[1].as_str()) } else { None };
                self.list_adapters(filter, adapter_manager).await?;
            }
            "info" => {
                if args.len() < 2 {
                    println!();
                    println!("{}", "Error: Missing adapter name".red());
                    println!("Usage: adapters info <name>");
                    println!();
                    return Ok(());
                }
                self.show_adapter_info(&args[1], adapter_manager).await?;
            }
            "reload" => {
                if args.len() < 2 {
                    println!();
                    println!("{}", "Error: Missing adapter name".red());
                    println!("Usage: adapters reload <name>");
                    println!();
                    return Ok(());
                }
                self.reload_adapters(adapter_manager, ctx).await?;
            }
            _ => {
                // Treat as filter or show help
                self.list_adapters(Some(&args[0]), adapter_manager).await?;
            }
        }
        
        Ok(())
    }
    
    fn complete(&self, args: &[String], ctx: &ReplContext) -> Vec<String> {
        let adapter_manager = match &ctx.adapter_manager {
            Some(am) => am,
            None => return Vec::new(),
        };
        
        let subcommands: Vec<String> = vec!["list", "ls", "info", "reload"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        // Note: We can't use async in complete, so we'll return subcommands only
        let adapters: Vec<String> = Vec::new();
        
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
                "info" if args.len() == 2 => {
                    // No adapter suggestions in complete
                    Vec::new()
                }
                _ => Vec::new()
            }
        }
    }
}

impl AdaptersCommand {
    async fn list_adapters(&self, filter: Option<&str>, adapter_manager: &crate::adapters::AdapterManager) -> Result<()> {
        let adapters = adapter_manager.list_adapter_infos().await;
        
        println!();
        
        let filtered_adapters: Vec<_> = if let Some(filter) = filter {
            adapters.iter()
                .filter(|a| a.adapter_id.contains(filter))
                .collect()
        } else {
            adapters.iter().collect()
        };
        
        if filtered_adapters.is_empty() {
            println!("{}", "No adapters found".yellow());
            println!();
            return Ok(());
        }
        
        // Print header
        let header = format!("{:<20} {:<15} {:<20}", 
            "Name", "Status", "Version");
        println!("{}", header.bold().underline());
        
        for adapter in &filtered_adapters {
            let status_text = format!("{:?}", adapter.status);
            let status_colored = if adapter.status.is_active() {
                status_text.green()
            } else if adapter.status.is_error() {
                status_text.red()
            } else {
                status_text.yellow()
            };
            
            println!("{:<20} {:<15} {:<20}",
                adapter.adapter_id.cyan(),
                status_colored,
                adapter.version,
            );
        }
        
        println!();
        println!("Total: {} adapter(s)", filtered_adapters.len());
        println!();
        
        Ok(())
    }
    
    async fn show_adapter_info(&self, name: &str, adapter_manager: &crate::adapters::AdapterManager) -> Result<()> {
        let adapters = adapter_manager.list_adapter_infos().await;
        
        if let Some(adapter) = adapters.iter().find(|a| a.adapter_id == name) {
            println!();
            println!("{}", "Adapter Information".bold().underline());
            println!();
            println!("  Name:    {}", adapter.adapter_id.cyan());
            println!("  Version: {}", adapter.version);
            
            let status_text = format!("{:?}", adapter.status);
            let status_colored = if adapter.status.is_active() {
                status_text.green()
            } else if adapter.status.is_error() {
                status_text.red()
            } else {
                status_text.yellow()
            };
            println!("  Status:  {}", status_colored);
            println!();
        } else {
            println!();
            println!("{}", format!("Adapter '{}' not found", name).red());
            println!();
        }
        
        Ok(())
    }
    
    async fn reload_adapters(&self, adapter_manager: &crate::adapters::AdapterManager, ctx: &ReplContext) -> Result<()> {
        println!();
        println!("{}", "Reloading adapters...".cyan());
        
        match adapter_manager.auto_load_adapters().await {
            Ok(results) => {
                let reloaded = results.iter().filter(|r| r.success).count();
                let failed = results.len() - reloaded;
                
                if reloaded > 0 {
                    println!("{}", format!("✓ Adapters reloaded successfully ({} adapters total)", reloaded).green());
                } else {
                    println!("{}", "✗ No adapters were reloaded".yellow());
                }
                
                if failed > 0 {
                    println!("{}", format!("✗ {} adapters failed to reload", failed).red());
                }
            }
            Err(e) => {
                println!("{}", format!("✗ Failed to reload adapters: {}", e).red());
            }
        }
        
        println!();
        Ok(())
    }
}
