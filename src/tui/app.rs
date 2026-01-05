//! TUI Main Application

use super::state::{AppState, UiState, ActivePanel, LogLevel};
use super::log_writer::TuiLogWriter;
use crate::repl::context::ReplContext;
use crate::repl::commands::CommandRegistry;
use crate::repl::commands::help::HelpCommand;
use crate::repl::commands::status::StatusCommand;
use crate::repl::commands::plugins::PluginsCommand;
use crate::repl::commands::adapters::AdaptersCommand;
use crate::repl::commands::reload::ReloadCommand;
use crate::repl::commands::logs::LogsCommand;
use crate::repl::commands::config::ConfigCommand;
use crate::repl::commands::engine::EngineCommand;
use crate::repl::commands::clear::ClearCommand;
use crate::repl::commands::exit::ExitCommand;
use crate::logging::{init_with_config, LogFormat, LogOutput, LogLevel as LogLogLevel};
use crate::errors::Result;
use tokio::sync::mpsc;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Paragraph, Wrap,
        List, ListItem,
    },
    Frame, Terminal,
};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Stdout};
use std::time::Duration;

/// Loquat TUI Application
pub struct LoquatTui {
    /// Terminal interface
    terminal: Terminal<CrosstermBackend<Stdout>>,
    /// Application state
    app_state: AppState,
    /// UI state
    ui_state: UiState,
    /// Should quit flag
    should_quit: bool,
}

impl LoquatTui {
    /// Create a new TUI instance
    pub fn new(context: ReplContext) -> Result<Self> {
        // Initialize terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        
        // Create log channel for TUI
        let (log_sender, log_receiver) = mpsc::unbounded_channel();
        
        // Initialize command registry with all commands
        let mut command_registry = CommandRegistry::new();
        
        // Get all command names for HelpCommand
        let all_commands = vec![
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
        ];
        
        command_registry.register(Box::new(HelpCommand::new(std::sync::Arc::new(all_commands))));
        command_registry.register(Box::new(StatusCommand));
        command_registry.register(Box::new(PluginsCommand));
        command_registry.register(Box::new(AdaptersCommand));
        command_registry.register(Box::new(ReloadCommand));
        command_registry.register(Box::new(LogsCommand));
        command_registry.register(Box::new(ConfigCommand));
        command_registry.register(Box::new(EngineCommand));
        command_registry.register(Box::new(ClearCommand));
        command_registry.register(Box::new(ExitCommand));
        
        // Initialize states
        let app_state = AppState::new(context, command_registry, log_receiver);
        let ui_state = UiState::default();
        
        // Initialize logger with TUI writer
        let tui_log_writer = TuiLogWriter::new(log_sender);
        let logger = init_with_config(
            std::sync::Arc::new(crate::logging::formatters::JsonFormatter::new()),
            std::sync::Arc::new(tui_log_writer),
            LogLogLevel::Info,
        )?;
        
        // Set global logger
        crate::logging::set_global_logger(logger.clone());
        
        // Log TUI initialization
        logger.log(LogLogLevel::Info, "TUI initialized successfully", &crate::logging::LogContext::current());
        
        Ok(Self {
            terminal,
            app_state,
            ui_state,
            should_quit: false,
        })
    }
    
    /// Run the TUI application
    pub async fn run(&mut self) -> Result<()> {
        self.print_welcome()?;
        
        // Main event loop
        loop {
            // Check for incoming logs
            while let Ok(log_msg) = self.app_state.log_receiver.try_recv() {
                // Convert TuiLogMessage to LogMessage
                let log_level = match log_msg.level {
                    crate::logging::LogLevel::Trace => LogLevel::Debug,
                    crate::logging::LogLevel::Debug => LogLevel::Debug,
                    crate::logging::LogLevel::Info => LogLevel::Info,
                    crate::logging::LogLevel::Warn => LogLevel::Warn,
                    crate::logging::LogLevel::Error => LogLevel::Error,
                };
                
                // Only add if level is >= min_log_level
                if log_level >= self.ui_state.min_log_level {
                    self.ui_state.logs.push(super::state::LogMessage {
                        timestamp: log_msg.timestamp,
                        level: log_level,
                        message: log_msg.message,
                        context: None,
                    });
                    
                    // Trim logs if exceeding max
                    if self.ui_state.logs.len() > self.ui_state.max_logs {
                        self.ui_state.logs.remove(0);
                    }
                }
            }
            
            // Draw UI
            let ui_state = self.ui_state.clone();
            let app_state = self.app_state.clone();
            self.terminal.draw(|f| draw_ui_impl(f, &ui_state, &app_state))?;
            
            // Handle events
            if self.should_quit {
                break;
            }
            
            // Poll for events with timeout
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key_event(key).await?;
                }
            }
        }
        
        // Cleanup
        self.cleanup()?;
        
        Ok(())
    }
    
    /// Print welcome message
    fn print_welcome(&mut self) -> Result<()> {
        let env_name = self.app_state.context.config.general.environment.clone();
        self.terminal.draw(|f| {
            let size = f.size();
            
            // Draw welcome banner
            let welcome_lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "╔══════════════════════════════════════════════════════╗",
                    Style::default().fg(Color::Cyan)
                )),
                Line::from(Span::styled(
                    "║        Loquat Framework - Terminal UI                      ║",
                    Style::default().fg(Color::Cyan)
                )),
                Line::from(Span::styled(
                    "╚══════════════════════════════════════════════════════╝",
                    Style::default().fg(Color::Cyan)
                )),
                Line::from(""),
                Line::from(vec![
                    Span::raw("  Version: "),
                    Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(Color::Green)),
                    Span::raw("    Environment: "),
                    Span::styled(
                        env_name.as_str(),
                        Style::default().fg(Color::Yellow)
                    ),
                ]),
                Line::from(""),
                Line::from("  Press any key to continue..."),
            ];
            
            let paragraph = Paragraph::new(welcome_lines)
                .alignment(Alignment::Center);
            
            f.render_widget(paragraph, size);
        })?;
        
        // Wait for key press
        loop {
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(_) = event::read()? {
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle key events
    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        // Ignore key releases
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }
        
        match key.code {
            // Ctrl+C - Exit
            KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            
            // Ctrl+L - Logs panel
            KeyCode::Char('l') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.ui_state.active_panel = ActivePanel::Logs;
            }
            
            // Ctrl+P - Plugins panel
            KeyCode::Char('p') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.ui_state.active_panel = ActivePanel::Plugins;
            }
            
            // Ctrl+A - Adapters panel
            KeyCode::Char('a') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.ui_state.active_panel = ActivePanel::Adapters;
            }
            
            // Ctrl+C - Config panel (without Ctrl)
            // Note: Ctrl+C with CONTROL modifier is handled above for exit
            KeyCode::Char('c') => {
                self.ui_state.active_panel = ActivePanel::Config;
            }
            
            // Ctrl+E - Engine panel
            KeyCode::Char('e') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.ui_state.active_panel = ActivePanel::Engine;
            }
            
            // Enter - Execute command
            KeyCode::Enter => {
                self.execute_command().await?;
            }
            
            // Backspace - Delete character
            KeyCode::Backspace => {
                self.ui_state.command_input.pop();
            }
            
            // Tab - Switch panel
            KeyCode::Tab => {
                self.ui_state.active_panel = self.ui_state.active_panel.next();
            }
            
            // Backtab - Switch panel reverse
            KeyCode::BackTab => {
                self.ui_state.active_panel = self.ui_state.active_panel.prev();
            }
            
            // Up arrow - Browse command history (when not in Logs panel)
            // Or scroll logs up (when in Logs panel)
            KeyCode::Up => {
                if self.ui_state.active_panel == ActivePanel::Logs {
                    // Scroll logs up
                    if self.ui_state.logs_scroll_offset > 0 {
                        self.ui_state.logs_scroll_offset -= 1;
                    }
                } else if !self.ui_state.command_history.is_empty() {
                    // Browse command history
                    let new_index = match self.ui_state.history_index {
                        None => Some(self.ui_state.command_history.len() - 1),
                        Some(idx) if idx > 0 => Some(idx - 1),
                        Some(_) => Some(0),
                    };
                    self.ui_state.history_index = new_index;
                    if let Some(idx) = new_index {
                        self.ui_state.command_input = self.ui_state.command_history[idx].clone();
                    }
                }
            }
            
            // Down arrow - Browse command history forward (when not in Logs panel)
            // Or scroll logs down (when in Logs panel)
            KeyCode::Down => {
                if self.ui_state.active_panel == ActivePanel::Logs {
                    // Scroll logs down (max scroll to bottom)
                    let max_offset = self.ui_state.logs.len().saturating_sub(1);
                    if self.ui_state.logs_scroll_offset < max_offset {
                        self.ui_state.logs_scroll_offset += 1;
                    }
                } else {
                    match self.ui_state.history_index {
                        None => {}
                        Some(idx) if idx < self.ui_state.command_history.len() - 1 => {
                            self.ui_state.history_index = Some(idx + 1);
                            self.ui_state.command_input = self.ui_state.command_history[idx + 1].clone();
                        }
                        Some(_) => {
                            self.ui_state.history_index = None;
                            self.ui_state.command_input.clear();
                        }
                    }
                }
            }
            
            // PageUp - Scroll logs up by page (when in Logs panel)
            KeyCode::PageUp => {
                if self.ui_state.active_panel == ActivePanel::Logs {
                    let page_size = 10;
                    self.ui_state.logs_scroll_offset = self.ui_state.logs_scroll_offset.saturating_sub(page_size);
                }
            }
            
            // PageDown - Scroll logs down by page (when in Logs panel)
            KeyCode::PageDown => {
                if self.ui_state.active_panel == ActivePanel::Logs {
                    let page_size = 10;
                    let max_offset = self.ui_state.logs.len().saturating_sub(1);
                    self.ui_state.logs_scroll_offset = (self.ui_state.logs_scroll_offset + page_size).min(max_offset);
                }
            }
            
            // Home - Scroll to top (when in Logs panel)
            KeyCode::Home => {
                if self.ui_state.active_panel == ActivePanel::Logs {
                    self.ui_state.logs_scroll_offset = 0;
                }
            }
            
            // End - Scroll to bottom (when in Logs panel)
            KeyCode::End => {
                if self.ui_state.active_panel == ActivePanel::Logs {
                    let max_offset = self.ui_state.logs.len().saturating_sub(1);
                    self.ui_state.logs_scroll_offset = max_offset;
                }
            }
            
            // F1 - Set log level to Debug
            KeyCode::F(1) => {
                self.ui_state.min_log_level = LogLevel::Debug;
                self.ui_state.logs.push(super::state::LogMessage::new(
                    LogLevel::Info,
                    "Log level set to DEBUG".to_string(),
                    None
                ));
            }
            
            // F2 - Set log level to Info
            KeyCode::F(2) => {
                self.ui_state.min_log_level = LogLevel::Info;
                self.ui_state.logs.push(super::state::LogMessage::new(
                    LogLevel::Info,
                    "Log level set to INFO".to_string(),
                    None
                ));
            }
            
            // F3 - Set log level to Warn
            KeyCode::F(3) => {
                self.ui_state.min_log_level = LogLevel::Warn;
                self.ui_state.logs.push(super::state::LogMessage::new(
                    LogLevel::Info,
                    "Log level set to WARN".to_string(),
                    None
                ));
            }
            
            // F4 - Set log level to Error
            KeyCode::F(4) => {
                self.ui_state.min_log_level = LogLevel::Error;
                self.ui_state.logs.push(super::state::LogMessage::new(
                    LogLevel::Info,
                    "Log level set to ERROR".to_string(),
                    None
                ));
            }
            
            // Character input
            KeyCode::Char(c) => {
                self.ui_state.command_input.push(c);
            }
            
            // Ignore other keys
            _ => {}
        }
        
        Ok(())
    }
    
    /// Execute command
    async fn execute_command(&mut self) -> Result<()> {
        let command = self.ui_state.command_input.clone();
        self.ui_state.command_input.clear();
        self.ui_state.history_index = None;
        
        if command.is_empty() {
            return Ok(());
        }
        
        // Add to history
        self.ui_state.command_history.push(command.clone());
        
        // Parse and execute command
        let parts: Vec<&str> = command.split_whitespace().collect();
        if let Some(cmd_name) = parts.first() {
            let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
            
            // Add log message about command execution
            let log_msg = format!("Executing command: {}", command);
            self.ui_state.logs.push(super::state::LogMessage::new(
                LogLevel::Info,
                log_msg,
                None
            ));
            
            // Find and execute command
            if let Some(cmd) = self.app_state.command_registry.find(cmd_name) {
                let ctx = self.app_state.context.clone();
                match cmd.execute(&args, &ctx).await {
                    Ok(()) => {
                        self.ui_state.logs.push(super::state::LogMessage::new(
                            LogLevel::Info,
                            format!("Command '{}' executed successfully", cmd_name),
                            None
                        ));
                    }
                    Err(e) => {
                        self.ui_state.logs.push(super::state::LogMessage::new(
                            LogLevel::Error,
                            format!("Command '{}' failed: {}", cmd_name, e),
                            None
                        ));
                    }
                }
            } else {
                self.ui_state.logs.push(super::state::LogMessage::new(
                    LogLevel::Error,
                    format!("Unknown command: {}", cmd_name),
                    None
                ));
            }
        }
        
        Ok(())
    }
    
    /// Cleanup and restore terminal
    fn cleanup(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(
            io::stdout(),
            LeaveAlternateScreen
        )?;
        Ok(())
    }
}

impl Drop for LoquatTui {
    fn drop(&mut self) {
        // Ensure terminal is restored on drop
        let _ = self.cleanup();
    }
}

/// Independent implementation of draw_ui to avoid borrow conflicts
fn draw_ui_impl(f: &mut Frame, ui_state: &UiState, app_state: &AppState) {
    let size = f.size();
    
    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(3),  // Command input
        ])
        .split(size);
    
    // Draw header
    draw_header_impl(f, chunks[0], app_state);
    
    // Draw main content
    draw_main_content_impl(f, chunks[1], ui_state, app_state);
    
    // Draw command input
    draw_command_input_impl(f, chunks[2], ui_state);
}
    
/// Independent implementation of draw_header
fn draw_header_impl(f: &mut Frame, area: Rect, app_state: &AppState) {
    let title = " Loquat TUI ";
    let version = format!(" v{}", env!("CARGO_PKG_VERSION"));
    let env = format!(" [{}]", app_state.context.config.general.environment);
    
    let header_line = Line::from(vec![
        Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(version, Style::default().fg(Color::Green)),
        Span::raw(" "),
        Span::styled(env, Style::default().fg(Color::Yellow)),
    ]);
    
    let header = Block::default()
        .borders(Borders::ALL)
        .title(header_line)
        .title_alignment(Alignment::Left);
    
    f.render_widget(header, area);
}
    
/// Independent implementation of draw_main_content
fn draw_main_content_impl(f: &mut Frame, area: Rect, ui_state: &UiState, app_state: &AppState) {
    // Split into side panel and main panel
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([
            Constraint::Length(25), // Side panel
            Constraint::Min(0),    // Main panel
        ])
        .split(area);
    
    // Draw side panel
    draw_side_panel_impl(f, chunks[0]);
    
    // Draw main panel
    draw_main_panel_impl(f, chunks[1], ui_state, app_state);
}
    
/// Independent implementation of draw_side_panel
fn draw_side_panel_impl(f: &mut Frame, area: Rect) {
    let items = vec![
        ListItem::new(" Logs [Ctrl+L]"),
        ListItem::new(" Plugins [Ctrl+P]"),
        ListItem::new(" Adapters [Ctrl+A]"),
        ListItem::new(" Config [Ctrl+C]"),
        ListItem::new(" Engine [Ctrl+E]"),
    ];
    
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Navigation ")
        )
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        );
    
    f.render_widget(list, area);
}
    
/// Independent implementation of draw_main_panel
fn draw_main_panel_impl(f: &mut Frame, area: Rect, ui_state: &UiState, app_state: &AppState) {
    let panel_name = ui_state.active_panel.name();
    
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", panel_name));
    
    // Calculate inner area before rendering
    let inner = block.inner(area);
    f.render_widget(block, area);
    
    // Draw panel content based on active panel
    match ui_state.active_panel {
        ActivePanel::Logs => draw_logs_panel_impl(f, inner, ui_state),
        ActivePanel::Plugins => draw_plugins_panel_impl(f, inner, app_state),
        ActivePanel::Adapters => draw_adapters_panel_impl(f, inner, app_state),
        ActivePanel::Config => draw_config_panel_impl(f, inner, app_state),
        ActivePanel::Engine => draw_engine_panel_impl(f, inner, app_state),
    }
}
    
/// Independent implementation of draw_logs_panel
fn draw_logs_panel_impl(f: &mut Frame, area: Rect, ui_state: &UiState) {
    if ui_state.logs.is_empty() {
        let text = Text::from(" No logs yet. Waiting for events...");
        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    }
    
    // Convert logs to text with color-coded levels
    let log_lines: Vec<Line> = ui_state.logs
        .iter()
        .map(|log| {
            let timestamp = format!("{} ", log.timestamp);
            let level = format!("[{}] ", log.level);
            
            // Color code by log level
            let level_color = match log.level {
                LogLevel::Debug => Color::Cyan,
                LogLevel::Info => Color::Green,
                LogLevel::Warn => Color::Yellow,
                LogLevel::Error => Color::Red,
            };
            
            Line::from(vec![
                Span::styled(
                    timestamp,
                    Style::default().fg(Color::DarkGray)
                ),
                Span::styled(
                    level,
                    Style::default().fg(level_color).add_modifier(Modifier::BOLD)
                ),
                Span::raw(&log.message),
            ])
        })
        .collect();
    
    // Use scroll offset from ui_state
    let scroll_offset = ui_state.logs_scroll_offset as u16;
    let paragraph = Paragraph::new(log_lines)
        .wrap(Wrap { trim: true })
        .scroll((0, scroll_offset));
    
    f.render_widget(paragraph, area);
}
    
/// Independent implementation of draw_plugins_panel
fn draw_plugins_panel_impl(f: &mut Frame, area: Rect, app_state: &AppState) {
    if let Some(plugin_manager) = &app_state.context.plugin_manager {
        let plugin_infos = plugin_manager.list_plugin_infos();
        
        if plugin_infos.is_empty() {
            let text_lines = vec![
                Line::from(" Plugins Panel "),
                Line::from(""),
                Line::from(" No plugins loaded."),
                Line::from(""),
                Line::from(" Keyboard shortcuts:"),
                Line::from("   'l' - Load plugin"),
                Line::from("   Enter - View details"),
            ];
            
            let text = Text::from(text_lines);
            let paragraph = Paragraph::new(text)
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Left);
            
            f.render_widget(paragraph, area);
            return;
        }
        
        // Build plugin list
        let mut lines = vec![
            Line::from(" Plugins Panel "),
            Line::from(""),
        ];
        
        // Add summary
        lines.push(Line::from(vec![
            Span::raw("  Total: "),
            Span::styled(
                format!("{} ", plugin_infos.len()),
                Style::default().fg(Color::Cyan)
            ),
            Span::raw("Active: "),
            Span::styled(
                format!("{} ", plugin_manager.active_plugin_count()),
                Style::default().fg(Color::Green)
            ),
        ]));
        lines.push(Line::from(""));
        
        // Add each plugin
        for (index, plugin_info) in plugin_infos.iter().enumerate() {
            let status_color = match &plugin_info.status {
                crate::plugins::types::PluginStatus::Loaded => Color::Green,
                crate::plugins::types::PluginStatus::Loading => Color::Yellow,
                crate::plugins::types::PluginStatus::Error { .. } => Color::Red,
                crate::plugins::types::PluginStatus::Disabled => Color::Gray,
                crate::plugins::types::PluginStatus::Unloaded => Color::Yellow,
            };
            
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}. ", index + 1),
                    Style::default().fg(Color::DarkGray)
                ),
                Span::styled(
                    format!("[{:?}] ", plugin_info.status),
                    Style::default().fg(status_color).add_modifier(Modifier::BOLD)
                ),
                Span::styled(
                    plugin_info.metadata.name.clone(),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                ),
                Span::raw(" v"),
                Span::styled(
                    plugin_info.metadata.version.clone(),
                    Style::default().fg(Color::Yellow)
                ),
            ]));
            
            lines.push(Line::from(vec![
                Span::raw("    Type: "),
                Span::styled(
                    format!("{:?} ", plugin_info.metadata.plugin_type),
                    Style::default().fg(Color::Cyan)
                ),
                Span::raw("Path: "),
                Span::styled(
                    plugin_info.metadata.entry_point.clone(),
                    Style::default().fg(Color::Gray)
                ),
            ]));
            lines.push(Line::from(""));
        }
        
        // Add keyboard shortcuts at the end
        lines.push(Line::from(""));
        lines.push(Line::from(" Keyboard shortcuts:"));
        lines.push(Line::from("   'l' - Load plugin"));
        lines.push(Line::from("   'u' - Unload selected plugin"));
        lines.push(Line::from("   Enter - View details"));
        
        let text = Text::from(lines);
        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);
        
        f.render_widget(paragraph, area);
    } else {
        // Plugin manager not available
        let text_lines = vec![
            Line::from(" Plugins Panel "),
            Line::from(""),
            Line::from(" Plugin manager not available."),
            Line::from(""),
            Line::from(" This may indicate that plugins are not"),
            Line::from(" configured or initialized."),
        ];
        
        let text = Text::from(text_lines);
        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);
        
        f.render_widget(paragraph, area);
    }
}
    
/// Independent implementation of draw_adapters_panel
fn draw_adapters_panel_impl(f: &mut Frame, area: Rect, app_state: &AppState) {
    if let Some(_adapter_manager) = &app_state.context.adapter_manager {
        // Note: Since list_adapter_infos() is async and draw functions are not async,
        // we show a placeholder message for now
        let text_lines = vec![
            Line::from(" Adapters Panel "),
            Line::from(""),
            Line::from(" Adapter list requires async data loading."),
            Line::from(""),
            Line::from(" This will be implemented with async data"),
            Line::from(" caching in the main TUI loop."),
            Line::from(""),
            Line::from(" Keyboard shortcuts:"),
            Line::from("   'r' - Reload adapter"),
            Line::from("   'u' - Unload adapter"),
            Line::from("   Enter - View details"),
        ];
        
        let text = Text::from(text_lines);
        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);
        
        f.render_widget(paragraph, area);
    } else {
        // Adapter manager not available
        let text_lines = vec![
            Line::from(" Adapters Panel "),
            Line::from(""),
            Line::from(" Adapter manager not available."),
            Line::from(""),
            Line::from(" This may indicate that adapters are not"),
            Line::from(" configured or initialized."),
        ];
        
        let text = Text::from(text_lines);
        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);
        
        f.render_widget(paragraph, area);
    }
}
    
/// Independent implementation of draw_config_panel
fn draw_config_panel_impl(f: &mut Frame, area: Rect, app_state: &AppState) {
    let config = &app_state.context.config;
    
    // Build config display
    let mut lines = vec![
        Line::from(" Config Panel "),
        Line::from(""),
    ];
    
    // General section
    lines.push(Line::from(vec![
        Span::styled(
            "  [General]",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    Environment: "),
        Span::styled(
            config.general.environment.clone(),
            Style::default().fg(Color::Yellow)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    Framework: "),
        Span::styled(
            config.general.name.clone(),
            Style::default().fg(Color::Green)
        ),
    ]));
    lines.push(Line::from(""));
    
    // Logging section
    lines.push(Line::from(vec![
        Span::styled(
            "  [Logging]",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    Level: "),
        Span::styled(
            format!("{:?}", config.logging.level),
            Style::default().fg(Color::Yellow)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    Format: "),
        Span::styled(
            format!("{:?}", config.logging.format),
            Style::default().fg(Color::Green)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    Output: "),
        Span::styled(
            format!("{:?}", config.logging.output),
            Style::default().fg(Color::Green)
        ),
    ]));
    lines.push(Line::from(""));
    
    // Plugins section
    lines.push(Line::from(vec![
        Span::styled(
            "  [Plugins]",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    Directory: "),
        Span::styled(
            config.plugins.plugin_dir.clone(),
            Style::default().fg(Color::Green)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    Auto-load: "),
        Span::styled(
            config.plugins.auto_load.to_string(),
            Style::default().fg(Color::Yellow)
        ),
    ]));
    lines.push(Line::from(""));
    
    // Adapters section
    lines.push(Line::from(vec![
        Span::styled(
            "  [Adapters]",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    Directory: "),
        Span::styled(
            config.adapters.adapter_dir.clone(),
            Style::default().fg(Color::Green)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    Auto-load: "),
        Span::styled(
            config.adapters.auto_load.to_string(),
            Style::default().fg(Color::Yellow)
        ),
    ]));
    lines.push(Line::from(""));
    
    // Engine section
    lines.push(Line::from(vec![
        Span::styled(
            "  [Engine]",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    Auto-route: "),
        Span::styled(
            config.engine.auto_route.to_string(),
            Style::default().fg(Color::Yellow)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    Auto-create-channels: "),
        Span::styled(
            config.engine.auto_create_channels.to_string(),
            Style::default().fg(Color::Yellow)
        ),
    ]));
    lines.push(Line::from(""));
    
    // Web section
    lines.push(Line::from(vec![
        Span::styled(
            "  [Web]",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    Host: "),
        Span::styled(
            config.web.host.clone(),
            Style::default().fg(Color::Green)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    Port: "),
        Span::styled(
            config.web.port.to_string(),
            Style::default().fg(Color::Yellow)
        ),
    ]));
    
    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);
    
    f.render_widget(paragraph, area);
}
    
/// Independent implementation of draw_engine_panel
fn draw_engine_panel_impl(f: &mut Frame, area: Rect, app_state: &AppState) {
    if let Some(engine) = &app_state.context.engine {
        let state = engine.try_state();
        let stats = engine.stats();
        let config = engine.config();
        
        // Build engine display
        let mut lines = vec![
            Line::from(" Engine Panel "),
            Line::from(""),
        ];
        
        // Status
        let status_color = match state.status {
            crate::engine::types::EngineStatus::Running => Color::Green,
            crate::engine::types::EngineStatus::Stopped => Color::Red,
            _ => Color::Yellow,
        };
        
        lines.push(Line::from(vec![
            Span::raw("  Status: "),
            Span::styled(
                format!("{:?}", state.status),
                Style::default().fg(status_color).add_modifier(Modifier::BOLD)
            ),
        ]));
        lines.push(Line::from(""));
        
        // Statistics - using correct field names
        lines.push(Line::from(vec![
            Span::styled(
                "  Statistics",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    Total packages: "),
            Span::styled(
                stats.total_packages.to_string(),
                Style::default().fg(Color::Green)
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    Successful packages: "),
            Span::styled(
                stats.successful_packages.to_string(),
                Style::default().fg(Color::Green)
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    Failed packages: "),
            Span::styled(
                stats.failed_packages.to_string(),
                Style::default().fg(Color::Red)
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    Active channels: "),
            Span::styled(
                stats.active_channels.to_string(),
                Style::default().fg(Color::Yellow)
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    Avg processing time: "),
            Span::styled(
                format!("{} ms", stats.avg_processing_time_ms),
                Style::default().fg(Color::Cyan)
            ),
        ]));
        lines.push(Line::from(""));
        
        // Configuration
        lines.push(Line::from(vec![
            Span::styled(
                "  Configuration",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    Auto-route: "),
            Span::styled(
                config.auto_route.to_string(),
                Style::default().fg(Color::Yellow)
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    Auto-create-channels: "),
            Span::styled(
                config.auto_create_channels.to_string(),
                Style::default().fg(Color::Yellow)
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    Auto-initialize: "),
            Span::styled(
                config.auto_initialize.to_string(),
                Style::default().fg(Color::Yellow)
            ),
        ]));
        
        let text = Text::from(lines);
        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);
        
        f.render_widget(paragraph, area);
    } else {
        // Engine not available
        let text_lines = vec![
            Line::from(" Engine Panel "),
            Line::from(""),
            Line::from(" Engine not available."),
            Line::from(""),
            Line::from(" This may indicate that to engine"),
            Line::from(" is not configured or initialized."),
        ];
        
        let text = Text::from(text_lines);
        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);
        
        f.render_widget(paragraph, area);
    }
}
    
/// Independent implementation of draw_command_input
fn draw_command_input_impl(f: &mut Frame, area: Rect, ui_state: &UiState) {
    let input_text = format!("> {}", ui_state.command_input);
    
    let paragraph = Paragraph::new(input_text)
        .style(Style::default().fg(Color::Green))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Command ")
        );
    
    f.render_widget(paragraph, area);
}
