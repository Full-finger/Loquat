//! REPL Engine - Core REPL implementation

use super::context::ReplContext;
use super::commands::CommandRegistry;
use super::commands::help::HelpCommand;
use super::commands::status::StatusCommand;
use super::commands::plugins::PluginsCommand;
use super::commands::adapters::AdaptersCommand;
use super::commands::reload::ReloadCommand;
use super::commands::logs::LogsCommand;
use super::commands::config::ConfigCommand;
use super::commands::engine::EngineCommand;
use super::commands::clear::ClearCommand;
use super::commands::exit::ExitCommand;
use super::prompt::generate_prompt;
use crate::errors::Result;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use colored::Colorize;
use std::sync::Arc;

/// REPL Engine configuration
#[derive(Debug, Clone)]
pub struct ReplEngineConfig {
    /// History file path
    pub history_path: Option<String>,
    /// Maximum history size
    pub history_max_size: usize,
    /// Enable auto-completion
    pub enable_completion: bool,
    /// Enable color output
    pub enable_color: bool,
}

impl Default for ReplEngineConfig {
    fn default() -> Self {
        Self {
            history_path: Some(".repl_history".to_string()),
            history_max_size: 1000,
            enable_completion: true,
            enable_color: true,
        }
    }
}

/// REPL Engine
pub struct ReplEngine {
    context: ReplContext,
    registry: CommandRegistry,
    config: ReplEngineConfig,
}

impl ReplEngine {
    /// Create a new REPL engine
    pub fn new(context: ReplContext) -> Self {
        Self {
            context,
            registry: CommandRegistry::new(),
            config: ReplEngineConfig::default(),
        }
    }
    
    /// Create a REPL engine with custom configuration
    pub fn with_config(context: ReplContext, config: ReplEngineConfig) -> Self {
        Self {
            context,
            registry: CommandRegistry::new(),
            config,
        }
    }
    
    /// Register all default commands
    pub fn register_default_commands(&mut self) {
        // Register help command
        let command_names = Arc::new(vec![
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
        ]);
        self.registry.register(Box::new(HelpCommand::new(command_names)));
        
        // Register other commands
        self.registry.register(Box::new(StatusCommand));
        self.registry.register(Box::new(PluginsCommand));
        self.registry.register(Box::new(AdaptersCommand));
        self.registry.register(Box::new(ReloadCommand));
        self.registry.register(Box::new(LogsCommand));
        self.registry.register(Box::new(ConfigCommand));
        self.registry.register(Box::new(EngineCommand));
        self.registry.register(Box::new(ClearCommand));
        self.registry.register(Box::new(ExitCommand));
    }
    
    /// Run the REPL
    pub async fn run(&mut self) -> Result<()> {
        self.print_banner();
        
        let mut editor = DefaultEditor::new()
            .map_err(|e| crate::errors::Error::Internal(format!("Failed to create REPL editor: {}", e)))?;
        
        // Load history if configured
        if let Some(ref history_path) = self.config.history_path {
            if let Err(e) = editor.load_history(history_path) {
                if !e.to_string().contains("No such file") {
                    eprintln!("Warning: Failed to load history: {}", e);
                }
            }
        }
        
        println!("{} Logs are being written to: {}",
            "Note:".yellow(),
            self.context.config.logging.file_path.cyan()
        );
        println!("{} Type 'help' for available commands.", 
            "Note:".yellow()
        );
        println!();
        
        // REPL loop
        loop {
            let prompt = generate_prompt(&self.context);
            
            let line = match editor.readline(&prompt) {
                Ok(line) => line,
                Err(ReadlineError::Interrupted) => {
                    // Ctrl+C - continue
                    println!();
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    // Ctrl+D - exit
                    println!();
                    println!("{}", "Goodbye!".cyan());
                    break;
                }
                Err(e) => {
                    eprintln!("Error reading input: {}", e);
                    continue;
                }
            };
            
            let line = line.trim();
            
            // Skip empty lines
            if line.is_empty() {
                continue;
            }
            
            // Add to history
            if let Err(e) = editor.add_history_entry(line) {
                // History is optional, don't fail REPL on error
                eprintln!("Warning: Failed to add to history: {}", e);
            }
            
            // Parse and execute command
            if let Err(e) = self.execute_command(line).await {
                let error_msg = e.to_string();
                if !error_msg.contains("Exit REPL") {
                    eprintln!("{}", format!("Error: {}", e).red());
                } else {
                    // Exit requested
                    break;
                }
            }
        }
        
        // Save history if configured
        if let Some(ref history_path) = self.config.history_path {
            if let Err(e) = editor.save_history(history_path) {
                eprintln!("Warning: Failed to save history: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Execute a command
    async fn execute_command(&self, input: &str) -> Result<()> {
        // Parse input into command and arguments
        let parts: Vec<&str> = input.split_whitespace().collect();
        
        if parts.is_empty() {
            return Ok(());
        }
        
        let command_name = parts[0];
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
        
        // Find and execute command
        if let Some(command) = self.registry.find(command_name) {
            command.execute(&args, &self.context).await
        } else {
            println!();
            println!("{}", format!("Unknown command: {}", command_name).red());
            println!("Type 'help' for available commands.");
            println!();
            Err(crate::errors::Error::Internal("Unknown command".to_string()))
        }
    }
    
    /// Print welcome banner
    fn print_banner(&self) {
        println!();
        println!("╔══════════════════════════════════════════════════════════╗");
        println!("║        Loquat Framework - Interactive Mode             ║");
        println!("╚══════════════════════════════════════════════════════════╝");
        println!();
        println!("Version: {}", env!("CARGO_PKG_VERSION").cyan());
        println!("Environment: {}", self.context.config.general.environment.cyan());
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repl_config_default() {
        let config = ReplEngineConfig::default();
        assert_eq!(config.history_max_size, 1000);
        assert!(config.enable_completion);
        assert!(config.enable_color);
    }
}
