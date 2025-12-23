//! Error handling module for Loquat framework

use thiserror::Error;

pub mod types;

pub use types::*;

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, LoquatError>;

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
