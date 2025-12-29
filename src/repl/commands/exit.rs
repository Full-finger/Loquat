//! Exit command

use super::Command;
use crate::repl::context::ReplContext;
use crate::errors::Result;
use colored::Colorize;

/// Exit REPL
pub struct ExitCommand;

#[async_trait::async_trait]
impl Command for ExitCommand {
    fn name(&self) -> &str {
        "exit"
    }
    
    fn aliases(&self) -> Vec<&str> {
        vec!["quit", "q"]
    }
    
    fn description(&self) -> &str {
        "Exit REPL"
    }
    
    fn usage(&self) -> &str {
        "exit"
    }
    
    fn detailed_help(&self) -> &str {
        r#"Exit the REPL and return to the application.

This will exit the interactive REPL mode but keep the application running.
To completely stop the application, press Ctrl+C.

Example:
  exit

Aliases: quit, q
"#
    }
    
    async fn execute(&self, _args: &[String], _ctx: &ReplContext) -> Result<()> {
        println!();
        println!("{}", "Goodbye!".cyan());
        println!();
        Err(crate::errors::Error::Internal("Exit REPL".to_string()))
    }
}
