//! Clear command

use super::Command;
use crate::repl::context::ReplContext;
use crate::errors::Result;
use colored::Colorize;

pub struct ClearCommand;

#[async_trait::async_trait]
impl Command for ClearCommand {
    fn name(&self) -> &str {
        "clear"
    }
    
    fn aliases(&self) -> Vec<&str> {
        vec!["cls", "c"]
    }
    
    fn description(&self) -> &str {
        "Clear the screen"
    }
    
    fn usage(&self) -> &str {
        "clear"
    }
    
    fn detailed_help(&self) -> &str {
        r#"Clear the terminal screen.

This is equivalent to the 'cls' command on Windows or 'clear' on Unix/Linux.

Example:
  clear

Aliases: cls, c
"#
    }
    
    async fn execute(&self, _args: &[String], _ctx: &ReplContext) -> Result<()> {
        println!("\x1b[2J\x1b[H");
        Ok(())
    }
}
