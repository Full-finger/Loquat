//! Web service module for Loquat framework

mod types;
mod traits;
mod handlers;

use crate::errors::{Result, WebError};
use crate::logging::traits::Logger;
use std::sync::Arc;
use tokio::net::TcpListener;
use axum::{
    Router,
    routing::{get, post},
};
use tower_http::cors::{CorsLayer, Any};

use types::*;
use traits::*;

// Re-export AppState for external use
pub use traits::AppState;

/// Main web service structure
pub struct WebService {
    config: WebServiceConfig,
    logger: Option<Arc<dyn Logger>>,
    app_state: Option<AppState>,
    running: Arc<std::sync::atomic::AtomicBool>,
    listener_handle: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl WebService {
    /// Create a new web service with default configuration
    pub fn new() -> Self {
        Self {
            config: WebServiceConfig::default(),
            logger: None,
            app_state: None,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            listener_handle: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// Create a new web service with configuration
    pub fn with_config(config: WebServiceConfig) -> Self {
        Self {
            config,
            logger: None,
            app_state: None,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            listener_handle: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// Set logger for web service
    pub fn with_logger(mut self, logger: Arc<dyn Logger>) -> Self {
        self.logger = Some(logger);
        self
    }

    /// Set app state for web service
    pub fn with_app_state(mut self, app_state: AppState) -> Self {
        self.app_state = Some(app_state);
        self
    }

    /// Create router with all routes
    fn create_router(&self) -> Router {
        let app_state = self.app_state.as_ref().expect("App state must be set").clone();

        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        Router::new()
            .route("/", get(handlers::welcome))
            .route("/health", get(handlers::health_check))
            .route("/api/plugins", get(handlers::list_plugins))
            .route("/api/plugins/:name", get(handlers::get_plugin))
            .route("/api/plugins/reload", post(handlers::reload_plugins))
            .route("/api/adapters", get(handlers::list_adapters))
            .route("/api/adapters/:name", get(handlers::get_adapter))
            .route("/api/adapters/reload", post(handlers::reload_adapters))
            .route("/api/reload", post(handlers::reload_all))
            .route("/api/config", get(handlers::get_config))
            .layer(cors)
            .with_state(app_state)
    }

    /// Start the web service
    pub async fn start(&self) -> Result<()> {
        if self.app_state.is_none() {
            return Err(WebError::Startup("App state is not set".to_string()).into());
        }

        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await.map_err(|e| {
            WebError::Startup(format!("Failed to bind to {}: {}", addr, e))
        })?;

        let running = Arc::clone(&self.running);
        running.store(true, std::sync::atomic::Ordering::SeqCst);

        if let Some(logger) = &self.logger {
            logger.info(&format!("Starting web service on {}", addr));
        }

        let router = self.create_router();
        
        // Graceful shutdown signal
        let logger_for_shutdown = self.logger.clone();
        let shutdown_signal = async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to setup Ctrl+C handler");
            
            if let Some(logger) = &logger_for_shutdown {
                logger.info("Web service received shutdown signal");
            }
        };

        // Spawn server in a task
        let handle = tokio::spawn(async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(shutdown_signal)
                .await
                .map_err(|e| eprintln!("Web server error: {}", e));
        });

        // Store the handle
        let mut listener_handle = self.listener_handle.lock().await;
        *listener_handle = Some(handle);

        if let Some(logger) = &self.logger {
            logger.info(&format!("Web service is running on {}", addr));
        }

        Ok(())
    }

    /// Stop the web service
    pub async fn stop(&self) -> Result<()> {
        if let Some(logger) = &self.logger {
            logger.info("Stopping web service");
        }

        let running = Arc::clone(&self.running);
        running.store(false, std::sync::atomic::Ordering::SeqCst);

        // Wait for the listener handle to finish
        let mut listener_handle = self.listener_handle.lock().await;
        if let Some(handle) = listener_handle.take() {
            let _ = handle.await;
        }

        if let Some(logger) = &self.logger {
            logger.info("Web service stopped");
        }

        Ok(())
    }

    /// Check if the web service is running
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get the web service address
    pub fn address(&self) -> String {
        format!("{}:{}", self.config.host, self.config.port)
    }
}

#[async_trait::async_trait]
impl WebServiceTrait for WebService {
    async fn start(&self) -> Result<()> {
        self.start().await
    }

    async fn stop(&self) -> Result<()> {
        self.stop().await
    }

    fn is_running(&self) -> bool {
        self.is_running()
    }

    fn address(&self) -> String {
        self.address()
    }
}

impl std::fmt::Debug for WebService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebService")
            .field("config", &self.config)
            .field("logger", &self.logger.as_ref().map(|_| "Logger present"))
            .field("app_state", &self.app_state.as_ref().map(|_| "AppState present"))
            .field("running", &self.is_running())
            .finish()
    }
}

impl Default for WebService {
    fn default() -> Self {
        Self::new()
    }
}

/// Web service configuration
#[derive(Debug, Clone)]
pub struct WebServiceConfig {
    /// Server host
    pub host: String,
    /// Server port
    pub port: u16,
    /// Whether to enable HTTPS
    pub https: bool,
    /// Request timeout in seconds
    pub request_timeout: u64,
    /// Maximum request body size in bytes
    pub max_request_size: usize,
    /// Whether to enable CORS
    pub enable_cors: bool,
    /// Allowed CORS origins
    pub cors_origins: Vec<String>,
}

impl Default for WebServiceConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            https: false,
            request_timeout: 30,
            max_request_size: 10 * 1024 * 1024, // 10MB
            enable_cors: true,
            cors_origins: vec!["*".to_string()],
        }
    }
}

/// HTTP request representation
#[derive(Debug, Clone)]
pub struct Request {
    /// HTTP method
    pub method: HttpMethod,
    /// Request path
    pub path: String,
    /// Request headers
    pub headers: std::collections::HashMap<String, String>,
    /// Request body
    pub body: Option<Vec<u8>>,
    /// Query parameters
    pub query: std::collections::HashMap<String, String>,
}

impl Request {
    /// Create a new request
    pub fn new(method: HttpMethod, path: &str) -> Self {
        Self {
            method,
            path: path.to_string(),
            headers: std::collections::HashMap::new(),
            body: None,
            query: std::collections::HashMap::new(),
        }
    }

    /// Add a header
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Set the request body
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    /// Add a query parameter
    pub fn with_query(mut self, key: &str, value: &str) -> Self {
        self.query.insert(key.to_string(), value.to_string());
        self
    }
}

/// HTTP response representation
#[derive(Debug, Clone)]
pub struct Response {
    /// HTTP status code
    pub status: HttpStatus,
    /// Response headers
    pub headers: std::collections::HashMap<String, String>,
    /// Response body
    pub body: Option<Vec<u8>>,
}

impl Response {
    /// Create a new response
    pub fn new(status: HttpStatus) -> Self {
        Self {
            status,
            headers: std::collections::HashMap::new(),
            body: None,
        }
    }

    /// Add a header
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Set the response body
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    /// Create a JSON response
    pub fn json<T: serde::Serialize>(status: HttpStatus, data: &T) -> Result<Self> {
        let json_bytes = serde_json::to_vec(data)
            .map_err(|e| WebError::Response(format!("Failed to serialize JSON: {}", e)))?;
        
        Ok(Self::new(status)
            .with_header("Content-Type", "application/json")
            .with_body(json_bytes))
    }

    /// Create a text response
    pub fn text(status: HttpStatus, text: &str) -> Self {
        Self::new(status)
            .with_header("Content-Type", "text/plain; charset=utf-8")
            .with_body(text.as_bytes().to_vec())
    }

    /// Create an HTML response
    pub fn html(status: HttpStatus, html: &str) -> Self {
        Self::new(status)
            .with_header("Content-Type", "text/html; charset=utf-8")
            .with_body(html.as_bytes().to_vec())
    }
}

/// HTTP methods
#[derive(Debug, Clone, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl HttpMethod {
    /// Get the method as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        }
    }

    /// Parse method from string
    pub fn from_str(method: &str) -> Option<Self> {
        match method.to_uppercase().as_str() {
            "GET" => Some(HttpMethod::Get),
            "POST" => Some(HttpMethod::Post),
            "PUT" => Some(HttpMethod::Put),
            "DELETE" => Some(HttpMethod::Delete),
            "PATCH" => Some(HttpMethod::Patch),
            "HEAD" => Some(HttpMethod::Head),
            "OPTIONS" => Some(HttpMethod::Options),
            _ => None,
        }
    }
}

/// HTTP status codes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HttpStatus {
    Continue = 100,
    SwitchingProtocols = 101,
    Processing = 102,
    EarlyHints = 103,
    
    Ok = 200,
    Created = 201,
    Accepted = 202,
    NonAuthoritativeInformation = 203,
    NoContent = 204,
    ResetContent = 205,
    PartialContent = 206,
    
    MultipleChoices = 300,
    MovedPermanently = 301,
    Found = 302,
    SeeOther = 303,
    NotModified = 304,
    UseProxy = 305,
    TemporaryRedirect = 307,
    PermanentRedirect = 308,
    
    BadRequest = 400,
    Unauthorized = 401,
    PaymentRequired = 402,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    NotAcceptable = 406,
    ProxyAuthenticationRequired = 407,
    RequestTimeout = 408,
    Conflict = 409,
    Gone = 410,
    LengthRequired = 411,
    PreconditionFailed = 412,
    PayloadTooLarge = 413,
    UriTooLong = 414,
    UnsupportedMediaType = 415,
    RangeNotSatisfiable = 416,
    ExpectationFailed = 417,
    ImATeapot = 418,
    MisdirectedRequest = 421,
    UnprocessableEntity = 422,
    Locked = 423,
    FailedDependency = 424,
    TooEarly = 425,
    UpgradeRequired = 426,
    PreconditionRequired = 428,
    TooManyRequests = 429,
    RequestHeaderFieldsTooLarge = 431,
    UnavailableForLegalReasons = 451,
    
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
    GatewayTimeout = 504,
    HttpVersionNotSupported = 505,
    VariantAlsoNegotiates = 506,
    InsufficientStorage = 507,
    LoopDetected = 508,
    NotExtended = 510,
    NetworkAuthenticationRequired = 511,
}

impl HttpStatus {
    /// Get the status code as a number
    pub fn as_u16(&self) -> u16 {
        *self as u16
    }

    /// Get the reason phrase
    pub fn reason_phrase(&self) -> &'static str {
        match self {
            HttpStatus::Continue => "Continue",
            HttpStatus::SwitchingProtocols => "Switching Protocols",
            HttpStatus::Processing => "Processing",
            HttpStatus::EarlyHints => "Early Hints",
            
            HttpStatus::Ok => "OK",
            HttpStatus::Created => "Created",
            HttpStatus::Accepted => "Accepted",
            HttpStatus::NonAuthoritativeInformation => "Non-Authoritative Information",
            HttpStatus::NoContent => "No Content",
            HttpStatus::ResetContent => "Reset Content",
            HttpStatus::PartialContent => "Partial Content",
            
            HttpStatus::MultipleChoices => "Multiple Choices",
            HttpStatus::MovedPermanently => "Moved Permanently",
            HttpStatus::Found => "Found",
            HttpStatus::SeeOther => "See Other",
            HttpStatus::NotModified => "Not Modified",
            HttpStatus::UseProxy => "Use Proxy",
            HttpStatus::TemporaryRedirect => "Temporary Redirect",
            HttpStatus::PermanentRedirect => "Permanent Redirect",
            
            HttpStatus::BadRequest => "Bad Request",
            HttpStatus::Unauthorized => "Unauthorized",
            HttpStatus::PaymentRequired => "Payment Required",
            HttpStatus::Forbidden => "Forbidden",
            HttpStatus::NotFound => "Not Found",
            HttpStatus::MethodNotAllowed => "Method Not Allowed",
            HttpStatus::NotAcceptable => "Not Acceptable",
            HttpStatus::ProxyAuthenticationRequired => "Proxy Authentication Required",
            HttpStatus::RequestTimeout => "Request Timeout",
            HttpStatus::Conflict => "Conflict",
            HttpStatus::Gone => "Gone",
            HttpStatus::LengthRequired => "Length Required",
            HttpStatus::PreconditionFailed => "Precondition Failed",
            HttpStatus::PayloadTooLarge => "Payload Too Large",
            HttpStatus::UriTooLong => "URI Too Long",
            HttpStatus::UnsupportedMediaType => "Unsupported Media Type",
            HttpStatus::RangeNotSatisfiable => "Range Not Satisfiable",
            HttpStatus::ExpectationFailed => "Expectation Failed",
            HttpStatus::ImATeapot => "I'm a teapot",
            HttpStatus::MisdirectedRequest => "Misdirected Request",
            HttpStatus::UnprocessableEntity => "Unprocessable Entity",
            HttpStatus::Locked => "Locked",
            HttpStatus::FailedDependency => "Failed Dependency",
            HttpStatus::TooEarly => "Too Early",
            HttpStatus::UpgradeRequired => "Upgrade Required",
            HttpStatus::PreconditionRequired => "Precondition Required",
            HttpStatus::TooManyRequests => "Too Many Requests",
            HttpStatus::RequestHeaderFieldsTooLarge => "Request Header Fields Too Large",
            HttpStatus::UnavailableForLegalReasons => "Unavailable For Legal Reasons",
            
            HttpStatus::InternalServerError => "Internal Server Error",
            HttpStatus::NotImplemented => "Not Implemented",
            HttpStatus::BadGateway => "Bad Gateway",
            HttpStatus::ServiceUnavailable => "Service Unavailable",
            HttpStatus::GatewayTimeout => "Gateway Timeout",
            HttpStatus::HttpVersionNotSupported => "HTTP Version Not Supported",
            HttpStatus::VariantAlsoNegotiates => "Variant Also Negotiates",
            HttpStatus::InsufficientStorage => "Insufficient Storage",
            HttpStatus::LoopDetected => "Loop Detected",
            HttpStatus::NotExtended => "Not Extended",
            HttpStatus::NetworkAuthenticationRequired => "Network Authentication Required",
        }
    }

    /// Check if the status is successful (2xx)
    pub fn is_success(&self) -> bool {
        matches!(self, HttpStatus::Ok | HttpStatus::Created | HttpStatus::Accepted | 
                  HttpStatus::NonAuthoritativeInformation | HttpStatus::NoContent | 
                  HttpStatus::ResetContent | HttpStatus::PartialContent)
    }

    /// Check if the status is a redirect (3xx)
    pub fn is_redirect(&self) -> bool {
        matches!(self, HttpStatus::MultipleChoices | HttpStatus::MovedPermanently | 
                  HttpStatus::Found | HttpStatus::SeeOther | HttpStatus::NotModified | 
                  HttpStatus::UseProxy | HttpStatus::TemporaryRedirect | 
                  HttpStatus::PermanentRedirect)
    }

    /// Check if the status is a client error (4xx)
    pub fn is_client_error(&self) -> bool {
        let code = self.as_u16();
        code >= 400 && code < 500
    }

    /// Check if the status is a server error (5xx)
    pub fn is_server_error(&self) -> bool {
        let code = self.as_u16();
        code >= 500 && code < 600
    }

    /// Check if the status is an error (4xx or 5xx)
    pub fn is_error(&self) -> bool {
        self.is_client_error() || self.is_server_error()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_method() {
        assert_eq!(HttpMethod::Get.as_str(), "GET");
        assert_eq!(HttpMethod::from_str("GET"), Some(HttpMethod::Get));
        assert_eq!(HttpMethod::from_str("get"), Some(HttpMethod::Get));
        assert_eq!(HttpMethod::from_str("INVALID"), None);
    }

    #[test]
    fn test_http_status() {
        assert_eq!(HttpStatus::Ok.as_u16(), 200);
        assert_eq!(HttpStatus::Ok.reason_phrase(), "OK");
        assert!(HttpStatus::Ok.is_success());
        assert!(!HttpStatus::Ok.is_error());
        
        assert_eq!(HttpStatus::NotFound.as_u16(), 404);
        assert_eq!(HttpStatus::NotFound.reason_phrase(), "Not Found");
        assert!(!HttpStatus::NotFound.is_success());
        assert!(HttpStatus::NotFound.is_client_error());
        assert!(HttpStatus::NotFound.is_error());
    }

    #[test]
    fn test_request_creation() {
        let request = Request::new(HttpMethod::Get, "/test")
            .with_header("Content-Type", "application/json")
            .with_query("param", "value");
        
        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(request.path, "/test");
        assert_eq!(request.headers.get("Content-Type"), Some(&"application/json".to_string()));
        assert_eq!(request.query.get("param"), Some(&"value".to_string()));
    }

    #[test]
    fn test_response_creation() {
        let response = Response::json(HttpStatus::Ok, &serde_json::json!({"message": "test"})).unwrap();
        
        assert_eq!(response.status, HttpStatus::Ok);
        assert_eq!(response.headers.get("Content-Type"), Some(&"application/json".to_string()));
        assert!(response.body.is_some());
    }

    #[test]
    fn test_web_service_config() {
        let config = WebServiceConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert!(!config.https);
        assert!(config.enable_cors);
    }

    #[tokio::test]
    async fn test_web_service_creation() {
        let service = WebService::new();
        assert_eq!(service.config.host, "127.0.0.1");
        assert_eq!(service.config.port, 8080);
        assert!(service.logger.is_none());
        assert!(!service.is_running());
    }
}
