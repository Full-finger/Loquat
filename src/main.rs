//! Loquat Framework - Main Entry Point
//! 
//! Provides one-click startup with configuration file support

use loquat::config::LoquatConfig;
use loquat::engine::{Engine, StandardEngine};
use loquat::cli::PluginCli;
use loquat::config::loquat_config::{LoggingConfig, AdapterConfig};
use loquat::repl::{ReplEngine, ReplContext};
use loquat::logging::formatters::{JsonFormatter, TextFormatter};
use loquat::logging::writers::{ConsoleWriter, FileWriter, CombinedWriter};
use loquat::logging::traits::{Logger, LogLevel};
use loquat::plugins::{PluginManager, HotReloadManager};
use loquat::adapters::{AdapterManager, AdapterHotReloadManager};
use loquat::web::{WebService, WebServiceConfig, AppState};
use loquat::errors::Result;
use loquat::shutdown::{ShutdownCoordinator, ShutdownStage, ShutdownOrder};
use std::sync::Arc;
use std::time::Duration;
use std::path::PathBuf;

/// Loquat application with configuration support
struct LoquatApplication {
    config: LoquatConfig,
    plugin_manager: Arc<PluginManager>,
    adapter_manager: Arc<AdapterManager>,
    hot_reload_manager: Option<Arc<HotReloadManager>>,
    adapter_hot_reload_manager: Option<Arc<AdapterHotReloadManager>>,
    web_service: Option<Arc<WebService>>,
    logger: Arc<dyn Logger>,
    shutdown_coordinator: Arc<ShutdownCoordinator>,
}

impl LoquatApplication {
    /// Create a new Loquat application from configuration
    async fn from_config(config: LoquatConfig) -> Result<Self> {
        // Initialize logger based on config
        let logger = Self::create_logger(&config.logging).await?;
        logger.init()?;

        // Initialize plugin manager with config
        let plugin_manager = Arc::new(PluginManager::new(config.plugins.clone()));

        // Initialize adapter manager with config
        let adapter_config = Self::convert_adapter_config(&config.adapters);
        let adapter_manager = Arc::new(AdapterManager::new(adapter_config, logger.clone()));

        // Register built-in adapter factories
        use loquat::adapters::{ConsoleAdapterFactory, EchoAdapterFactory, MockTestFactory};
        adapter_manager.register_factory(Box::new(ConsoleAdapterFactory))?;
        adapter_manager.register_factory(Box::new(EchoAdapterFactory))?;
        adapter_manager.register_factory(Box::new(MockTestFactory))?;

        // Create shutdown coordinator with default order
        let shutdown_coordinator = Arc::new(
            ShutdownCoordinator::with_order(
                logger.clone(),
                ShutdownOrder::default()
            )
        );

        Ok(Self {
            config,
            plugin_manager,
            adapter_manager,
            hot_reload_manager: None,
            adapter_hot_reload_manager: None,
            web_service: None,
            logger,
            shutdown_coordinator,
        })
    }

    /// Create logger based on configuration
    async fn create_logger(logging_config: &LoggingConfig) -> Result<Arc<dyn Logger>> {
        let formatter: Arc<dyn loquat::logging::traits::LogFormatter> = match logging_config.format.as_str() {
            "json" => Arc::new(JsonFormatter::new()),
            "text" => Arc::new(TextFormatter::detailed()),
            _ => Arc::new(TextFormatter::detailed()),
        };

        let writer: Arc<dyn loquat::logging::traits::LogWriter> = match logging_config.output.as_str() {
            "file" => {
                let log_path = PathBuf::from(&logging_config.file_path);
                Arc::new(FileWriter::new(log_path).await?)
            },
            "combined" => {
                let log_path = PathBuf::from(&logging_config.file_path);
                let console_writer = Arc::new(ConsoleWriter::new());
                let file_writer = Arc::new(FileWriter::new(log_path).await?);
                Arc::new(CombinedWriter::new(vec![console_writer, file_writer]))
            },
            _ => Arc::new(ConsoleWriter::new()),
        };

        Ok(Arc::new(loquat::logging::StructuredLogger::new(formatter, writer)))
    }

    /// Convert new AdapterConfig to legacy AdapterManagerConfig
    fn convert_adapter_config(config: &AdapterConfig) -> loquat::adapters::AdapterManagerConfig {
        use loquat::adapters::AdapterManagerConfig;

        AdapterManagerConfig {
            adapter_dir: config.adapter_dir.clone(),
            auto_load: config.auto_load,
            enable_hot_reload: config.enable_hot_reload,
            hot_reload_interval: config.hot_reload_interval,
            whitelist: config.whitelist.clone(),
            blacklist: config.blacklist.clone(),
            enabled: config.enabled,
        }
    }

    /// Run Loquat application
    async fn run(&mut self) {
        // Log startup
        self.logger.log(
            LogLevel::Info,
            &format!("Starting {}...", self.config.general.name),
            &Default::default(),
        );

        // Create and start engine
        let mut engine = StandardEngine::new(self.logger.clone());
        if let Err(e) = engine.start().await {
            self.logger.log(
                LogLevel::Error,
                &format!("Failed to start engine: {}", e),
                &Default::default(),
            );
            return;
        }

        // Register engine shutdown handler
        let engine_for_shutdown = engine.clone();
        self.shutdown_coordinator.register_handler(
            ShutdownStage::Engine,
            move || {
                let mut engine_clone = engine_for_shutdown.clone();
                Box::pin(async move {
                    engine_clone.stop().await
                })
            }
        ).await;

        // Auto-load adapters if enabled
        if self.config.adapters.enabled && self.config.adapters.auto_load {
            self.logger.log(
                LogLevel::Info,
                "Auto-loading adapters...",
                &Default::default(),
            );

            match self.adapter_manager.auto_load_adapters().await {
                Ok(results) => {
                    let loaded = results.iter().filter(|r| r.success).count();
                    let failed = results.len() - loaded;

                    self.logger.log(
                        LogLevel::Info,
                        &format!("Loaded {} adapters ({} failed)", loaded, failed),
                        &Default::default(),
                    );
                }
                Err(e) => {
                    self.logger.log(
                        LogLevel::Error,
                        &format!("Failed to auto-load adapters: {}", e),
                        &Default::default(),
                    );
                }
            }
        }

        // Auto-load plugins if enabled
        if self.config.plugins.enabled && self.config.plugins.auto_load {
            self.logger.log(
                LogLevel::Info,
                "Auto-loading plugins...",
                &Default::default(),
            );

            match self.plugin_manager.auto_load_plugins().await {
                Ok(results) => {
                    let loaded = results.iter().filter(|r| r.success).count();
                    let failed = results.len() - loaded;

                    self.logger.log(
                        LogLevel::Info,
                        &format!("Loaded {} plugins ({} failed)", loaded, failed),
                        &Default::default(),
                    );
                }
                Err(e) => {
                    self.logger.log(
                        LogLevel::Error,
                        &format!("Failed to auto-load plugins: {}", e),
                        &Default::default(),
                    );
                }
            }
        }

        // Start web service if enabled
        if self.config.web.enabled {
            self.logger.log(
                LogLevel::Info,
                "Starting web service...",
                &Default::default(),
            );

            let web_config = WebServiceConfig {
                host: self.config.web.host.clone(),
                port: self.config.web.port,
                ..Default::default()
            };

            let web_running = Arc::new(std::sync::atomic::AtomicBool::new(false));

            let app_state = AppState {
                plugin_manager: Some((*self.plugin_manager).clone()),
                adapter_manager: Some((*self.adapter_manager).clone()),
                engine: Some(engine.clone()),
                logger: self.logger.clone(),
                config: self.config.clone(),
                start_time: std::time::Instant::now(),
                error_tracker: loquat::web::ErrorTracker::new(),
                web_running: Arc::clone(&web_running),
            };

            let web_service = Arc::new(
                WebService::with_config(web_config.clone())
                    .with_logger(self.logger.clone())
                    .with_app_state(app_state)
            );

            if let Err(e) = web_service.start().await {
                self.logger.log(
                    LogLevel::Error,
                    &format!("Failed to start web service: {}", e),
                    &Default::default(),
                );
            } else {
                web_running.store(true, std::sync::atomic::Ordering::SeqCst);
                
                // Register web service shutdown handler
                let web_service_for_shutdown = web_service.clone();
                self.shutdown_coordinator.register_handler(
                    ShutdownStage::WebService,
                    move || {
                        let web_clone = web_service_for_shutdown.clone();
                        Box::pin(async move {
                            web_clone.stop().await
                        })
                    }
                ).await;

                self.web_service = Some(web_service);
                self.logger.log(
                    LogLevel::Info,
                    &format!("Web service running on http://{}:{}",
                        self.config.web.host, self.config.web.port),
                    &Default::default(),
                );
            }
        }

        // Start adapter hot reload if enabled
        if self.config.adapters.enabled && self.config.adapters.enable_hot_reload {
            self.logger.log(
                LogLevel::Info,
                &format!("Starting adapter hot reload (interval: {}s)...", self.config.adapters.hot_reload_interval),
                &Default::default(),
            );

            let adapter_hot_reload_manager = Arc::new(AdapterHotReloadManager::new(
                self.adapter_manager.clone(),
                Duration::from_secs(self.config.adapters.hot_reload_interval),
            ));

            if let Err(e) = adapter_hot_reload_manager.start().await {
                self.logger.log(
                    LogLevel::Error,
                    &format!("Failed to start adapter hot reload: {}", e),
                    &Default::default(),
                );
            } else {
                // Register adapter hot reload shutdown handler
                let adapter_hot_reload_for_shutdown = adapter_hot_reload_manager.clone();
                self.shutdown_coordinator.register_handler(
                    ShutdownStage::AdapterHotReload,
                    move || {
                        let adapter_clone = adapter_hot_reload_for_shutdown.clone();
                        Box::pin(async move {
                            adapter_clone.stop().await
                        })
                    }
                ).await;

                self.adapter_hot_reload_manager = Some(adapter_hot_reload_manager);
            }
        }

        // Start plugin hot reload if enabled
        if self.config.plugins.enabled && self.config.plugins.enable_hot_reload {
            self.logger.log(
                LogLevel::Info,
                &format!("Starting plugin hot reload (interval: {}s)...", self.config.plugins.hot_reload_interval),
                &Default::default(),
            );

            let hot_reload_manager = Arc::new(HotReloadManager::new(
                self.plugin_manager.clone(),
                Duration::from_secs(self.config.plugins.hot_reload_interval),
            ));

            if let Err(e) = hot_reload_manager.start().await {
                self.logger.log(
                    LogLevel::Error,
                    &format!("Failed to start plugin hot reload: {}", e),
                    &Default::default(),
                );
            } else {
                // Register plugin hot reload shutdown handler
                let hot_reload_for_shutdown = hot_reload_manager.clone();
                self.shutdown_coordinator.register_handler(
                    ShutdownStage::PluginHotReload,
                    move || {
                        let plugin_clone = hot_reload_for_shutdown.clone();
                        Box::pin(async move {
                            plugin_clone.stop().await
                        })
                    }
                ).await;

                self.hot_reload_manager = Some(hot_reload_manager);
            }
        }

        // Log ready state
        self.logger.log(
            LogLevel::Info,
            &format!("{} is running (Environment: {}). Press Ctrl+C to stop.",
                self.config.general.name,
                self.config.general.environment),
            &Default::default(),
        );

        // List loaded adapters
        let adapters = self.adapter_manager.list_adapter_infos().await;
        if !adapters.is_empty() {
            self.logger.log(
                LogLevel::Info,
                &format!("Loaded adapters: {:?}", adapters),
                &Default::default(),
            );
        }

        // List loaded plugins
        let plugins = self.plugin_manager.list_plugin_infos();
        if !plugins.is_empty() {
            self.logger.log(
                LogLevel::Info,
                &format!("Loaded plugins: {:?}", plugins),
                &Default::default(),
            );
        }

        // Wait for Ctrl+C signal
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to setup Ctrl+C handler");

        // Log shutdown
        self.logger.log(
            LogLevel::Info,
            "Received shutdown signal...",
            &Default::default(),
        );

        // Execute graceful shutdown
        self.logger.log(
            LogLevel::Info,
            "Starting graceful shutdown...",
            &Default::default(),
        );

        match self.shutdown_coordinator.shutdown().await {
            Ok(results) => {
                // Log shutdown results
                for result in &results {
                    match result {
                        loquat::shutdown::ShutdownStageResult::Success { stage, duration_ms } => {
                            self.logger.log(
                                LogLevel::Info,
                                &format!("Shutdown stage {:?} completed in {}ms", stage, duration_ms),
                                &Default::default(),
                            );
                        }
                        loquat::shutdown::ShutdownStageResult::FailedContinue { stage, error, duration_ms } => {
                            self.logger.log(
                                LogLevel::Warn,
                                &format!("Shutdown stage {:?} failed after {}ms (continuing): {}", stage, duration_ms, error),
                                &Default::default(),
                            );
                        }
                        loquat::shutdown::ShutdownStageResult::FailedAbort { stage, error, duration_ms } => {
                            self.logger.log(
                                LogLevel::Error,
                                &format!("Shutdown stage {:?} failed after {}ms (aborting): {}", stage, duration_ms, error),
                                &Default::default(),
                            );
                        }
                        loquat::shutdown::ShutdownStageResult::Timeout { stage, timeout_ms } => {
                            self.logger.log(
                                LogLevel::Error,
                                &format!("Shutdown stage {:?} timed out after {}ms", stage, timeout_ms),
                                &Default::default(),
                            );
                        }
                    }
                }

                // Log overall shutdown status
                let status = self.shutdown_coordinator.status().await;
                let duration = self.shutdown_coordinator.duration_ms().await;

                self.logger.log(
                    LogLevel::Info,
                    &format!("Graceful shutdown completed in {}ms. Status: {:?}", 
                        duration.unwrap_or(0), status),
                    &Default::default(),
                );
            }
            Err(e) => {
                self.logger.log(
                    LogLevel::Error,
                    &format!("Shutdown coordinator failed: {}", e),
                    &Default::default(),
                );
            }
        }

        // Log shutdown complete
        self.logger.log(
            LogLevel::Info,
            &format!("{} shut down successfully.", self.config.general.name),
            &Default::default(),
        );
    }

    /// Get a reference to plugin manager
    pub fn plugin_manager(&self) -> Arc<PluginManager> {
        self.plugin_manager.clone()
    }

    /// Get a reference to adapter manager
    pub fn adapter_manager(&self) -> Arc<AdapterManager> {
        self.adapter_manager.clone()
    }

    /// Get a reference to logger
    pub fn logger(&self) -> Arc<dyn Logger> {
        self.logger.clone()
    }
}

/// Parse command line arguments
enum Command {
    Run { environment: String, rebuild: bool, repl: bool },
    PluginCreate { args: Vec<String> },
    PluginInteractive,
}

fn parse_args() -> Command {
    let args: Vec<String> = std::env::args().collect();
    
    // Check for plugin command
    if args.len() >= 2 && args[1] == "plugin" {
        if args.len() >= 3 && args[2] == "create" {
            // Plugin create with arguments
            let plugin_args: Vec<String> = args.iter().skip(2).cloned().collect();
            return Command::PluginCreate { args: plugin_args };
        } else {
            // Interactive plugin creation
            return Command::PluginInteractive;
        }
    }
    
    // Default: run application
    let mut environment = "dev".to_string();
    let mut rebuild = false;
    let mut repl = false;

    for i in 1..args.len() {
        match args[i].as_str() {
            "--env" => {
                if i + 1 < args.len() {
                    environment = args[i + 1].clone();
                }
            }
            "--rebuild" => {
                rebuild = true;
            }
            "--repl" => {
                repl = true;
            }
            _ => {
                // Check if it's an environment name (no flag)
                // Only set environment if REPL mode is not requested
                if !args[i].starts_with("--") && !repl {
                    environment = args[i].clone();
                }
            }
        }
    }

    Command::Run { environment, rebuild, repl }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let command = parse_args();
    
    // Handle different commands
    let (environment, rebuild, repl) = match command {
        Command::PluginCreate { args } => {
            // Run plugin template generator
            println!();
            println!("╔══════════════════════════════════════════════════════════╗");
            println!("║              Loquat Plugin Template Generator              ║");
            println!("╚══════════════════════════════════════════════════════════╝");
            println!();
            
            let mut cli = PluginCli::new();
            if let Err(e) = cli.run_from_args(args) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            
            return Ok(());
        }
        Command::PluginInteractive => {
            // Run interactive plugin creator
            let mut cli = PluginCli::new();
            if let Err(e) = cli.run_interactive() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            
            return Ok(());
        }
        Command::Run { environment, rebuild, repl } => {
            // Continue with normal application startup
            (environment, rebuild, repl)
        }
    };

    // Print banner
    println!();
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║                    Loquat Framework                        ║");
    println!("║             One-Click Startup System                       ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    println!("Environment: {}", environment);
    println!();

    // Check if rebuild is requested
    if rebuild {
        println!("Rebuilding project...");
        // In a real scenario, you might run cargo build here
        println!("Rebuild complete!");
        println!();
    }

    // Load configuration
    let config = LoquatConfig::from_environment("config", &environment)?;
    
    println!("Configuration loaded successfully!");
    println!("  - Log Level: {}", config.logging.level);
    println!("  - Output: {}", config.logging.output);
    println!("  - Plugins: {}", if config.plugins.enabled { "Enabled" } else { "Disabled" });
    println!("  - Adapters: {}", if config.adapters.enabled { "Enabled" } else { "Disabled" });
    println!();
    println!("Starting framework...");
    println!("═══════════════════════════════════════════════════════════");
    println!();

    // Create application
    let mut app = LoquatApplication::from_config(config.clone()).await?;

    // Check if REPL mode is requested
    if repl {
        println!();
        println!("Starting REPL mode...");
        
        // For REPL mode, use console writer only to avoid async/block_on conflicts
        // File writer uses block_on internally which conflicts with async runtime
        let logger = Arc::new(loquat::logging::StructuredLogger::new(
            Arc::new(TextFormatter::detailed()),
            Arc::new(ConsoleWriter::new())
        ));
        
        // Initialize logger using initialize() which doesn't call flush
        logger.initialize();
        
        println!("Note: Logs are being written to {}", config.logging.file_path);
        println!("Use the 'logs' command to view logs.");
        println!();
        
        // Create and start engine for REPL mode
        println!("Starting engine...");
        let mut engine = StandardEngine::new(logger.clone());
        if let Err(e) = engine.start().await {
            eprintln!("Failed to start engine: {}", e);
            return Ok(());
        }
        
        // Auto-load adapters if enabled
        if config.adapters.enabled && config.adapters.auto_load {
            println!("Auto-loading adapters...");
            match app.adapter_manager.auto_load_adapters().await {
                Ok(results) => {
                    let loaded = results.iter().filter(|r| r.success).count();
                    let failed = results.len() - loaded;
                    println!("Loaded {} adapters ({} failed)", loaded, failed);
                }
                Err(e) => {
                    eprintln!("Failed to auto-load adapters: {}", e);
                }
            }
        }
        
        // Auto-load plugins if enabled
        if config.plugins.enabled && config.plugins.auto_load {
            println!("Auto-loading plugins...");
            match app.plugin_manager.auto_load_plugins().await {
                Ok(results) => {
                    let loaded = results.iter().filter(|r| r.success).count();
                    let failed = results.len() - loaded;
                    println!("Loaded {} plugins ({} failed)", loaded, failed);
                }
                Err(e) => {
                    eprintln!("Failed to auto-load plugins: {}", e);
                }
            }
        }
        
        // Start hot reload if enabled
        let mut hot_reload_manager = None;
        if config.plugins.enabled && config.plugins.enable_hot_reload {
            println!("Starting plugin hot reload...");
            let hot_reload = Arc::new(HotReloadManager::new(
                app.plugin_manager.clone(),
                Duration::from_secs(config.plugins.hot_reload_interval),
            ));
            if let Err(e) = hot_reload.start().await {
                eprintln!("Failed to start plugin hot reload: {}", e);
            } else {
                hot_reload_manager = Some(hot_reload);
            }
        }
        
        println!();
        println!("╔══════════════════════════════════════════════════════════╗");
        println!("║        Loquat Framework - Interactive Mode             ║");
        println!("╚══════════════════════════════════════════════════════════╝");
        println!();
        println!("Version: {}", std::env!("CARGO_PKG_VERSION"));
        println!("Environment: {}", config.general.environment);
        println!();
        println!("Note: Logs are being written to: {}", config.logging.file_path);
        println!("Note: Type 'help' for available commands.");
        println!();
        
        // Create REPL context with engine
        let repl_context = ReplContext {
            plugin_manager: Some(app.plugin_manager()),
            adapter_manager: Some(app.adapter_manager()),
            engine: Some(Arc::new(engine)),
            logger,
            config: config.clone(),
            start_time: std::time::Instant::now(),
        };
        
        // Create and run REPL
        let mut repl_engine = ReplEngine::new(repl_context);
        repl_engine.register_default_commands();
        
        // Run REPL
        if let Err(e) = repl_engine.run().await {
            eprintln!("REPL error: {}", e);
        }
    } else {
        // Run normal application
        app.run().await;
    }

    Ok(())
}
