//! Secure error handling and sanitization for RustF framework
//!
//! This module provides secure error handling that prevents information leakage,
//! sanitizes error messages, and provides safe error responses for production use.

use super::HtmlEscaper;
use crate::error::Error;
use crate::http::Response;
use std::collections::HashMap;

/// Security-focused error handler that prevents information leakage
pub struct SecureErrorHandler {
    /// Whether to show detailed error messages (false in production)
    show_details: bool,
    /// Custom error messages for different error types
    custom_messages: HashMap<String, String>,
    /// Whether to log detailed errors server-side
    log_errors: bool,
    /// Whether to include request ID in error responses
    include_request_id: bool,
}

impl SecureErrorHandler {
    /// Create error handler for development (shows details)
    pub fn development() -> Self {
        Self {
            show_details: true,
            custom_messages: HashMap::new(),
            log_errors: true,
            include_request_id: true,
        }
    }

    /// Create error handler for production (hides details)
    pub fn production() -> Self {
        Self {
            show_details: false,
            custom_messages: Self::default_production_messages(),
            log_errors: true,
            include_request_id: true,
        }
    }

    /// Create error handler with custom configuration
    pub fn new(show_details: bool) -> Self {
        Self {
            show_details,
            custom_messages: if show_details {
                HashMap::new()
            } else {
                Self::default_production_messages()
            },
            log_errors: true,
            include_request_id: true,
        }
    }

    /// Set custom error message for a specific error type
    pub fn custom_message(mut self, error_type: &str, message: &str) -> Self {
        self.custom_messages
            .insert(error_type.to_string(), message.to_string());
        self
    }

    /// Enable or disable error logging
    pub fn log_errors(mut self, enable: bool) -> Self {
        self.log_errors = enable;
        self
    }

    /// Enable or disable request ID in responses
    pub fn include_request_id(mut self, enable: bool) -> Self {
        self.include_request_id = enable;
        self
    }

    /// Default production error messages that don't leak information
    fn default_production_messages() -> HashMap<String, String> {
        let mut messages = HashMap::new();
        messages.insert(
            "internal".to_string(),
            "An internal server error occurred".to_string(),
        );
        messages.insert(
            "not_found".to_string(),
            "The requested resource was not found".to_string(),
        );
        messages.insert(
            "unauthorized".to_string(),
            "Authentication required".to_string(),
        );
        messages.insert("forbidden".to_string(), "Access denied".to_string());
        messages.insert("bad_request".to_string(), "Invalid request".to_string());
        messages.insert(
            "validation".to_string(),
            "Request validation failed".to_string(),
        );
        messages.insert("rate_limit".to_string(), "Too many requests".to_string());
        messages.insert("timeout".to_string(), "Request timeout".to_string());
        messages
    }

    /// Sanitize error message to prevent information leakage
    pub fn sanitize_message(&self, message: &str) -> String {
        if !self.show_details {
            // In production, replace potentially sensitive information
            self.remove_sensitive_info(message)
        } else {
            // In development, escape HTML but keep details
            HtmlEscaper::escape(message)
        }
    }

    /// Remove sensitive information from error messages
    fn remove_sensitive_info(&self, message: &str) -> String {
        let mut sanitized = message.to_string();

        // Remove file paths
        let path_patterns = [
            r"/[a-zA-Z0-9_\-./]+\.(rs|toml|yaml|yml|json|env)",
            r"C:\\[a-zA-Z0-9_\-\\]+\.(rs|toml|yaml|yml|json|env)",
            r"/home/[a-zA-Z0-9_\-/]+",
            r"/Users/[a-zA-Z0-9_\-/]+",
        ];

        for pattern in &path_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                sanitized = re.replace_all(&sanitized, "[REDACTED_PATH]").to_string();
            }
        }

        // Remove IP addresses
        if let Ok(ip_re) = regex::Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b") {
            sanitized = ip_re.replace_all(&sanitized, "[REDACTED_IP]").to_string();
        }

        // Remove environment variables
        if let Ok(env_re) = regex::Regex::new(r"[A-Z_][A-Z0-9_]*=\S+") {
            sanitized = env_re.replace_all(&sanitized, "[REDACTED_ENV]").to_string();
        }

        // Remove stack traces
        if let Ok(stack_re) = regex::Regex::new(r"at [a-zA-Z0-9_:.<>]+\([^)]+\)") {
            sanitized = stack_re
                .replace_all(&sanitized, "[REDACTED_STACK]")
                .to_string();
        }

        // Escape any remaining HTML
        HtmlEscaper::escape(&sanitized)
    }

    /// Create a secure error response
    pub fn create_error_response(&self, error: &Error, request_id: Option<&str>) -> Response {
        let (status, error_type, _message) = self.categorize_error(error);

        // Log the error if enabled
        if self.log_errors {
            log::error!(
                "Request error [{}]: {} - {}",
                request_id.unwrap_or("unknown"),
                error_type,
                error
            );
        }

        // Get appropriate message
        let safe_message = if let Some(custom) = self.custom_messages.get(&error_type) {
            custom.clone()
        } else if self.show_details {
            self.sanitize_message(&error.to_string())
        } else {
            self.custom_messages
                .get("internal")
                .unwrap_or(&"An error occurred".to_string())
                .clone()
        };

        // Create error response
        let error_data = SecureErrorResponse {
            error: error_type,
            message: safe_message,
            request_id: if self.include_request_id {
                request_id.map(|s| s.to_string())
            } else {
                None
            },
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        // Create JSON response
        match serde_json::to_string(&error_data) {
            Ok(json) => Response::new(status)
                .with_header("Content-Type", "application/json")
                .with_header("X-Content-Type-Options", "nosniff")
                .with_header("Cache-Control", "no-cache, no-store, must-revalidate")
                .with_body(json.into_bytes()),
            Err(_) => Response::new(status)
                .with_header("Content-Type", "text/plain")
                .with_body(b"An error occurred".to_vec()),
        }
    }

    /// Categorize error and determine appropriate status code
    fn categorize_error(&self, error: &Error) -> (hyper::StatusCode, String, String) {
        let error_str = error.to_string().to_lowercase();

        if error_str.contains("not found") || error_str.contains("404") {
            (
                hyper::StatusCode::NOT_FOUND,
                "not_found".to_string(),
                error.to_string(),
            )
        } else if error_str.contains("unauthorized") || error_str.contains("authentication") {
            (
                hyper::StatusCode::UNAUTHORIZED,
                "unauthorized".to_string(),
                error.to_string(),
            )
        } else if error_str.contains("forbidden") || error_str.contains("access denied") {
            (
                hyper::StatusCode::FORBIDDEN,
                "forbidden".to_string(),
                error.to_string(),
            )
        } else if error_str.contains("bad request") || error_str.contains("invalid") {
            (
                hyper::StatusCode::BAD_REQUEST,
                "bad_request".to_string(),
                error.to_string(),
            )
        } else if error_str.contains("validation") {
            (
                hyper::StatusCode::BAD_REQUEST,
                "validation".to_string(),
                error.to_string(),
            )
        } else if error_str.contains("rate limit") || error_str.contains("too many") {
            (
                hyper::StatusCode::TOO_MANY_REQUESTS,
                "rate_limit".to_string(),
                error.to_string(),
            )
        } else if error_str.contains("timeout") {
            (
                hyper::StatusCode::REQUEST_TIMEOUT,
                "timeout".to_string(),
                error.to_string(),
            )
        } else {
            (
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                "internal".to_string(),
                error.to_string(),
            )
        }
    }

    /// Create a secure 404 response
    pub fn not_found_response(&self, request_id: Option<&str>) -> Response {
        let error_data = SecureErrorResponse {
            error: "not_found".to_string(),
            message: self
                .custom_messages
                .get("not_found")
                .unwrap_or(&"The requested resource was not found".to_string())
                .clone(),
            request_id: if self.include_request_id {
                request_id.map(|s| s.to_string())
            } else {
                None
            },
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        match serde_json::to_string(&error_data) {
            Ok(json) => Response::not_found()
                .with_header("Content-Type", "application/json")
                .with_header("X-Content-Type-Options", "nosniff")
                .with_header("Cache-Control", "no-cache, no-store, must-revalidate")
                .with_body(json.into_bytes()),
            Err(_) => Response::not_found(),
        }
    }
}

/// Secure error response structure
#[derive(serde::Serialize)]
struct SecureErrorResponse {
    error: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
    timestamp: u64,
}

/// Error sanitization utilities
pub struct ErrorSanitizer;

impl ErrorSanitizer {
    /// Sanitize database error messages
    pub fn sanitize_database_error(error: &str) -> String {
        let mut sanitized = error.to_string();

        // Remove SQL queries
        if let Ok(sql_re) =
            regex::Regex::new(r"(?i)(SELECT|INSERT|UPDATE|DELETE|CREATE|DROP|ALTER)\s+[^;]+")
        {
            sanitized = sql_re.replace_all(&sanitized, "[REDACTED_SQL]").to_string();
        }

        // Remove connection strings
        if let Ok(conn_re) =
            regex::Regex::new(r"(?i)(host|server|database|user|password|uid|pwd)=[^;\s]+")
        {
            sanitized = conn_re
                .replace_all(&sanitized, "[REDACTED_CONNECTION]")
                .to_string();
        }

        // Remove table/column names in common patterns
        if let Ok(table_re) = regex::Regex::new(r"(?i)table\s+`[^`]+`") {
            sanitized = table_re
                .replace_all(&sanitized, "table `[REDACTED]`")
                .to_string();
        }

        sanitized
    }

    /// Sanitize filesystem error messages
    pub fn sanitize_filesystem_error(error: &str) -> String {
        let mut sanitized = error.to_string();

        // Remove file paths
        let path_patterns = [
            r"(?i)/[a-zA-Z0-9_\-./]+",
            r"(?i)C:\\[a-zA-Z0-9_\-\\]+",
            r"(?i)[a-zA-Z]:[a-zA-Z0-9_\-\\]+",
        ];

        for pattern in &path_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                sanitized = re.replace_all(&sanitized, "[REDACTED_PATH]").to_string();
            }
        }

        sanitized
    }

    /// Sanitize network error messages
    pub fn sanitize_network_error(error: &str) -> String {
        let mut sanitized = error.to_string();

        // Remove IP addresses and ports
        if let Ok(ip_re) = regex::Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}(?::\d+)?\b") {
            sanitized = ip_re
                .replace_all(&sanitized, "[REDACTED_ENDPOINT]")
                .to_string();
        }

        // Remove URLs
        if let Ok(url_re) = regex::Regex::new(r"https?://[^\s]+") {
            sanitized = url_re.replace_all(&sanitized, "[REDACTED_URL]").to_string();
        }

        sanitized
    }

    /// Generic error sanitization
    pub fn sanitize_generic_error(error: &str) -> String {
        let mut sanitized = error.to_string();

        // Apply all sanitization methods
        sanitized = Self::sanitize_database_error(&sanitized);
        sanitized = Self::sanitize_filesystem_error(&sanitized);
        sanitized = Self::sanitize_network_error(&sanitized);

        // Remove any remaining sensitive patterns
        let sensitive_patterns = [
            r"(?i)password[=:]\s*\S+",
            r"(?i)token[=:]\s*\S+",
            r"(?i)key[=:]\s*\S+",
            r"(?i)secret[=:]\s*\S+",
            r"(?i)auth[=:]\s*\S+",
        ];

        for pattern in &sensitive_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                sanitized = re
                    .replace_all(&sanitized, "[REDACTED_CREDENTIAL]")
                    .to_string();
            }
        }

        sanitized
    }
}

/// Request ID generator for error tracking
pub struct RequestIdGenerator;

impl RequestIdGenerator {
    /// Generate a unique request ID
    pub fn generate() -> String {
        use rand::Rng;
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        let random: u32 = rand::thread_rng().gen();

        format!("req_{:x}_{:x}", timestamp, random)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_handler_development() {
        let handler = SecureErrorHandler::development();
        assert!(handler.show_details);
        assert!(handler.log_errors);
        assert!(handler.include_request_id);
    }

    #[test]
    fn test_error_handler_production() {
        let handler = SecureErrorHandler::production();
        assert!(!handler.show_details);
        assert!(handler.log_errors);
        assert!(!handler.custom_messages.is_empty());
    }

    #[test]
    fn test_error_sanitization() {
        let handler = SecureErrorHandler::production();

        // Test path removal
        let error_with_path = "Failed to read /home/user/.env file";
        let sanitized = handler.sanitize_message(error_with_path);
        assert!(sanitized.contains("[REDACTED_PATH]"));
        assert!(!sanitized.contains("/home/user"));

        // Test IP removal
        let error_with_ip = "Connection failed to 192.168.1.100:5432";
        let sanitized = handler.sanitize_message(error_with_ip);
        assert!(sanitized.contains("[REDACTED_IP]"));
        assert!(!sanitized.contains("192.168.1.100"));
    }

    #[test]
    fn test_database_error_sanitization() {
        let db_error = "SELECT * FROM users WHERE password='secret123'";
        let sanitized = ErrorSanitizer::sanitize_database_error(db_error);
        assert!(sanitized.contains("[REDACTED_SQL]"));
        assert!(!sanitized.contains("SELECT"));
        assert!(!sanitized.contains("secret123"));
    }

    #[test]
    fn test_filesystem_error_sanitization() {
        let fs_error = "Permission denied: /etc/passwd";
        let sanitized = ErrorSanitizer::sanitize_filesystem_error(fs_error);
        assert!(sanitized.contains("[REDACTED_PATH]"));
        assert!(!sanitized.contains("/etc/passwd"));
    }

    #[test]
    fn test_network_error_sanitization() {
        let net_error = "Failed to connect to https://api.secret.com/v1/users";
        let sanitized = ErrorSanitizer::sanitize_network_error(net_error);
        assert!(sanitized.contains("[REDACTED_URL]"));
        assert!(!sanitized.contains("api.secret.com"));
    }

    #[test]
    fn test_request_id_generation() {
        let id1 = RequestIdGenerator::generate();
        let id2 = RequestIdGenerator::generate();

        assert_ne!(id1, id2);
        assert!(id1.starts_with("req_"));
        assert!(id2.starts_with("req_"));
    }

    #[test]
    fn test_error_response_creation() {
        let handler = SecureErrorHandler::production();
        let error = Error::template("Test error".to_string());
        let response = handler.create_error_response(&error, Some("req_123"));

        assert_eq!(response.status, hyper::StatusCode::INTERNAL_SERVER_ERROR);

        // Check headers
        let has_content_type = response
            .headers
            .iter()
            .any(|(name, value)| name == "Content-Type" && value == "application/json");
        assert!(has_content_type);

        let has_security_header = response
            .headers
            .iter()
            .any(|(name, value)| name == "X-Content-Type-Options" && value == "nosniff");
        assert!(has_security_header);
    }
}
