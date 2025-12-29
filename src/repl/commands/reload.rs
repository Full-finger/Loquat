//! Reload command

use super::Command;
use crate::repl::context::ReplContext;
use crate::errors::Result;
use colored::Colorize;

pub struct ReloadCommand;

#[async_trait::async_trait]
impl Command for ReloadCommand {
    fn name(&self) -> &str {
        "reload"
    }
    
    fn aliases(&self) -> Vec<&str> {
        vec!["r"]
    }
    
    fn description(&self) -> &str {
        "Reload configuration/plugins/adapters"
    }
    
    fn usage(&self) -> &str {
        "reload [all|plugins|adapters|config]"
    }
    
    fn detailed_help(&self) -> &str {
        r#"Reload various components of the Loquat framework.

Subcommands:
  reload all        Reload everything (config, plugins, and adapters)
  reload plugins    Reload all plugins
  reload adapters   Reload all adapters
  reload config     Reload configuration file

Examples:
  reload             Reload everything (same as 'reload all')
  reload plugins     Reload only plugins
  reload adapters    Reload only adapters

Aliases: r
"#
    }
    
    async fn execute(&self, args: &[String], ctx: &ReplContext) -> Result<()> {
        let target = if args.is_empty() {
            "all"
        } else {
            args[0].as_str()
        };
        
        match target {
            "all" => {
                self.reload_all(ctx).await?;
            }
            "plugins" | "plugin" | "pl" => {
                self.reload_plugins(ctx).await?;
            }
            "adapters" | "adapter" | "ad" => {
                self.reload_adapters(ctx).await?;
            }
            "config" | "configuration" => {
                self.reload_config(ctx).await?;
            }
            _ => {
                println!();
                println!("{}", format!("Unknown reload target: {}", target).red());
                println!("Valid targets: all, plugins, adapters, config");
                println!();
            }
        }
        
        Ok(())
    }
    
    fn complete(&self, args: &[String], ctx: &ReplContext) -> Vec<String> {
        let targets = vec!["all", "plugins", "adapters", "config"];
        
        if args.is_empty() || args.len() == 1 {
            let prefix = if args.is_empty() { "" } else { &args[0] };
            targets
                .into_iter()
                .filter(|t| t.starts_with(prefix))
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl ReloadCommand {
    async fn reload_all(&self, ctx: &ReplContext) -> Result<()> {
        println!();
        println!("{}", "Reloading everything...".cyan());
        println!();
        
        let mut plugins_reloaded = 0u32;
        let mut adapters_reloaded = 0u32;
        let mut errors = Vec::new();
        
        // Reload plugins
        if let Some(plugin_manager) = &ctx.plugin_manager {
            println!("{}", "Reloading plugins...".yellow());
            match plugin_manager.auto_load_plugins().await {
                Ok(results) => {
                    plugins_reloaded = results.iter().filter(|r| r.success).count() as u32;
                    let failed = results.iter().filter(|r| !r.success).count();
                    
                    if plugins_reloaded > 0 {
                        println!("{}", format!("  ✓ {} plugins reloaded", plugins_reloaded).green());
                    }
                    if failed > 0 {
                        errors.push(format!("  ✗ {} plugins failed to reload", failed));
                        println!("{}", errors.last().unwrap().red());
                    }
                }
                Err(e) => {
                    errors.push(format!("  ✗ Failed to reload plugins: {}", e));
                    println!("{}", errors.last().unwrap().red());
                }
            }
        } else {
            println!("{}", "  ⊗ Plugin system is disabled".yellow());
        }
        
        println!();
        
        // Reload adapters
        if let Some(adapter_manager) = &ctx.adapter_manager {
            println!("{}", "Reloading adapters...".yellow());
            match adapter_manager.auto_load_adapters().await {
                Ok(results) => {
                    adapters_reloaded = results.iter().filter(|r| r.success).count() as u32;
                    let failed = results.iter().filter(|r| !r.success).count();
                    
                    if adapters_reloaded > 0 {
                        println!("{}", format!("  ✓ {} adapters reloaded", adapters_reloaded).green());
                    }
                    if failed > 0 {
                        errors.push(format!("  ✗ {} adapters failed to reload", failed));
                        println!("{}", errors.last().unwrap().red());
                    }
                }
                Err(e) => {
                    errors.push(format!("  ✗ Failed to reload adapters: {}", e));
                    println!("{}", errors.last().unwrap().red());
                }
            }
        } else {
            println!("{}", "  ⊗ Adapter system is disabled".yellow());
        }
        
        println!();
        println!("{}", "Reloading configuration...".yellow());
        // Note: Config reload would need to be implemented
        println!("{}", "  ⊗ Config reload not yet implemented".yellow());
        
        println!();
        
        // Summary
        if errors.is_empty() {
            println!("{}", format!("✓ Reload completed successfully: {} plugins, {} adapters", 
                plugins_reloaded, adapters_reloaded).green());
        } else {
            println!("{}", format!("⚠ Reload completed with {} error(s)", errors.len()).yellow());
        }
        
        println!();
        Ok(())
    }
    
    async fn reload_plugins(&self, ctx: &ReplContext) -> Result<()> {
        println!();
        println!("{}", "Reloading plugins...".cyan());
        
        let plugin_manager = ctx.plugin_manager.as_ref()
            .ok_or_else(|| crate::errors::Error::Internal("Plugin system is not enabled".to_string()))?;
        
        match plugin_manager.auto_load_plugins().await {
            Ok(results) => {
                let reloaded = results.iter().filter(|r| r.success).count();
                let failed = results.len() - reloaded;
                
                println!();
                if reloaded > 0 {
                    println!("{}", format!("✓ {} plugins reloaded successfully", reloaded).green());
                } else {
                    println!("{}", "✗ No plugins were reloaded".yellow());
                }
                
                if failed > 0 {
                    println!("{}", format!("✗ {} plugins failed to reload", failed).red());
                }
            }
            Err(e) => {
                println!();
                println!("{}", format!("✗ Failed to reload plugins: {}", e).red());
            }
        }
        
        println!();
        Ok(())
    }
    
    async fn reload_adapters(&self, ctx: &ReplContext) -> Result<()> {
        println!();
        println!("{}", "Reloading adapters...".cyan());
        
        let adapter_manager = ctx.adapter_manager.as_ref()
            .ok_or_else(|| crate::errors::Error::Internal("Adapter system is not enabled".to_string()))?;
        
        match adapter_manager.auto_load_adapters().await {
            Ok(results) => {
                let reloaded = results.iter().filter(|r| r.success).count();
                let failed = results.len() - reloaded;
                
                println!();
                if reloaded > 0 {
                    println!("{}", format!("✓ {} adapters reloaded successfully", reloaded).green());
                } else {
                    println!("{}", "✗ No adapters were reloaded".yellow());
                }
                
                if failed > 0 {
                    println!("{}", format!("✗ {} adapters failed to reload", failed).red());
                }
            }
            Err(e) => {
                println!();
                println!("{}", format!("✗ Failed to reload adapters: {}", e).red());
            }
        }
        
        println!();
        Ok(())
    }
    
    async fn reload_config(&self, ctx: &ReplContext) -> Result<()> {
        println!();
        println!("{}", "Reloading configuration...".cyan());
        println!();
        println!("{}", "Note: Configuration reload is not yet implemented".yellow());
        println!("You need to restart the application to apply configuration changes.");
        println!();
        Ok(())
    }
}
