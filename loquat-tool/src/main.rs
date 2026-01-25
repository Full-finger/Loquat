//! Loquat Development Tool
//!
//! A command-line tool for managing Loquat framework adapters and plugins

mod commands;
mod utils;

use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(name = "loquat-tool")]
#[command(about = "Loquat development tool", long_about = "A CLI tool for creating, managing, and running Loquat adapters and plugins")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new adapter or plugin
    New {
        #[command(subcommand)]
        target: NewTarget,
    },
    
    /// Remove an adapter or plugin
    Remove {
        #[command(subcommand)]
        target: RemoveTarget,
    },
    
    /// List adapters or plugins
    List {
        #[command(subcommand)]
        target: ListTarget,
    },
    
    /// Check project for errors
    Check,
    
    /// Run Loquat framework
    Run {
        #[arg(short, long, default_value = "dev", help = "Environment to run")]
        env: String,
        
        #[arg(short, long, help = "Start in REPL mode")]
        repl: bool,
        
        #[arg(short, long, help = "Start in TUI mode")]
        tui: bool,
    },
}

#[derive(Subcommand)]
enum NewTarget {
    /// Create a new adapter
    Adapter {
        #[arg(help = "Name of the adapter")]
        name: String,
    },
    
    /// Create a new plugin
    Plugin {
        #[arg(help = "Name of the plugin")]
        name: String,
    },
}

#[derive(Subcommand)]
enum RemoveTarget {
    /// Remove an adapter
    Adapter {
        #[arg(help = "Name of the adapter")]
        name: String,
        
        #[arg(short, long, help = "Force removal without confirmation")]
        force: bool,
    },
    
    /// Remove a plugin
    Plugin {
        #[arg(help = "Name of the plugin")]
        name: String,
        
        #[arg(short, long, help = "Force removal without confirmation")]
        force: bool,
    },
}

#[derive(Subcommand)]
enum ListTarget {
    /// List all adapters
    Adapters,
    
    /// List all plugins
    Plugins,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    println!();
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║              {}               ║", "Loquat Development Tool".cyan().bold());
    println!("╚════════════════════════════════════════════════════════╝");
    println!();
    
    match cli.command {
        Commands::New { target } => {
            match target {
                NewTarget::Adapter { name } => {
                    commands::new::create_adapter(&name)?;
                }
                NewTarget::Plugin { name } => {
                    commands::new::create_plugin(&name)?;
                }
            }
        }
        Commands::Remove { target } => {
            match target {
                RemoveTarget::Adapter { name, force } => {
                    commands::remove::remove_adapter(&name, force)?;
                }
                RemoveTarget::Plugin { name, force } => {
                    commands::remove::remove_plugin(&name, force)?;
                }
            }
        }
        Commands::List { target } => {
            match target {
                ListTarget::Adapters => {
                    commands::list::list_adapters()?;
                }
                ListTarget::Plugins => {
                    commands::list::list_plugins()?;
                }
            }
        }
        Commands::Check => {
            commands::check::check_project()?;
        }
        Commands::Run { env, repl, tui } => {
            commands::run::run_loquat(env, repl, tui)?;
        }
    }
    
    Ok(())
}
