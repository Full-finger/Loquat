//! Commands for REPL

use async_trait::async_trait;

pub mod help;
pub mod status;
pub mod plugins;
pub mod adapters;
pub mod reload;
pub mod logs;
pub mod config;
pub mod engine;
pub mod clear;
pub mod exit;

use crate::repl::context::ReplContext;
use crate::errors::Result;

/// Command trait
#[async_trait]
pub trait Command: Send + Sync {
    /// Get command name
    fn name(&self) -> &str;
    
    /// Get command aliases
    fn aliases(&self) -> Vec<&str>;
    
    /// Get command description
    fn description(&self) -> &str;
    
    /// Get command usage
    fn usage(&self) -> &str;
    
    /// Execute the command
    async fn execute(&self, args: &[String], ctx: &ReplContext) -> Result<()>;
    
    /// Get auto-completion suggestions
    fn complete(&self, args: &[String], ctx: &ReplContext) -> Vec<String> {
        Vec::new()
    }
    
    /// Show detailed help for this command
    fn help(&self) -> String {
        format!(
            "Usage: {}\n\nDescription: {}\n\n{}",
            self.usage(),
            self.description(),
            self.detailed_help()
        )
    }
    
    /// Get detailed help text
    fn detailed_help(&self) -> &str {
        "No additional help available."
    }
}

/// Command registry
pub struct CommandRegistry {
    commands: Vec<Box<dyn Command>>,
}

impl std::fmt::Debug for CommandRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandRegistry")
            .field("command_count", &self.commands.len())
            .finish()
    }
}

impl CommandRegistry {
    /// Create a new command registry
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }
    
    /// Register a command
    pub fn register(&mut self, command: Box<dyn Command>) {
        self.commands.push(command);
    }
    
    /// Find a command by name or alias
    pub fn find(&self, name: &str) -> Option<&dyn Command> {
        for cmd in &self.commands {
            if cmd.name() == name || cmd.aliases().contains(&name) {
                return Some(cmd.as_ref());
            }
        }
        None
    }
    
    /// Get all command names
    pub fn all_names(&self) -> Vec<String> {
        self.commands
            .iter()
            .flat_map(|cmd| {
                let mut names = vec![cmd.name().to_string()];
                names.extend(cmd.aliases().iter().map(|s| s.to_string()));
                names
            })
            .collect()
    }
    
    /// Get completion suggestions
    pub fn complete(&self, input: &str, ctx: &ReplContext) -> Vec<String> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        
        if parts.is_empty() {
            // Complete command name
            return self.commands
                .iter()
                .map(|cmd| cmd.name().to_string())
                .collect();
        }
        
        if parts.len() == 1 {
            // Complete command name
            return self.commands
                .iter()
                .filter_map(|cmd| {
                    let name = cmd.name();
                    if name.starts_with(parts[0]) {
                        Some(name.to_string())
                    } else {
                        None
                    }
                })
                .collect();
        }
        
        // Complete arguments
        if let Some(cmd) = self.find(parts[0]) {
            let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
            let suggestions = cmd.complete(&args, ctx);
            if suggestions.is_empty() {
                return Vec::new();
            }
            // Prepend the command to each suggestion
            suggestions
                .into_iter()
                .map(|s| format!("{} {}", parts[0], s))
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}
