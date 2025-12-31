//! Status command

use super::Command;
use crate::repl::context::ReplContext;
use crate::errors::Result;
use colored::Colorize;
use std::fmt::Write as FmtWrite;

pub struct StatusCommand;

#[async_trait::async_trait]
impl Command for StatusCommand {
    fn name(&self) -> &str {
        "status"
    }
    
    fn aliases(&self) -> Vec<&str> {
        vec!["st", "info"]
    }
    
    fn description(&self) -> &str {
        "Display system status"
    }
    
    fn usage(&self) -> &str {
        "status"
    }
    
    fn detailed_help(&self) -> &str {
        r#"Display the current status of the Loquat framework including:
- Engine state
- Uptime
- Environment
- Plugin status
- Adapter status

Example:
  status
"#
    }
    
    async fn execute(&self, args: &[String], ctx: &ReplContext) -> Result<()> {
        println!();
        
        // Print header
        let separator = "─".repeat(58);
        println!("┌{}┐", separator);
        println!("│{:^58}│", "System Status".bold());
        println!("├{}┤", separator);
        println!();
        
        // Engine status
        if let Some(engine) = &ctx.engine {
            let state = engine.state().await;
            let status_text = format!("{:?}", state.status);
            let status_colored = if state.status == crate::engine::types::EngineStatus::Running {
                status_text.green()
            } else if state.status == crate::engine::types::EngineStatus::Error {
                status_text.red()
            } else {
                status_text.yellow()
            };
            
            println!("  Engine:      {}", status_colored);
        } else {
            println!("  Engine:      {}", "Not available".red());
        }
        
        // Uptime
        let uptime = ctx.start_time.elapsed();
        let uptime_str = format_duration(uptime);
        println!("  Uptime:      {}", uptime_str);
        
        // Environment
        println!("  Environment: {}", ctx.config.general.environment.cyan());
        println!();
        
        // Plugin status
        if let Some(plugin_manager) = &ctx.plugin_manager {
            let plugins = plugin_manager.list_plugin_infos();
            let active = plugins.iter().filter(|p| matches!(p.status, crate::plugins::types::PluginStatus::Loaded)).count();
            let errors = plugins.iter().filter(|p| {
                matches!(p.status, crate::plugins::types::PluginStatus::Error { .. })
            }).count();
            let total = plugins.len();
            
            println!("  Plugins:     {}", format!("{} loaded", total).cyan());
            println!("    ├─ Active:  {}", active.to_string().green());
            println!("    ├─ Errors:  {}", if errors > 0 { errors.to_string().red() } else { "0".to_string().green() });
            
            if !plugins.is_empty() {
                println!("    └─ Latest:");
                for plugin in plugins.iter().take(5) {
                    let status_symbol = match plugin.status {
                        crate::plugins::types::PluginStatus::Loaded => "✓".green(),
                        crate::plugins::types::PluginStatus::Error { .. } => "✗".red(),
                        _ => "○".yellow(),
                    };
                    println!("       {} {} (v{}) - {}", 
                        status_symbol, 
                        plugin.metadata.name, 
                        plugin.metadata.version,
                        format!("{:?}", plugin.status)
                    );
                }
                if plugins.len() > 5 {
                    println!("       ... and {} more", plugins.len() - 5);
                }
            }
            println!();
        } else {
            println!("  Plugins:     {}", "Disabled".yellow());
            println!();
        }
        
        // Adapter status
        if let Some(adapter_manager) = &ctx.adapter_manager {
            let adapters = adapter_manager.list_adapter_infos().await;
            if !adapters.is_empty() {
                let active = adapters.iter().filter(|a| a.status.is_active()).count();
                let errors = adapters.iter().filter(|a| a.status.is_error()).count();
                let total = adapters.len();
                
                println!("  Adapters:    {}", format!("{} loaded", total).cyan());
                println!("    ├─ Active:  {}", active.to_string().green());
                println!("    ├─ Errors:  {}", if errors > 0 { errors.to_string().red() } else { "0".to_string().green() });
                
                if !adapters.is_empty() {
                    println!("    └─ Latest:");
                    for adapter in adapters.iter().take(5) {
                        let status_symbol = if adapter.status.is_active() {
                            "✓".green()
                        } else if adapter.status.is_error() {
                            "✗".red()
                        } else {
                            "○".yellow()
                        };
                        println!("       {} {} (v{}) - {}", 
                            status_symbol, 
                            adapter.adapter_id, 
                            adapter.version,
                            format!("{:?}", adapter.status)
                        );
                    }
                    if adapters.len() > 5 {
                        println!("       ... and {} more", adapters.len() - 5);
                    }
                }
            } else {
                println!("  Adapters:    {}", "Error loading adapters".red());
            }
            println!();
        } else {
            println!("  Adapters:    {}", "Disabled".yellow());
            println!();
        }
        
        // Logging status
        println!("  Logging:     {}", format!("{} | {} | {}", 
            ctx.config.logging.level,
            ctx.config.logging.format,
            ctx.config.logging.output
        ).cyan());
        
        println!();
        println!("└{}┘", separator);
        println!();
        
        Ok(())
    }
}

/// Format duration as human-readable string
fn format_duration(duration: std::time::Duration) -> String {
    let total_secs = duration.as_secs();
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    
    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, seconds)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}
