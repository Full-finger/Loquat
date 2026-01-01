//! TUI (Terminal User Interface) Module
//! Provides a modern, interactive terminal-based UI for Loquat Framework

mod app;
mod log_writer;
mod state;

pub use app::LoquatTui;
pub use state::{AppState, UiState};

/// TUI entry point
pub async fn run_tui(context: crate::repl::context::ReplContext) -> crate::errors::Result<()> {
    let mut tui = LoquatTui::new(context)?;
    tui.run().await
}
