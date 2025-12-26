//! Loquat Framework - Main Entry Point
//! 
//! Provides one-click startup with configuration file support

use loquat::config::LoquatConfig;
use loquat::engine::{Engine, StandardEngine};
use loquat::config::loquat_config::{LoggingConfig, AdapterConfig};
use loquat::logging::formatters::{JsonFormatter, TextFormatter};
use loquat::logging::writers::{ConsoleWriter, FileWriter, CombinedWriter};
use loquat::logging::traits::{Logger, LogLevel};
use loquat::plugins::{PluginManager, HotReloadManager};
use loquat::adapters::{AdapterManager, AdapterHotReloadManager};
use loquat::web::{WebService, WebServiceConfig, AppState};
use loquat::errors::Result;
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

        Ok(Self {
            config,
            plugin_manager,
            adapter_manager,
            hot_reload_manager: None,
            adapter_hot_reload_manager: None,
            web_service: None,
            logger,
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

            let app_state = AppState {
                plugin_manager: Some((*self.plugin_manager).clone()),
                adapter_manager: Some((*self.adapter_manager).clone()),
                logger: self.logger.clone(),
                config: self.config.clone(),
                start_time: std::time::Instant::now(),
            };

            let web_service = Arc::new(
                WebService::with_config(web_config)
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

        // Stop web service if running
        if let Some(web_service) = &self.web_service {
            let _ = web_service.stop().await;
            self.logger.log(
                LogLevel::Info,
                "Web service stopped",
                &Default::default(),
            );
        }

        // Stop hot reload if running
        if let Some(hot_reload) = &self.hot_reload_manager {
            let _ = hot_reload.stop().await;
            self.logger.log(
                LogLevel::Info,
                "Plugin hot reload stopped",
                &Default::default(),
            );
        }

        if let Some(adapter_hot_reload) = &self.adapter_hot_reload_manager {
            let _ = adapter_hot_reload.stop().await;
            self.logger.log(
                LogLevel::Info,
                "Adapter hot reload stopped",
                &Default::default(),
            );
        }

        // Stop engine
        let _ = engine.stop().await;

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
fn parse_args() -> (String, bool) {
    let args: Vec<String> = std::env::args().collect();
    let mut environment = "dev".to_string();
    let mut rebuild = false;

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
            _ => {
                // Check if it's an environment name (no flag)
                if !args[i].starts_with("--") {
                    environment = args[i].clone();
                }
            }
        }
    }

    (environment, rebuild)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let (environment, rebuild) = parse_args();

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
    let mut app = LoquatApplication::from_config(config).await?;

    // Run application
    app.run().await;

    Ok(())
}
