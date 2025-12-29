//! Engine command

use super::Command;
use crate::repl::context::ReplContext;
use crate::errors::Result;
use colored::Colorize;
use std::sync::Arc;

pub struct EngineCommand;

#[async_trait::async_trait]
impl Command for EngineCommand {
    fn name(&self) -> &str {
        "engine"
    }
    
    fn aliases(&self) -> Vec<&str> {
        vec!["eng"]
    }
    
    fn description(&self) -> &str {
        "Control engine"
    }
    
    fn usage(&self) -> &str {
        "engine [start|stop|restart|status]"
    }
    
    fn detailed_help(&self) -> &str {
        r#"Control the Loquat engine.

Subcommands:
  engine start        Start the engine
  engine stop         Stop the engine
  engine restart      Restart the engine
  engine status       Show engine status

Examples:
  engine start         Start the engine
  engine stop          Stop the engine
  engine restart       Restart the engine

Note: The engine is automatically started on application launch.
"#
    }
    
    async fn execute(&self, args: &[String], ctx: &ReplContext) -> Result<()> {
        let engine = ctx.engine.as_ref()
            .ok_or_else(|| crate::errors::Error::Internal("Engine is not available".to_string()))?;
        
        if args.is_empty() {
            // Default: show status
            self.show_status(engine).await?;
            return Ok(());
        }
        
        match args[0].as_str() {
            "start" => {
                self.start(engine, ctx).await?;
            }
            "stop" => {
                self.stop(engine).await?;
            }
            "restart" => {
                self.restart(engine, ctx).await?;
            }
            "status" => {
                self.show_status(engine).await?;
            }
            _ => {
                println!();
                println!("{}", format!("Unknown engine command: {}", args[0]).red());
                println!("Valid commands: start, stop, restart, status");
                println!();
            }
        }
        
        Ok(())
    }
    
    fn complete(&self, args: &[String], ctx: &ReplContext) -> Vec<String> {
        let commands = vec!["start", "stop", "restart", "status"];
        
        if args.is_empty() || args.len() == 1 {
            let prefix = if args.is_empty() { "" } else { &args[0] };
            commands
                .into_iter()
                .filter(|c| c.starts_with(prefix))
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl EngineCommand {
    async fn start(&self, engine: &Arc<dyn crate::engine::traits::Engine>, _ctx: &ReplContext) -> Result<()> {
        println!();
        println!("{}", "Starting engine...".cyan());
        
        let state = engine.state();
        if state.status == crate::engine::types::EngineStatus::Running {
            println!("{}", "⊗ Engine is already running".yellow());
            println!();
            return Ok(());
        }
        
        // Note: Actual engine start would need to be implemented
        println!("{}", "Note: Engine start is handled automatically at startup".yellow());
        println!("Engine status: {:?}", state.status);
        println!();
        
        Ok(())
    }
    
    async fn stop(&self, _engine: &Arc<dyn crate::engine::traits::Engine>) -> Result<()> {
        println!();
        println!("{}", "Stopping engine...".cyan());
        
        // Note: Actual engine stop would need to be implemented
        println!("{}", "Note: Engine stop is not yet implemented".yellow());
        println!("To stop the application, use the 'exit' command");
        println!();
        
        Ok(())
    }
    
    async fn restart(&self, _engine: &Arc<dyn crate::engine::traits::Engine>, _ctx: &ReplContext) -> Result<()> {
        println!();
        println!("{}", "Restarting engine...".cyan());
        
        // Note: Actual engine restart would need to be implemented
        println!("{}", "Note: Engine restart is not yet implemented".yellow());
        println!("To restart the application, exit and run start.bat again");
        println!();
        
        Ok(())
    }
    
    async fn show_status(&self, engine: &Arc<dyn crate::engine::traits::Engine>) -> Result<()> {
        println!();
        println!("{}", "Engine Status".bold().underline());
        println!();
        
        let state = engine.state();
        let status_text = format!("{:?}", state.status);
        let status_colored = if state.status == crate::engine::types::EngineStatus::Running {
            status_text.green()
        } else if state.status == crate::engine::types::EngineStatus::Error {
            status_text.red()
        } else {
            status_text.yellow()
        };
        
        println!("  Status:      {}", status_colored);
        
        if let Some(ref error) = state.last_error {
            println!("  Last Error:  {}", error.red());
        } else {
            println!("  Last Error:  {}", "None".dimmed());
        }
        
        println!();
        
        Ok(())
    }
}
