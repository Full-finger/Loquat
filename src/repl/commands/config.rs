//! Config command

use super::Command;
use crate::repl::context::ReplContext;
use crate::errors::Result;
use colored::Colorize;

pub struct ConfigCommand;

#[async_trait::async_trait]
impl Command for ConfigCommand {
    fn name(&self) -> &str {
        "config"
    }
    
    fn aliases(&self) -> Vec<&str> {
        vec!["cfg"]
    }
    
    fn description(&self) -> &str {
        "Show configuration"
    }
    
    fn usage(&self) -> &str {
        "config"
    }
    
    fn detailed_help(&self) -> &str {
        r#"Display the current Loquat framework configuration.

Shows:
- General settings (name, environment)
- Logging settings (level, format, output)
- Plugin settings (enabled, auto_load, hot_reload)
- Adapter settings (enabled, auto_load, hot_reload)
- Web settings (enabled, host, port)

Example:
  config

Note: This only shows the configuration, not modify it.
To modify configuration, edit the config files in the config/ directory.
"#
    }
    
    async fn execute(&self, args: &[String], ctx: &ReplContext) -> Result<()> {
        println!();
        println!("{}", "Current Configuration".bold().underline());
        println!();
        
        // General
        println!("{} General", "─".repeat(50).cyan());
        println!("  Name:        {}", ctx.config.general.name);
        println!("  Environment: {}", ctx.config.general.environment.cyan());
        println!();
        
        // Logging
        println!("{} Logging", "─".repeat(50).cyan());
        println!("  Level:       {}", ctx.config.logging.level);
        println!("  Format:      {}", ctx.config.logging.format);
        println!("  Output:      {}", ctx.config.logging.output);
        println!("  File Path:   {}", ctx.config.logging.file_path);
        println!();
        
        // Plugins
        println!("{} Plugins", "─".repeat(50).cyan());
        println!("  Enabled:     {}", if ctx.config.plugins.enabled { 
            "Yes".green() 
        } else { 
            "No".red() 
        });
        println!("  Auto Load:   {}", if ctx.config.plugins.auto_load { 
            "Yes".green() 
        } else { 
            "No".red() 
        });
        println!("  Hot Reload:  {}", if ctx.config.plugins.enable_hot_reload { 
            format!("Yes ({}s)", ctx.config.plugins.hot_reload_interval).green()
        } else { 
            "No".red() 
        });
        println!();
        
        // Adapters
        println!("{} Adapters", "─".repeat(50).cyan());
        println!("  Enabled:     {}", if ctx.config.adapters.enabled { 
            "Yes".green() 
        } else { 
            "No".red() 
        });
        println!("  Auto Load:   {}", if ctx.config.adapters.auto_load { 
            "Yes".green() 
        } else { 
            "No".red() 
        });
        println!("  Hot Reload:  {}", if ctx.config.adapters.enable_hot_reload { 
            format!("Yes ({}s)", ctx.config.adapters.hot_reload_interval).green()
        } else { 
            "No".red() 
        });
        println!();
        
        // Web
        println!("{} Web Service", "─".repeat(50).cyan());
        println!("  Enabled:     {}", if ctx.config.web.enabled { 
            "Yes".green() 
        } else { 
            "No".red() 
        });
        println!("  Host:        {}", ctx.config.web.host);
        println!("  Port:        {}", ctx.config.web.port);
        println!();
        
        Ok(())
    }
}
