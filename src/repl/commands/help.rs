//! Help command

use super::Command;
use crate::repl::context::ReplContext;
use crate::errors::Result;
use colored::Colorize;
use std::sync::Arc;

pub struct HelpCommand {
    command_names: Arc<Vec<String>>,
}

impl HelpCommand {
    pub fn new(command_names: Arc<Vec<String>>) -> Self {
        Self { command_names }
    }
}

#[async_trait::async_trait]
impl Command for HelpCommand {
    fn name(&self) -> &str {
        "help"
    }
    
    fn aliases(&self) -> Vec<&str> {
        vec!["h", "?"]
    }
    
    fn description(&self) -> &str {
        "Show help information"
    }
    
    fn usage(&self) -> &str {
        "help [command]"
    }
    
    fn detailed_help(&self) -> &str {
        r#"Show help information for all commands or a specific command.

Examples:
  help              Show help for all commands
  help status       Show detailed help for the status command
  help reload       Show detailed help for the reload command

Aliases: h, ?
"#
    }
    
    async fn execute(&self, args: &[String], ctx: &ReplContext) -> Result<()> {
        // Note: This is a simplified help command
        // For a full implementation, we'd need access to the CommandRegistry
        // from the context or a different approach
        
        if args.is_empty() {
            // Show basic help
            println!();
            println!("{}", "Available Commands:".bold().underline());
            println!();
            println!("  help          - Show help information");
            println!("  status        - Show system status");
            println!("  plugins       - Plugin management");
            println!("  adapters      - Adapter management");
            println!("  reload        - Reload components");
            println!("  logs          - View logs");
            println!("  config        - Show configuration");
            println!("  engine        - Engine control");
            println!("  clear         - Clear screen");
            println!("  exit          - Exit REPL");
            println!();
            println!("{}", "Usage: help [command]".yellow());
            println!("Use 'help <command>' for more information about a specific command.");
            println!();
        } else {
            // Show help for specific command (simplified)
            println!();
            println!("{}", format!("Help for command: {}", args[0]).bold());
            println!();
            println!("Detailed help is not yet implemented.");
            println!("Type 'help' to see all available commands.");
            println!();
        }
        
        Ok(())
    }
    
    fn complete(&self, args: &[String], _ctx: &ReplContext) -> Vec<String> {
        let commands = vec![
            "help".to_string(),
            "status".to_string(),
            "plugins".to_string(),
            "adapters".to_string(),
            "reload".to_string(),
            "logs".to_string(),
            "config".to_string(),
            "engine".to_string(),
            "clear".to_string(),
            "exit".to_string(),
        ];
        
        if args.is_empty() || args.len() == 1 {
            let prefix = if args.len() == 1 { &args[0] } else { "" };
            
            commands
                .into_iter()
                .filter(|name| name.starts_with(prefix))
                .collect()
        } else {
            Vec::new()
        }
    }
}
