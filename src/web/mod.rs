//! Web service module for Loquat framework

use crate::errors::{Result, WebError};
use crate::logging::traits::Logger;
use std::sync::Arc;

/// Main web service structure
pub struct WebService {
    config: WebServiceConfig,
    logger: Option<Arc<dyn Logger>>,
}

impl WebService {
    /// Create a new web service with default configuration
    pub fn new() -> Self {
        Self {
            config: WebServiceConfig::default(),
            logger: None,
        }
    }

    /// Create a new web service with configuration
    pub fn with_config(config: WebServiceConfig) -> Self {
        Self {
            config,
            logger: None,
        }
    }

    /// Set the logger for the web service
    pub fn with_logger(mut self, logger: Arc<dyn Logger>) -> Self {
        self.logger = Some(logger);
        self
    }

    /// Start the web service
    pub async fn start(&self) -> Result<()> {
        if let Some(logger) = &self.logger {
            logger.info(&format!("Starting web service on {}:{}", self.config.host, self.config.port));
        }

        // In a real implementation, this would start an actual web server
        // For now, we'll just log that we would start
        
        println!("Web service would start on {}:{}", self.config.host, self.config.port);
        
        Ok(())
    }

    /// Stop the web service
    pub async fn stop(&self) -> Result<()> {
        if let Some(logger) = &self.logger {
            logger.info("Stopping web service");
        }

        // In a real implementation, this would gracefully stop the web server
        println!("Web service would stop");
        
        Ok(())
    }
}

impl std::fmt::Debug for WebService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebService")
            .field("config", &self.config)
            .field("logger", &self.logger.as_ref().map(|_| "Logger present"))
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
#[derive(Debug, Clone, PartialEq)]
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
        match self {
            HttpStatus::Continue => 100,
            HttpStatus::SwitchingProtocols => 101,
            HttpStatus::Processing => 102,
            HttpStatus::EarlyHints => 103,
            
            HttpStatus::Ok => 200,
            HttpStatus::Created => 201,
            HttpStatus::Accepted => 202,
            HttpStatus::NonAuthoritativeInformation => 203,
            HttpStatus::NoContent => 204,
            HttpStatus::ResetContent => 205,
            HttpStatus::PartialContent => 206,
            
            HttpStatus::MultipleChoices => 300,
            HttpStatus::MovedPermanently => 301,
            HttpStatus::Found => 302,
            HttpStatus::SeeOther => 303,
            HttpStatus::NotModified => 304,
            HttpStatus::UseProxy => 305,
            HttpStatus::TemporaryRedirect => 307,
            HttpStatus::PermanentRedirect => 308,
            
            HttpStatus::BadRequest => 400,
            HttpStatus::Unauthorized => 401,
            HttpStatus::PaymentRequired => 402,
            HttpStatus::Forbidden => 403,
            HttpStatus::NotFound => 404,
            HttpStatus::MethodNotAllowed => 405,
            HttpStatus::NotAcceptable => 406,
            HttpStatus::ProxyAuthenticationRequired => 407,
            HttpStatus::RequestTimeout => 408,
            HttpStatus::Conflict => 409,
            HttpStatus::Gone => 410,
            HttpStatus::LengthRequired => 411,
            HttpStatus::PreconditionFailed => 412,
            HttpStatus::PayloadTooLarge => 413,
            HttpStatus::UriTooLong => 414,
            HttpStatus::UnsupportedMediaType => 415,
            HttpStatus::RangeNotSatisfiable => 416,
            HttpStatus::ExpectationFailed => 417,
            HttpStatus::ImATeapot => 418,
            HttpStatus::MisdirectedRequest => 421,
            HttpStatus::UnprocessableEntity => 422,
            HttpStatus::Locked => 423,
            HttpStatus::FailedDependency => 424,
            HttpStatus::TooEarly => 425,
            HttpStatus::UpgradeRequired => 426,
            HttpStatus::PreconditionRequired => 428,
            HttpStatus::TooManyRequests => 429,
            HttpStatus::RequestHeaderFieldsTooLarge => 431,
            HttpStatus::UnavailableForLegalReasons => 451,
            
            HttpStatus::InternalServerError => 500,
            HttpStatus::NotImplemented => 501,
            HttpStatus::BadGateway => 502,
            HttpStatus::ServiceUnavailable => 503,
            HttpStatus::GatewayTimeout => 504,
            HttpStatus::HttpVersionNotSupported => 505,
            HttpStatus::VariantAlsoNegotiates => 506,
            HttpStatus::InsufficientStorage => 507,
            HttpStatus::LoopDetected => 508,
            HttpStatus::NotExtended => 510,
            HttpStatus::NetworkAuthenticationRequired => 511,
        }
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
    }
}
