//! Error handling module for Loquat framework

use thiserror::Error;

pub mod types;

pub use types::*;

/// Result type alias for convenience (2-arg variant for compatibility)
pub type Result<T, E = LoquatError> = std::result::Result<T, E>;

/// Main error type for the Loquat framework
#[derive(Debug, Clone, thiserror::Error)]
pub enum LoquatError {
    #[error("Logging error: {0}")]
    Logging(#[from] LoggingError),
    
    #[error("AOP error: {0}")]
    Aop(#[from] AopError),
    
    #[error("Web error: {0}")]
    Web(#[from] WebError),
    
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("Plugin error: {0}")]
    Plugin(#[from] PluginError),
    
    #[error("Channel error: {0}")]
    Channel(#[from] ChannelError),
    
    #[error("IO error: {0}")]
    Io(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Logging related errors
#[derive(Error, Debug, Clone)]
pub enum LoggingError {
    #[error("Failed to initialize logger: {0}")]
    Initialization(String),
    
    #[error("Failed to write log: {0}")]
    WriteError(String),
    
    #[error("Invalid log level: {0}")]
    InvalidLevel(String),
    
    #[error("Logger configuration error: {0}")]
    Configuration(String),
}

/// AOP related errors
#[derive(Error, Debug, Clone)]
pub enum AopError {
    #[error("Aspect execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Invalid aspect configuration: {0}")]
    InvalidConfiguration(String),
    
    #[error("Proxy creation failed: {0}")]
    ProxyCreation(String),
}

/// Web service related errors
#[derive(Error, Debug, Clone)]
pub enum WebError {
    #[error("Request handling failed: {0}")]
    RequestHandling(String),
    
    #[error("Middleware error: {0}")]
    Middleware(String),
    
    #[error("Response error: {0}")]
    Response(String),
}

/// Configuration related errors
#[derive(Error, Debug, Clone)]
pub enum ConfigError {
    #[error("Failed to load configuration: {0}")]
    LoadError(String),
    
    #[error("Invalid configuration format: {0}")]
    InvalidFormat(String),
    
    #[error("Missing required configuration: {0}")]
    MissingRequired(String),
}

/// Plugin related errors
#[derive(Error, Debug, Clone)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),
    
    #[error("Plugin load failed: {0}")]
    LoadFailed(String),
    
    #[error("Plugin unload failed: {0}")]
    UnloadFailed(String),
    
    #[error("Plugin init failed: {0}")]
    InitFailed(String),
    
    #[error("Plugin reload failed: {0}")]
    ReloadFailed(String),
    
    #[error("Invalid plugin configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Plugin dependency not satisfied: {0}")]
    DependencyNotSatisfied(String),
    
    #[error("Plugin already loaded: {0}")]
    AlreadyLoaded(String),
    
    #[error("Plugin already exists: {0}")]
    AlreadyExists(String),
    
    #[error("Plugin is disabled: {0}")]
    Disabled(String),
    
    #[error("Hot reload error: {0}")]
    HotReloadError(String),
    
    #[error("Unsupported plugin type: {0}")]
    UnsupportedType(String),
    
    #[error("Registry error: {0}")]
    RegistryError(String),
}

/// Channel related errors
#[derive(Error, Debug, Clone)]
pub enum ChannelError {
    #[error("Channel not found: {0}")]
    NotFound(String),
    
    #[error("Max channels ({0}) reached")]
    MaxChannelsReached(usize),
    
    #[error("Channel creation failed: {0}")]
    CreationFailed(String),
    
    #[error("Channel removal failed: {0}")]
    RemovalFailed(String),
}
