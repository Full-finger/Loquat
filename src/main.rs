//! Loquat Framework - Main Entry Point

use loquat::engine::{Engine, StandardEngine};
use loquat::logging::formatters::JsonFormatter;
use loquat::logging::writers::ConsoleWriter;
use loquat::logging::traits::{Logger, LogLevel};
use loquat::plugins::manager::{HotReloadManager, PluginManager};
use loquat::plugins::types::PluginConfig;
use loquat::errors::Result;
use std::sync::Arc;
use std::time::Duration;

/// Loquat application
struct LoquatApplication {
    plugin_manager: Arc<PluginManager>,
    hot_reload_manager: Option<Arc<HotReloadManager>>,
    logger: Arc<dyn Logger>,
}

impl LoquatApplication {
    /// Create a new Loquat application
    fn new() -> Result<Self> {
        // Initialize logger with JSON formatter and console output
        let formatter = Arc::new(JsonFormatter::new());
        let console_writer = Arc::new(ConsoleWriter::new());
        let logger = Arc::new(loquat::logging::StructuredLogger::new(formatter, console_writer));
        logger.init()?;

        // Initialize plugin manager with default config
        let plugin_config = PluginConfig::new();
        let plugin_manager = Arc::new(PluginManager::new(plugin_config));

        Ok(Self {
            plugin_manager,
            hot_reload_manager: None,
            logger,
        })
    }

    /// Run Loquat application
    async fn run(&mut self) {
        // Log startup
        self.logger.log(
            LogLevel::Info,
            "Starting Loquat Framework...",
            &Default::default(),
        );

        // Create and start engine
        let mut engine = StandardEngine::new(self.logger.clone());
        engine.start().await;

        // Auto-load plugins if configured
        let config = self.plugin_manager.config();
        if config.auto_load {
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

        // Start hot reload if configured
        if config.enable_hot_reload {
            self.logger.log(
                LogLevel::Info,
                &format!("Starting hot reload (interval: {}s)...", config.hot_reload_interval),
                &Default::default(),
            );

            let hot_reload_manager = Arc::new(HotReloadManager::new(
                self.plugin_manager.clone(),
                Duration::from_secs(config.hot_reload_interval),
            ));

            if let Err(e) = hot_reload_manager.start().await {
                self.logger.log(
                    LogLevel::Error,
                    &format!("Failed to start hot reload: {}", e),
                    &Default::default(),
                );
            } else {
                self.hot_reload_manager = Some(hot_reload_manager);
            }
        }

        // Log ready state
        self.logger.log(
            LogLevel::Info,
            "Loquat is running. Press Ctrl+C to stop.",
            &Default::default(),
        );

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

        // Stop hot reload if running
        if let Some(hot_reload) = &self.hot_reload_manager {
            let _ = hot_reload.stop().await;
            self.logger.log(
                LogLevel::Info,
                "Hot reload stopped",
                &Default::default(),
            );
        }

        // Stop engine
        engine.stop().await;

        // Log shutdown complete
        self.logger.log(
            LogLevel::Info,
            "Loquat shut down successfully.",
            &Default::default(),
        );
    }

    /// Get a reference to plugin manager
    pub fn plugin_manager(&self) -> Arc<PluginManager> {
        self.plugin_manager.clone()
    }

    /// Get a reference to logger
    pub fn logger(&self) -> Arc<dyn Logger> {
        self.logger.clone()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = LoquatApplication::new()?;

    // Run application
    app.run().await;

    Ok(())
}
