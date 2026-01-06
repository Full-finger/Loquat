//! TUI State Management

use crate::repl::context::ReplContext;
use crate::repl::commands::CommandRegistry;
use crate::tui::log_writer::TuiLogMessage;
use ratatui::style::Color;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Color theme for TUI
#[derive(Debug, Clone)]
pub struct ColorTheme {
    // Background colors
    pub bg_primary: Color,      // Main background
    pub bg_secondary: Color,    // Secondary background (panels)
    pub bg_header: Color,       // Header background
    
    // Text colors
    pub fg_primary: Color,       // Primary text
    pub fg_secondary: Color,     // Secondary text
    pub fg_muted: Color,         // Muted text
    
    // Accent colors
    pub accent: Color,           // Primary accent color
    pub accent_dim: Color,       // Dimmed accent color
    
    // Status colors
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    
    // Border colors
    pub border: Color,
    pub border_active: Color,
}

impl Default for ColorTheme {
    fn default() -> Self {
        Self {
            // Enhanced dark theme with better contrast
            bg_primary: Color::Rgb(23, 28, 36),       // Deep blue-gray
            bg_secondary: Color::Rgb(33, 39, 50),     // Slightly lighter
            bg_header: Color::Rgb(29, 35, 45),         // Header background
            
            fg_primary: Color::Rgb(230, 230, 230),    // Bright white
            fg_secondary: Color::Rgb(190, 190, 190),  // Light gray
            fg_muted: Color::Rgb(130, 130, 130),      // Muted gray
            
            accent: Color::Rgb(0, 210, 210),           // Cyan
            accent_dim: Color::Rgb(0, 150, 150),       // Dimmed cyan
            
            success: Color::Rgb(76, 175, 80),         // Material green
            warning: Color::Rgb(255, 193, 7),         // Amber
            error: Color::Rgb(244, 67, 54),           // Material red
            info: Color::Rgb(33, 150, 243),          // Material blue
            
            border: Color::Rgb(70, 75, 85),           // Subtle border
            border_active: Color::Rgb(0, 210, 210),   // Active border (cyan)
        }
    }
}

/// Application State
/// Manages overall state of TUI application
#[derive(Debug)]
pub struct AppState {
    /// REPL context containing all managers
    pub context: Arc<ReplContext>,
    /// Command registry for executing commands
    pub command_registry: Arc<CommandRegistry>,
    /// Log receiver for real-time log display
    pub log_receiver: mpsc::UnboundedReceiver<TuiLogMessage>,
    /// Cached adapter infos for TUI display
    pub cached_adapter_infos: Vec<crate::adapters::core::types::AdapterInfo>,
    /// Cached active adapter count
    pub cached_active_adapter_count: usize,
    /// Color theme for TUI
    pub theme: ColorTheme,
}

impl AppState {
    /// Create a new application state
    pub fn new(
        context: ReplContext,
        command_registry: CommandRegistry,
        log_receiver: mpsc::UnboundedReceiver<TuiLogMessage>,
    ) -> Self {
        Self {
            context: Arc::new(context),
            command_registry: Arc::new(command_registry),
            log_receiver,
            cached_adapter_infos: Vec::new(),
            cached_active_adapter_count: 0,
            theme: ColorTheme::default(),
        }
    }
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        // Note: log_receiver is intentionally not cloned
        // This is safe because we only clone AppState for UI drawing,
        // and we never try to receive logs from the cloned version
        let (_, dummy_rx) = mpsc::unbounded_channel();
        Self {
            context: self.context.clone(),
            command_registry: self.command_registry.clone(),
            log_receiver: dummy_rx,
            cached_adapter_infos: self.cached_adapter_infos.clone(),
            cached_active_adapter_count: self.cached_active_adapter_count,
            theme: self.theme.clone(),
        }
    }
}

/// Command output entry
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Timestamp
    pub timestamp: String,
    /// Command that was executed
    pub command: String,
    /// Output from command execution
    pub output: String,
    /// Whether command succeeded
    pub success: bool,
}

impl CommandOutput {
    /// Create a new command output
    pub fn new(command: String, output: String, success: bool) -> Self {
        Self {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            command,
            output,
            success,
        }
    }
}

/// UI State
/// Manages UI-specific state
#[derive(Debug, Clone)]
pub struct UiState {
    /// Current selected panel
    pub active_panel: ActivePanel,
    /// Command input buffer
    pub command_input: String,
    /// Command history
    pub command_history: Vec<String>,
    /// Command history index (for browsing history)
    pub history_index: Option<usize>,
    /// Log messages
    pub logs: Vec<LogMessage>,
    /// Maximum number of logs to keep
    pub max_logs: usize,
    /// Minimum log level to display
    pub min_log_level: LogLevel,
    /// Logs scroll offset
    pub logs_scroll_offset: usize,
    /// Command outputs history
    pub command_outputs: Vec<CommandOutput>,
    /// Maximum number of command outputs to keep
    pub max_outputs: usize,
    /// Command outputs scroll offset
    pub outputs_scroll_offset: usize,
    /// Whether to show the output panel
    pub show_output_panel: bool,
    /// Show help modal
    pub show_help: bool,
    /// Show exit confirmation
    pub show_exit_confirm: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            active_panel: ActivePanel::Logs,
            command_input: String::new(),
            command_history: Vec::new(),
            history_index: None,
            logs: Vec::new(),
            max_logs: 1000,
            min_log_level: LogLevel::Info,
            logs_scroll_offset: 0,
            command_outputs: Vec::new(),
            max_outputs: 100,
            outputs_scroll_offset: 0,
            show_output_panel: false,
            show_help: false,
            show_exit_confirm: false,
        }
    }
}

/// Active panel enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    /// Main logs panel
    Logs,
    /// Command outputs panel
    Outputs,
    /// Plugins panel
    Plugins,
    /// Adapters panel
    Adapters,
    /// Config panel
    Config,
    /// Engine panel
    Engine,
}

impl ActivePanel {
    /// Get to next panel
    pub fn next(&self) -> Self {
        match self {
            ActivePanel::Logs => ActivePanel::Outputs,
            ActivePanel::Outputs => ActivePanel::Plugins,
            ActivePanel::Plugins => ActivePanel::Adapters,
            ActivePanel::Adapters => ActivePanel::Config,
            ActivePanel::Config => ActivePanel::Engine,
            ActivePanel::Engine => ActivePanel::Logs,
        }
    }

    /// Get to previous panel
    pub fn prev(&self) -> Self {
        match self {
            ActivePanel::Logs => ActivePanel::Engine,
            ActivePanel::Outputs => ActivePanel::Logs,
            ActivePanel::Plugins => ActivePanel::Outputs,
            ActivePanel::Adapters => ActivePanel::Plugins,
            ActivePanel::Config => ActivePanel::Adapters,
            ActivePanel::Engine => ActivePanel::Config,
        }
    }

    /// Get panel name
    pub fn name(&self) -> String {
        match self {
            ActivePanel::Logs => "Logs".to_string(),
            ActivePanel::Outputs => "Outputs".to_string(),
            ActivePanel::Plugins => "Plugins".to_string(),
            ActivePanel::Adapters => "Adapters".to_string(),
            ActivePanel::Config => "Config".to_string(),
            ActivePanel::Engine => "Engine".to_string(),
        }
    }
}

/// Log message
#[derive(Debug, Clone)]
pub struct LogMessage {
    /// Timestamp
    pub timestamp: String,
    /// Log level
    pub level: LogLevel,
    /// Message content
    pub message: String,
    /// Optional context
    pub context: Option<String>,
}

impl LogMessage {
    /// Create a new log message
    pub fn new(level: LogLevel, message: String, context: Option<String>) -> Self {
        Self {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            level,
            message,
            context,
        }
    }
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
}

impl LogLevel {
    /// Get log level name
    pub fn name(&self) -> &str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    /// Get log level color
    pub fn color(&self) -> &'static str {
        match self {
            LogLevel::Debug => "\x1b[36m",  // Cyan
            LogLevel::Info => "\x1b[32m",   // Green
            LogLevel::Warn => "\x1b[33m",   // Yellow
            LogLevel::Error => "\x1b[31m",  // Red
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_panel_navigation() {
        let mut panel = ActivePanel::Logs;
        
        panel = panel.next();
        assert_eq!(panel, ActivePanel::Plugins);
        
        panel = panel.next();
        assert_eq!(panel, ActivePanel::Adapters);
        
        panel = panel.prev();
        assert_eq!(panel, ActivePanel::Plugins);
    }

    #[test]
    fn test_log_level_colors() {
        assert_eq!(LogLevel::Debug.color(), "\x1b[36m");
        assert_eq!(LogLevel::Info.color(), "\x1b[32m");
        assert_eq!(LogLevel::Warn.color(), "\x1b[33m");
        assert_eq!(LogLevel::Error.color(), "\x1b[31m");
    }
}
