//! Logs command

use super::Command;
use crate::repl::context::ReplContext;
use crate::errors::Result;
use colored::Colorize;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use chrono::{DateTime, Local};

pub struct LogsCommand;

#[async_trait::async_trait]
impl Command for LogsCommand {
    fn name(&self) -> &str {
        "logs"
    }
    
    fn aliases(&self) -> Vec<&str> {
        vec!["log"]
    }
    
    fn description(&self) -> &str {
        "View and follow logs"
    }
    
    fn usage(&self) -> &str {
        "logs [follow|clear] [lines]"
    }
    
    fn detailed_help(&self) -> &str {
        r#"View and follow application logs.

Subcommands:
  logs [lines]             Show the last N lines (default: 50)
  logs follow [lines]       Follow logs in real-time (like tail -f)
  logs clear               Clear the log file

Examples:
  logs                     Show last 50 log lines
  logs 100                 Show last 100 log lines
  logs follow               Follow logs in real-time
  logs follow 20            Follow and show last 20 lines
  logs clear               Clear the log file

Note: Press Ctrl+C to exit follow mode
"#
    }
    
    async fn execute(&self, args: &[String], ctx: &ReplContext) -> Result<()> {
        if args.is_empty() {
            // Show recent logs (default 50 lines)
            self.show_logs(50, ctx).await?;
            return Ok(());
        }
        
        match args[0].as_str() {
            "follow" | "f" => {
                let lines = if args.len() > 1 {
                    args[1].parse::<usize>().unwrap_or(50)
                } else {
                    50
                };
                self.follow_logs(lines, ctx).await?;
            }
            "clear" | "c" => {
                self.clear_logs(ctx).await?;
            }
            _ => {
                // Try to parse as number of lines
                if let Ok(lines) = args[0].parse::<usize>() {
                    self.show_logs(lines, ctx).await?;
                } else {
                    println!();
                    println!("{}", format!("Unknown logs command: {}", args[0]).red());
                    println!("Usage: logs [follow|clear] [lines]");
                    println!();
                }
            }
        }
        
        Ok(())
    }
}

impl LogsCommand {
    async fn show_logs(&self, lines: usize, ctx: &ReplContext) -> Result<()> {
        println!();
        println!("{}", format!("Showing last {} log lines...", lines).cyan());
        println!();
        
        let log_path = self.get_log_path(ctx);
        
        if !log_path.exists() {
            println!("{}", "Log file not found".yellow());
            println!("Path: {}", log_path.display());
            println!();
            return Ok(());
        }
        
        let file = File::open(&log_path)?;
        let reader = BufReader::new(file);
        let log_lines: Vec<String> = reader.lines()
            .filter_map(|l| l.ok())
            .collect();
        
        let start = if log_lines.len() > lines {
            log_lines.len() - lines
        } else {
            0
        };
        
        for line in &log_lines[start..] {
            self.print_log_line(line);
        }
        
        println!();
        println!("{}", format!("Total: {} lines shown", log_lines.len() - start).dimmed());
        println!();
        
        Ok(())
    }
    
    async fn follow_logs(&self, initial_lines: usize, ctx: &ReplContext) -> Result<()> {
        println!();
        println!("{}", format!("Following logs (initial: {} lines)...", initial_lines).cyan());
        println!("{}", "Press Ctrl+C to exit".dimmed());
        println!();
        
        let log_path = self.get_log_path(ctx);
        
        if !log_path.exists() {
            println!("{}", "Log file not found".yellow());
            println!("Path: {}", log_path.display());
            println!();
            return Ok(());
        }
        
        // Show initial lines first
        if initial_lines > 0 {
            let file = File::open(&log_path)?;
            let reader = BufReader::new(file);
            let log_lines: Vec<String> = reader.lines()
                .filter_map(|l| l.ok())
                .collect();
            
            let start = if log_lines.len() > initial_lines {
                log_lines.len() - initial_lines
            } else {
                0
            };
            
            for line in &log_lines[start..] {
                self.print_log_line(line);
            }
            
            println!("{}", "...".dimmed());
            println!();
        }
        
        // Set up Ctrl+C handler
        let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let running_clone = running.clone();
        
        ctrlc::set_handler(move || {
            running_clone.store(false, std::sync::atomic::Ordering::SeqCst);
        }).map_err(|e| crate::errors::Error::Internal(format!("Failed to set Ctrl+C handler: {}", e)))?;
        
        // Follow the log file
        let mut last_size = log_path.metadata()?.len();
        let mut last_modified = log_path.metadata()?.modified()?;
        
        while running.load(std::sync::atomic::Ordering::SeqCst) {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            // Check if file has been modified
            if let Ok(metadata) = log_path.metadata() {
                let current_modified = metadata.modified().unwrap_or(last_modified);
                
                if current_modified != last_modified {
                    last_modified = current_modified;
                    let current_size = metadata.len();
                    
                    if current_size > last_size {
                        // File has grown, read new lines
                        let file = File::open(&log_path)?;
                        let mut reader = BufReader::new(file);
                        
                        // Seek to last known position
                        use std::io::Seek;
                        let _ = reader.seek(io::SeekFrom::Start(last_size));
                        
                        // Read new lines
                        let new_lines: Vec<String> = reader.lines()
                            .filter_map(|l| l.ok())
                            .collect();
                        
                        for line in new_lines {
                            self.print_log_line(&line);
                        }
                        
                        last_size = current_size;
                    } else if current_size < last_size {
                        // File was truncated or rotated
                        println!("{}", "--- Log file was truncated/rotated ---".yellow());
                        last_size = current_size;
                    }
                }
            }
        }
        
        println!();
        println!("{}", "Stopped following logs".dimmed());
        println!();
        
        Ok(())
    }
    
    async fn clear_logs(&self, ctx: &ReplContext) -> Result<()> {
        println!();
        println!("{}", "Clearing log file...".cyan());
        
        let log_path = self.get_log_path(ctx);
        
        if log_path.exists() {
            std::fs::write(&log_path, "")?;
            println!("{}", "✓ Log file cleared".green());
        } else {
            println!("{}", "⊗ Log file does not exist".yellow());
        }
        
        println!();
        Ok(())
    }
    
    fn get_log_path(&self, ctx: &ReplContext) -> PathBuf {
        PathBuf::from(&ctx.config.logging.file_path)
    }
    
    fn print_log_line(&self, line: &str) {
        // Try to parse log level and colorize
        if line.contains("[ERROR]") {
            println!("{}", line.red());
        } else if line.contains("[WARN]") {
            println!("{}", line.yellow());
        } else if line.contains("[INFO]") {
            println!("{}", line);
        } else if line.contains("[DEBUG]") {
            println!("{}", line.dimmed());
        } else if line.contains("[TRACE]") {
            println!("{}", line.dimmed());
        } else {
            println!("{}", line);
        }
    }
}
