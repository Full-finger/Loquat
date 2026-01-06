//! TUI Main Application

use super::state::{AppState, UiState, ActivePanel, LogLevel, ColorTheme};
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
use crate::logging::{init_with_config, LogLevel as LogLogLevel};
use crate::errors::Result;
use tokio::sync::mpsc;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Paragraph, Wrap,
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
        let mut update_counter: usize = 0;
        
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
            
            // Update adapter cache every 10 iterations (approximately 1 second)
            update_counter = (update_counter + 1) % 10;
            if update_counter == 0 {
                if let Some(adapter_manager) = &self.app_state.context.adapter_manager {
                    let adapter_infos = adapter_manager.list_adapter_infos().await;
                    let active_count = adapter_manager.active_adapter_count().await;
                    
                    self.app_state.cached_adapter_infos = adapter_infos;
                    self.app_state.cached_active_adapter_count = active_count;
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
        let theme = &self.app_state.theme;
        
        self.terminal.draw(|f| {
            let size = f.size();
            
            // Draw welcome banner with modern design
            let welcome_lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "╔══════════════════════════════════════════════════════════════╗",
                    Style::default().fg(theme.accent)
                )),
                Line::from(Span::styled(
                    "║                    🍃 Loquat Framework                         ║",
                    Style::default().fg(theme.accent)
                )),
                Line::from(Span::styled(
                    "║                  Terminal User Interface                       ║",
                    Style::default().fg(theme.fg_secondary)
                )),
                Line::from(Span::styled(
                    "╚══════════════════════════════════════════════════════════════╝",
                    Style::default().fg(theme.accent)
                )),
                Line::from(""),
                Line::from(vec![
                    Span::raw("  📦 Version: "),
                    Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(theme.success)),
                    Span::raw("    🌍 Environment: "),
                    Span::styled(
                        env_name.as_str(),
                        Style::default().fg(theme.warning)
                    ),
                ]),
                Line::from(vec![
                    Span::raw("  ⚡ Status: "),
                    Span::styled("● Ready", Style::default().fg(theme.success)),
                ]),
                Line::from(""),
                Line::from("  🎯 Press any key to continue..."),
            ];
            
            let paragraph = Paragraph::new(welcome_lines)
                .style(Style::default().fg(theme.fg_primary).bg(theme.bg_primary))
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
            
            // Ctrl+O - Toggle output panel
            KeyCode::Char('o') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                if !self.ui_state.command_outputs.is_empty() {
                    self.ui_state.show_output_panel = !self.ui_state.show_output_panel;
                }
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
            
            // Up arrow - Browse command history (when not in Logs or Outputs panel)
            // Or scroll logs up (when in Logs panel)
            // Or scroll outputs up (when in Outputs panel)
            KeyCode::Up => {
                if self.ui_state.active_panel == ActivePanel::Logs {
                    // Scroll logs up
                    if self.ui_state.logs_scroll_offset > 0 {
                        self.ui_state.logs_scroll_offset -= 1;
                    }
                } else if self.ui_state.active_panel == ActivePanel::Outputs {
                    // Scroll outputs up
                    if self.ui_state.outputs_scroll_offset > 0 {
                        self.ui_state.outputs_scroll_offset -= 1;
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
            
            // Down arrow - Browse command history forward (when not in Logs or Outputs panel)
            // Or scroll logs down (when in Logs panel)
            // Or scroll outputs down (when in Outputs panel)
            KeyCode::Down => {
                if self.ui_state.active_panel == ActivePanel::Logs {
                    // Scroll logs down (max scroll to bottom)
                    let max_offset = self.ui_state.logs.len().saturating_sub(1);
                    if self.ui_state.logs_scroll_offset < max_offset {
                        self.ui_state.logs_scroll_offset += 1;
                    }
                } else if self.ui_state.active_panel == ActivePanel::Outputs {
                    // Scroll outputs down
                    let max_offset = self.ui_state.command_outputs.len().saturating_sub(1);
                    if self.ui_state.outputs_scroll_offset < max_offset {
                        self.ui_state.outputs_scroll_offset += 1;
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
            // Or scroll outputs up by page (when in Outputs panel)
            KeyCode::PageUp => {
                if self.ui_state.active_panel == ActivePanel::Logs {
                    let page_size = 10;
                    self.ui_state.logs_scroll_offset = self.ui_state.logs_scroll_offset.saturating_sub(page_size);
                } else if self.ui_state.active_panel == ActivePanel::Outputs {
                    let page_size = 10;
                    self.ui_state.outputs_scroll_offset = self.ui_state.outputs_scroll_offset.saturating_sub(page_size);
                }
            }
            
            // PageDown - Scroll logs down by page (when in Logs panel)
            // Or scroll outputs down by page (when in Outputs panel)
            KeyCode::PageDown => {
                if self.ui_state.active_panel == ActivePanel::Logs {
                    let page_size = 10;
                    let max_offset = self.ui_state.logs.len().saturating_sub(1);
                    self.ui_state.logs_scroll_offset = (self.ui_state.logs_scroll_offset + page_size).min(max_offset);
                } else if self.ui_state.active_panel == ActivePanel::Outputs {
                    let page_size = 10;
                    let max_offset = self.ui_state.command_outputs.len().saturating_sub(1);
                    self.ui_state.outputs_scroll_offset = (self.ui_state.outputs_scroll_offset + page_size).min(max_offset);
                }
            }
            
            // Home - Scroll to top (when in Logs or Outputs panel)
            KeyCode::Home => {
                if self.ui_state.active_panel == ActivePanel::Logs {
                    self.ui_state.logs_scroll_offset = 0;
                } else if self.ui_state.active_panel == ActivePanel::Outputs {
                    self.ui_state.outputs_scroll_offset = 0;
                }
            }
            
            // End - Scroll to bottom (when in Logs or Outputs panel)
            KeyCode::End => {
                if self.ui_state.active_panel == ActivePanel::Logs {
                    let max_offset = self.ui_state.logs.len().saturating_sub(1);
                    self.ui_state.logs_scroll_offset = max_offset;
                } else if self.ui_state.active_panel == ActivePanel::Outputs {
                    let max_offset = self.ui_state.command_outputs.len().saturating_sub(1);
                    self.ui_state.outputs_scroll_offset = max_offset;
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
                let result = cmd.execute(&args, &ctx).await;
                
                let success = result.is_ok();
                let output_text = match result {
                    Ok(_) => format!("Command '{}' executed successfully", cmd_name),
                    Err(e) => format!("Command '{}' failed: {}", cmd_name, e),
                };
                
                // Add to logs
                self.ui_state.logs.push(super::state::LogMessage::new(
                    if success { LogLevel::Info } else { LogLevel::Error },
                    output_text.clone(),
                    None
                ));
                
                // Add to command outputs
                self.ui_state.command_outputs.push(super::state::CommandOutput::new(
                    command.clone(),
                    output_text,
                    success
                ));
                
                // Trim outputs if exceeding max
                if self.ui_state.command_outputs.len() > self.ui_state.max_outputs {
                    self.ui_state.command_outputs.remove(0);
                }
                
                // Show output panel and switch to Outputs panel to show the result
                self.ui_state.show_output_panel = true;
                self.ui_state.active_panel = ActivePanel::Outputs;
            } else {
                let error_msg = format!("Unknown command: {}", cmd_name);
                self.ui_state.logs.push(super::state::LogMessage::new(
                    LogLevel::Error,
                    error_msg.clone(),
                    None
                ));
                
                // Add to command outputs
                self.ui_state.command_outputs.push(super::state::CommandOutput::new(
                    command.clone(),
                    error_msg,
                    false
                ));
                
                // Show output panel for error messages too
                self.ui_state.show_output_panel = true;
                self.ui_state.active_panel = ActivePanel::Outputs;
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
    let theme = &app_state.theme;
    
    // Clear with background color
    f.render_widget(
        ratatui::widgets::Block::default()
            .style(Style::default().bg(theme.bg_primary)),
        size
    );
    
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
    let theme = &app_state.theme;
    let title = " Loquat TUI ";
    let version = format!(" v{}", env!("CARGO_PKG_VERSION"));
    let env = format!(" [{}]", app_state.context.config.general.environment);
    let time = chrono::Local::now().format("%H:%M:%S").to_string();
    
    let header_line = Line::from(vec![
        Span::styled("🍃", Style::default().fg(theme.accent)),
        Span::styled(title, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(version, Style::default().fg(theme.success)),
        Span::raw("  "),
        Span::styled(env, Style::default().fg(theme.warning)),
        Span::raw("  "),
        Span::styled("🕐 ", Style::default().fg(theme.fg_muted)),
        Span::styled(time, Style::default().fg(theme.fg_muted)),
    ]);
    
    let header = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(theme.bg_header))
        .title(header_line)
        .title_alignment(Alignment::Left)
        .border_style(Style::default().fg(theme.border_active));
    
    f.render_widget(header, area);
}
    
/// Independent implementation of draw_main_content
fn draw_main_content_impl(f: &mut Frame, area: Rect, ui_state: &UiState, app_state: &AppState) {
    let theme = &app_state.theme;
    
    // Dynamic layout based on whether outputs panel is shown
    if ui_state.show_output_panel && !ui_state.command_outputs.is_empty() {
        // Three-column layout when outputs are shown
        // Left navigation (18) + Main panel (flexible) + Outputs panel (min 30)
        let available_width = area.width.saturating_sub(48); // Subtract 18 + 30
        let main_width = available_width;
        
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(0)
            .constraints([
                Constraint::Length(18), // Side panel (narrower)
                Constraint::Min(30),     // Main panel
                Constraint::Min(30),    // Outputs panel (minimum 30)
            ])
            .split(area);
        
        // Draw side panel
        draw_side_panel_impl(f, chunks[0], theme, ui_state);
        
        // Draw main panel
        draw_main_panel_impl(f, chunks[1], ui_state, app_state);
        
        // Draw outputs panel
        draw_outputs_panel_wrapper(f, chunks[2], ui_state, theme);
    } else {
        // Two-column layout when no outputs
        // Left navigation (20) + Main panel
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(0)
            .constraints([
                Constraint::Length(20), // Side panel
                Constraint::Min(0),     // Main panel
            ])
            .split(area);
        
        // Draw side panel
        draw_side_panel_impl(f, chunks[0], theme, ui_state);
        
        // Draw main panel
        draw_main_panel_impl(f, chunks[1], ui_state, app_state);
    }
}

/// Draw outputs panel wrapper (with border and title)
fn draw_outputs_panel_wrapper(f: &mut Frame, area: Rect, ui_state: &UiState, theme: &ColorTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Command Outputs [Ctrl+O] ")
        .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(theme.border_active))
        .style(Style::default().bg(theme.bg_secondary));
    
    // Calculate inner area before rendering
    let inner = block.inner(area);
    f.render_widget(block, area);
    
    // Draw outputs panel content
    draw_outputs_panel_impl(f, inner, ui_state, theme);
}
    
/// Independent implementation of draw_outputs_panel
fn draw_outputs_panel_impl(f: &mut Frame, area: Rect, ui_state: &UiState, theme: &ColorTheme) {
    if ui_state.command_outputs.is_empty() {
        let text = Text::from(vec![
            Line::from(""),
            Line::from(" No command outputs yet."),
            Line::from(""),
            Line::from(" Execute commands to see their output here."),
            Line::from(""),
            Line::from(" Press Ctrl+O to toggle output panel"),
        ]);
        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(theme.fg_muted))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
        return;
    }
    
    // Build output lines
    let output_lines: Vec<Line> = ui_state.command_outputs
        .iter()
        .flat_map(|output| {
            let status_color = if output.success {
                theme.success
            } else {
                theme.error
            };
            
            let status_icon = if output.success {
                "✓"
            } else {
                "✗"
            };
            
            let mut lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("🕐 ", Style::default().fg(theme.fg_muted)),
                    Span::styled(&output.timestamp, Style::default().fg(theme.fg_muted)),
                    Span::raw("  "),
                    Span::styled(status_icon, Style::default().fg(status_color)),
                    Span::raw("  "),
                    Span::styled(&output.command, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                ]),
            ];
            
            // Split output into lines
            for line in output.output.lines() {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(line, Style::default().fg(theme.fg_primary)),
                ]));
            }
            
            lines
        })
        .collect();
    
    // Use scroll offset from ui_state
    let scroll_offset = ui_state.outputs_scroll_offset as u16;
    let paragraph = Paragraph::new(output_lines)
        .style(Style::default().fg(theme.fg_primary))
        .wrap(Wrap { trim: true })
        .scroll((0, scroll_offset));
    
    f.render_widget(paragraph, area);
}
    
/// Independent implementation of draw_side_panel
fn draw_side_panel_impl(f: &mut Frame, area: Rect, theme: &ColorTheme, ui_state: &UiState) {
    let items = vec![
        ListItem::new(" 📋 Logs [Ctrl+L]"),
        ListItem::new(" 📤 Outputs [Ctrl+O]"),
        ListItem::new(" ⚙ Plugins [Ctrl+P]"),
        ListItem::new(" 🔌 Adapters [Ctrl+A]"),
        ListItem::new(" ⚙ Config [Ctrl+C]"),
        ListItem::new(" ⚡ Engine [Ctrl+E]"),
    ];
    
    // Highlight active panel
    let highlight_index = match ui_state.active_panel {
        ActivePanel::Logs => 0,
        ActivePanel::Outputs => 1,
        ActivePanel::Plugins => 2,
        ActivePanel::Adapters => 3,
        ActivePanel::Config => 4,
        ActivePanel::Engine => 5,
    };
    
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Navigation ")
                .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
                .border_style(Style::default().fg(theme.border_active))
                .style(Style::default().bg(theme.bg_secondary))
        )
        .style(Style::default().fg(theme.fg_secondary))
        .highlight_style(
            Style::default()
                .fg(theme.accent)
                .bg(theme.bg_header)
                .add_modifier(Modifier::BOLD)
        )
        .highlight_symbol(">> ");
    
    f.render_stateful_widget(
        list,
        area,
        &mut ratatui::widgets::ListState::default().with_selected(Some(highlight_index))
    );
}

/// Independent implementation of draw_main_panel
fn draw_main_panel_impl(f: &mut Frame, area: Rect, ui_state: &UiState, app_state: &AppState) {
    let theme = &app_state.theme;
    let panel_name = ui_state.active_panel.name();
    
    // Don't show Outputs in main panel if it's shown in separate panel
    if ui_state.active_panel == ActivePanel::Outputs && ui_state.show_output_panel {
        return;
    }
    
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", panel_name))
        .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(theme.border_active))
        .style(Style::default().bg(theme.bg_secondary));
    
    // Calculate inner area before rendering
    let inner = block.inner(area);
    f.render_widget(block, area);
    
    // Draw panel content based on active panel
    match ui_state.active_panel {
        ActivePanel::Logs => draw_logs_panel_impl(f, inner, ui_state, theme),
        ActivePanel::Outputs => draw_outputs_panel_impl(f, inner, ui_state, theme),
        ActivePanel::Plugins => draw_plugins_panel_impl(f, inner, app_state, theme),
        ActivePanel::Adapters => draw_adapters_panel_impl(f, inner, app_state, theme),
        ActivePanel::Config => draw_config_panel_impl(f, inner, app_state, theme),
        ActivePanel::Engine => draw_engine_panel_impl(f, inner, app_state, theme),
    }
}

/// Independent implementation of draw_logs_panel
fn draw_logs_panel_impl(f: &mut Frame, area: Rect, ui_state: &UiState, theme: &ColorTheme) {
    if ui_state.logs.is_empty() {
        let text = Text::from(vec![
            Line::from(""),
            Line::from(" No logs yet. Waiting for events..."),
            Line::from(""),
            Line::from(" Press F1-F4 to change log level"),
        ]);
        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(theme.fg_muted))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);
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
                LogLevel::Debug => theme.info,
                LogLevel::Info => theme.success,
                LogLevel::Warn => theme.warning,
                LogLevel::Error => theme.error,
            };
            
            // Add icon based on level
            let icon = match log.level {
                LogLevel::Debug => "ℹ ",
                LogLevel::Info => "✓ ",
                LogLevel::Warn => "⚠ ",
                LogLevel::Error => "✗ ",
            };
            
            Line::from(vec![
                Span::styled(
                    timestamp,
                    Style::default().fg(theme.fg_muted)
                ),
                Span::styled(icon, Style::default().fg(level_color)),
                Span::styled(
                    level,
                    Style::default().fg(level_color).add_modifier(Modifier::BOLD)
                ),
                Span::styled(&log.message, Style::default().fg(theme.fg_primary)),
            ])
        })
        .collect();
    
    // Use scroll offset from ui_state
    let scroll_offset = ui_state.logs_scroll_offset as u16;
    let paragraph = Paragraph::new(log_lines)
        .style(Style::default().fg(theme.fg_primary))
        .wrap(Wrap { trim: true })
        .scroll((0, scroll_offset));
    
    f.render_widget(paragraph, area);
}
    
/// Independent implementation of draw_plugins_panel
fn draw_plugins_panel_impl(f: &mut Frame, area: Rect, app_state: &AppState, theme: &ColorTheme) {
    if let Some(plugin_manager) = &app_state.context.plugin_manager {
        let plugin_infos = plugin_manager.list_plugin_infos();
        
        if plugin_infos.is_empty() {
            let text_lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("📦", Style::default().fg(theme.accent)),
                    Span::styled(" Plugins Panel ", Style::default().fg(theme.fg_primary).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from("  No plugins loaded."),
                Line::from(""),
                Line::from("  Keyboard shortcuts:"),
                Line::from("    'l' - Load plugin"),
                Line::from("    Enter - View details"),
            ];
            
            let text = Text::from(text_lines);
            let paragraph = Paragraph::new(text)
                .style(Style::default().fg(theme.fg_primary))
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Left);
            
            f.render_widget(paragraph, area);
            return;
        }
        
        // Build plugin list
        let mut lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("📦", Style::default().fg(theme.accent)),
                Span::styled(" Plugins Panel ", Style::default().fg(theme.fg_primary).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
        ];
        
        // Add summary
        lines.push(Line::from(vec![
            Span::raw("  📊 Total: "),
            Span::styled(
                format!("{} ", plugin_infos.len()),
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD)
            ),
            Span::raw("✓ Active: "),
            Span::styled(
                format!("{} ", plugin_manager.active_plugin_count()),
                Style::default().fg(theme.success).add_modifier(Modifier::BOLD)
            ),
        ]));
        lines.push(Line::from(""));
        
        // Add each plugin
        for (index, plugin_info) in plugin_infos.iter().enumerate() {
            let status_color = match &plugin_info.status {
                crate::plugins::types::PluginStatus::Loaded => theme.success,
                crate::plugins::types::PluginStatus::Loading => theme.warning,
                crate::plugins::types::PluginStatus::Error { .. } => theme.error,
                crate::plugins::types::PluginStatus::Disabled => theme.fg_muted,
                crate::plugins::types::PluginStatus::Unloaded => theme.warning,
            };
            
            let status_icon = match &plugin_info.status {
                crate::plugins::types::PluginStatus::Loaded => "✓",
                crate::plugins::types::PluginStatus::Loading => "⟳",
                crate::plugins::types::PluginStatus::Error { .. } => "✗",
                crate::plugins::types::PluginStatus::Disabled => "○",
                crate::plugins::types::PluginStatus::Unloaded => "○",
            };
            
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {}. ", index + 1),
                    Style::default().fg(theme.fg_muted)
                ),
                Span::styled(
                    status_icon,
                    Style::default().fg(status_color)
                ),
                Span::styled(
                    format!("[{:?}] ", plugin_info.status),
                    Style::default().fg(status_color).add_modifier(Modifier::BOLD)
                ),
                Span::styled(
                    plugin_info.metadata.name.clone(),
                    Style::default().fg(theme.fg_primary).add_modifier(Modifier::BOLD)
                ),
                Span::raw(" v"),
                Span::styled(
                    plugin_info.metadata.version.clone(),
                    Style::default().fg(theme.warning)
                ),
            ]));
            
            lines.push(Line::from(vec![
                Span::raw("      🔧 Type: "),
                Span::styled(
                    format!("{:?} ", plugin_info.metadata.plugin_type),
                    Style::default().fg(theme.info)
                ),
                Span::raw("📁 Path: "),
                Span::styled(
                    plugin_info.metadata.entry_point.clone(),
                    Style::default().fg(theme.fg_muted)
                ),
            ]));
            lines.push(Line::from(""));
        }
        
        // Add keyboard shortcuts at the end
        lines.push(Line::from(""));
        lines.push(Line::from("  Keyboard shortcuts:"));
        lines.push(Line::from("    'l' - Load plugin"));
        lines.push(Line::from("    'u' - Unload selected plugin"));
        lines.push(Line::from("    Enter - View details"));
        
        let text = Text::from(lines);
        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(theme.fg_primary))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);
        
        f.render_widget(paragraph, area);
    } else {
        // Plugin manager not available
        let text_lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("📦", Style::default().fg(theme.accent)),
                Span::styled(" Plugins Panel ", Style::default().fg(theme.fg_primary).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from("  Plugin manager not available."),
            Line::from(""),
            Line::from("  This may indicate that plugins are not"),
            Line::from("  configured or initialized."),
        ];
        
        let text = Text::from(text_lines);
        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(theme.fg_primary))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);
        
        f.render_widget(paragraph, area);
    }
}
    
/// Independent implementation of draw_adapters_panel
fn draw_adapters_panel_impl(f: &mut Frame, area: Rect, app_state: &AppState, theme: &ColorTheme) {
    // Use cached adapter data
    let adapter_infos = &app_state.cached_adapter_infos;
    let active_count = app_state.cached_active_adapter_count;
    
    if adapter_infos.is_empty() {
        let text_lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("🔌", Style::default().fg(theme.accent)),
                Span::styled(" Adapters Panel ", Style::default().fg(theme.fg_primary).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from("  No adapters loaded."),
            Line::from(""),
            Line::from("  Keyboard shortcuts:"),
            Line::from("    'r' - Reload adapter"),
            Line::from("    'u' - Unload adapter"),
            Line::from("    Enter - View details"),
        ];
        
        let text = Text::from(text_lines);
        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(theme.fg_primary))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);
        
        f.render_widget(paragraph, area);
        return;
    }
    
    // Build adapter list
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("🔌", Style::default().fg(theme.accent)),
            Span::styled(" Adapters Panel ", Style::default().fg(theme.fg_primary).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
    ];
    
    // Add summary
    lines.push(Line::from(vec![
        Span::raw("  📊 Total: "),
        Span::styled(
            format!("{} ", adapter_infos.len()),
            Style::default().fg(theme.info).add_modifier(Modifier::BOLD)
        ),
        Span::raw("✓ Active: "),
        Span::styled(
            format!("{} ", active_count),
            Style::default().fg(theme.success).add_modifier(Modifier::BOLD)
        ),
    ]));
    lines.push(Line::from(""));
    
    // Add each adapter
    for (index, adapter_info) in adapter_infos.iter().enumerate() {
        let (status_color, status_icon) = match adapter_info.status {
            crate::adapters::core::status::AdapterStatus::Running => (theme.success, "●"),
            crate::adapters::core::status::AdapterStatus::Ready => (theme.success, "●"),
            crate::adapters::core::status::AdapterStatus::Stopped => (theme.fg_muted, "○"),
            crate::adapters::core::status::AdapterStatus::Error { .. } => (theme.error, "✗"),
            _ => (theme.warning, "?"),
        };
        
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {}. ", index + 1),
                Style::default().fg(theme.fg_muted)
            ),
            Span::styled(status_icon, Style::default().fg(status_color)),
            Span::styled(
                format!("[{:?}] ", adapter_info.status),
                Style::default().fg(status_color).add_modifier(Modifier::BOLD)
            ),
            Span::styled(
                adapter_info.name.clone(),
                Style::default().fg(theme.fg_primary).add_modifier(Modifier::BOLD)
            ),
            Span::raw(" v"),
            Span::styled(
                adapter_info.version.clone(),
                Style::default().fg(theme.warning)
            ),
        ]));
        
        lines.push(Line::from(vec![
            Span::raw("      🔧 Type: "),
            Span::styled(
                format!("{} ", adapter_info.adapter_type),
                Style::default().fg(theme.info)
            ),
            Span::raw("ID: "),
            Span::styled(
                adapter_info.adapter_id.clone(),
                Style::default().fg(theme.fg_muted)
            ),
        ]));
        
        lines.push(Line::from(""));
    }
    
    // Add keyboard shortcuts at the end
    lines.push(Line::from(""));
    lines.push(Line::from("  Keyboard shortcuts:"));
    lines.push(Line::from("    'r' - Reload adapter"));
    lines.push(Line::from("    'u' - Unload adapter"));
    lines.push(Line::from("    Enter - View details"));
    
    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(theme.fg_primary))
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);
    
    f.render_widget(paragraph, area);
}
    
/// Independent implementation of draw_config_panel
fn draw_config_panel_impl(f: &mut Frame, area: Rect, app_state: &AppState, theme: &ColorTheme) {
    let config = &app_state.context.config;
    
    // Build config display
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("⚙", Style::default().fg(theme.accent)),
            Span::styled(" Config Panel ", Style::default().fg(theme.fg_primary).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
    ];
    
    // General section
    lines.push(Line::from(vec![
        Span::styled(
            "  [General]",
            Style::default().fg(theme.info).add_modifier(Modifier::BOLD)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    🌍 Environment: "),
        Span::styled(
            config.general.environment.clone(),
            Style::default().fg(theme.warning)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    📦 Framework: "),
        Span::styled(
            config.general.name.clone(),
            Style::default().fg(theme.success)
        ),
    ]));
    lines.push(Line::from(""));
    
    // Logging section
    lines.push(Line::from(vec![
        Span::styled(
            "  [Logging]",
            Style::default().fg(theme.info).add_modifier(Modifier::BOLD)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    📊 Level: "),
        Span::styled(
            format!("{:?}", config.logging.level),
            Style::default().fg(theme.warning)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    📝 Format: "),
        Span::styled(
            format!("{:?}", config.logging.format),
            Style::default().fg(theme.success)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    📤 Output: "),
        Span::styled(
            format!("{:?}", config.logging.output),
            Style::default().fg(theme.success)
        ),
    ]));
    lines.push(Line::from(""));
    
    // Plugins section
    lines.push(Line::from(vec![
        Span::styled(
            "  [Plugins]",
            Style::default().fg(theme.info).add_modifier(Modifier::BOLD)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    📁 Directory: "),
        Span::styled(
            config.plugins.plugin_dir.clone(),
            Style::default().fg(theme.success)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    🔄 Auto-load: "),
        Span::styled(
            config.plugins.auto_load.to_string(),
            Style::default().fg(theme.warning)
        ),
    ]));
    lines.push(Line::from(""));
    
    // Adapters section
    lines.push(Line::from(vec![
        Span::styled(
            "  [Adapters]",
            Style::default().fg(theme.info).add_modifier(Modifier::BOLD)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    📁 Directory: "),
        Span::styled(
            config.adapters.adapter_dir.clone(),
            Style::default().fg(theme.success)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    🔄 Auto-load: "),
        Span::styled(
            config.adapters.auto_load.to_string(),
            Style::default().fg(theme.warning)
        ),
    ]));
    lines.push(Line::from(""));
    
    // Engine section
    lines.push(Line::from(vec![
        Span::styled(
            "  [Engine]",
            Style::default().fg(theme.info).add_modifier(Modifier::BOLD)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    🛣️ Auto-route: "),
        Span::styled(
            config.engine.auto_route.to_string(),
            Style::default().fg(theme.warning)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    📡 Auto-create-channels: "),
        Span::styled(
            config.engine.auto_create_channels.to_string(),
            Style::default().fg(theme.warning)
        ),
    ]));
    lines.push(Line::from(""));
    
    // Web section
    lines.push(Line::from(vec![
        Span::styled(
            "  [Web]",
            Style::default().fg(theme.info).add_modifier(Modifier::BOLD)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    🌐 Host: "),
        Span::styled(
            config.web.host.clone(),
            Style::default().fg(theme.success)
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    🔌 Port: "),
        Span::styled(
            config.web.port.to_string(),
            Style::default().fg(theme.warning)
        ),
    ]));
    
    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(theme.fg_primary))
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);
    
    f.render_widget(paragraph, area);
}
    
/// Independent implementation of draw_engine_panel
fn draw_engine_panel_impl(f: &mut Frame, area: Rect, app_state: &AppState, theme: &ColorTheme) {
    if let Some(engine) = &app_state.context.engine {
        let state = engine.try_state();
        let stats = engine.stats();
        let config = engine.config();
        
        // Build engine display
        let mut lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("⚡", Style::default().fg(theme.accent)),
                Span::styled(" Engine Panel ", Style::default().fg(theme.fg_primary).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
        ];
        
        // Status
        let (status_color, status_icon) = match state.status {
            crate::engine::types::EngineStatus::Running => (theme.success, "●"),
            crate::engine::types::EngineStatus::Stopped => (theme.error, "✗"),
            _ => (theme.warning, "?"),
        };
        
        lines.push(Line::from(vec![
            Span::raw("  🎯 Status: "),
            Span::styled(status_icon, Style::default().fg(status_color)),
            Span::styled(
                format!("{:?}", state.status),
                Style::default().fg(status_color).add_modifier(Modifier::BOLD)
            ),
        ]));
        lines.push(Line::from(""));
        
        // Statistics - using correct field names
        lines.push(Line::from(vec![
            Span::styled(
                "  📊 Statistics",
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD)
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    📦 Total packages: "),
            Span::styled(
                stats.total_packages.to_string(),
                Style::default().fg(theme.success)
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    ✓ Successful packages: "),
            Span::styled(
                stats.successful_packages.to_string(),
                Style::default().fg(theme.success)
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    ✗ Failed packages: "),
            Span::styled(
                stats.failed_packages.to_string(),
                Style::default().fg(theme.error)
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    🔌 Active channels: "),
            Span::styled(
                stats.active_channels.to_string(),
                Style::default().fg(theme.warning)
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    ⏱️ Avg processing time: "),
            Span::styled(
                format!("{} ms", stats.avg_processing_time_ms),
                Style::default().fg(theme.info)
            ),
        ]));
        lines.push(Line::from(""));
        
        // Configuration
        lines.push(Line::from(vec![
            Span::styled(
                "  ⚙️ Configuration",
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD)
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    🛣️ Auto-route: "),
            Span::styled(
                config.auto_route.to_string(),
                Style::default().fg(theme.warning)
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    📡 Auto-create-channels: "),
            Span::styled(
                config.auto_create_channels.to_string(),
                Style::default().fg(theme.warning)
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    🚀 Auto-initialize: "),
            Span::styled(
                config.auto_initialize.to_string(),
                Style::default().fg(theme.warning)
            ),
        ]));
        
        let text = Text::from(lines);
        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(theme.fg_primary))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);
        
        f.render_widget(paragraph, area);
    } else {
        // Engine not available
        let text_lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("⚡", Style::default().fg(theme.accent)),
                Span::styled(" Engine Panel ", Style::default().fg(theme.fg_primary).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from("  Engine not available."),
            Line::from(""),
            Line::from("  This may indicate that the engine"),
            Line::from("  is not configured or initialized."),
        ];
        
        let text = Text::from(text_lines);
        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(theme.fg_primary))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);
        
        f.render_widget(paragraph, area);
    }
}
    
/// Independent implementation of draw_command_input
fn draw_command_input_impl(f: &mut Frame, area: Rect, ui_state: &UiState) {
    let theme = ColorTheme::default();
    let input_text = format!("> {}", ui_state.command_input);
    
    let paragraph = Paragraph::new(input_text)
        .style(Style::default().fg(theme.fg_primary))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Command ")
                .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
                .border_style(Style::default().fg(theme.border_active))
                .style(Style::default().bg(theme.bg_header))
        );
    
    f.render_widget(paragraph, area);
}
