//! Helper functions

use colored::Colorize;
use std::io::{self, Write};

/// Convert adapter name to factory name
/// Example: "my_adapter" -> "MyAdapterFactory"
pub fn to_factory_name(adapter_name: &str) -> String {
    let mut parts: Vec<String> = adapter_name
        .split('_')
        .map(|s| {
            let mut chars = s.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect();
    
    parts.push("Factory".to_string());
    parts.join("")
}

/// Convert adapter name to struct name
/// Example: "my_adapter" -> "MyAdapter"
pub fn to_struct_name(name: &str) -> String {
    name.split('_')
        .map(|s| {
            let mut chars = s.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Prompt user for confirmation
pub fn prompt_confirmation(message: &str) -> anyhow::Result<bool> {
    print!("{} [y/N]: ", message.yellow());
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    Ok(input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes")
}

/// Print success message
pub fn print_success(message: &str) {
    println!("✓ {}", message.green());
}

/// Print error message
pub fn print_error(message: &str) {
    eprintln!("✗ {}", message.red());
}

/// Print info message
pub fn print_info(message: &str) {
    println!("ℹ {}", message.blue());
}

/// Print warning message
pub fn print_warning(message: &str) {
    println!("⚠ {}", message.yellow());
}

/// Print step header
pub fn print_step(step: &str) {
    println!();
    println!("{}", step.bold().cyan());
    println!("{}", "─".repeat(step.len()));
}

/// Print command suggestion
pub fn print_command_suggestion(command: &str) {
    println!("  $ {}", command.bright_white());
}
