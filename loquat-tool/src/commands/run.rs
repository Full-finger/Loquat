//! Run Loquat framework

use anyhow::Result;
use colored::Colorize;
use crate::utils::{
    file_ops, helpers::{print_step, print_info, print_command_suggestion}
};
use std::process::Command;

/// Run Loquat framework
pub fn run_loquat(env: String, repl: bool, tui: bool) -> Result<()> {
    print_step("Starting Loquat framework");
    
    // Check if we're in a Loquat project
    if !file_ops::is_loquat_project() {
        return Err(anyhow::anyhow!(
            "Not in a Loquat project directory. Please run this command from a Loquat project root."
        ));
    }
    
    print_info(&format!("Environment: {}", env.bright_white()));
    
    // Build the command
    let mut cmd = Command::new("cargo");
    cmd.arg("run");
    cmd.arg("--").arg(format!("--env={}", env));
    
    if repl {
        cmd.arg("--repl");
        print_info("Mode: REPL");
    } else if tui {
        cmd.arg("--tui");
        print_info("Mode: TUI");
    } else {
        print_info("Mode: Standard");
    }
    
    println!();
    print_info("Starting Loquat...");
    println!();
    println!("{}", "─".repeat(60).cyan());
    println!();
    
    // Run the command
    let mut child = cmd.spawn()?;
    let status = child.wait()?;
    
    println!();
    println!("{}", "─".repeat(60).cyan());
    println!();
    
    if status.success() {
        print_info("Loquat stopped successfully");
    } else {
        print_info(&format!("Loquat exited with status: {}", status));
    }
    
    Ok(())
}
