//! Error logging system for RustF framework
//!
//! Provides structured error logging with configurable levels, output targets,
//! and context information for debugging production issues.

use crate::config::AppConfig;
use crate::error::{Error, Result};
use crate::http::Request;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;

/// Log levels for error logging
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
    Critical = 4,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Critical => "CRITICAL",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "DEBUG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARN" | "WARNING" => LogLevel::Warn,
            "ERROR" => LogLevel::Error,
            "CRITICAL" | "FATAL" => LogLevel::Critical,
            _ => LogLevel::Info,
        }
    }
}

/// Output targets for logging
#[derive(Debug, Clone)]
pub enum LogOutput {
    Console,
    File(String),
    Both(String),
    None,
}

/// Configuration for error logging
#[derive(Debug, Clone)]
pub struct LogConfig {
    pub level: LogLevel,
    pub output: LogOutput,
    pub include_stack_trace: bool,
    pub include_request_context: bool,
    pub max_file_size: Option<u64>,
    pub max_files: Option<u32>,
}

impl Default for LogConfig {
    fn default() -> Self {
        // Default configuration based on build type
        let development_mode = cfg!(debug_assertions)
            || std::env::var("RUSTF_ENV").unwrap_or_default() == "development";

        Self {
            level: if development_mode {
                LogLevel::Debug
            } else {
                LogLevel::Info
            },
            output: LogOutput::Console,
            include_stack_trace: development_mode,
            include_request_context: true,
            max_file_size: Some(10 * 1024 * 1024), // 10MB
            max_files: Some(5),
        }
    }
}

/// Structured log entry
#[derive(Debug, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub error_type: Option<String>,
    pub request_id: Option<String>,
    pub request_context: Option<RequestContext>,
    pub stack_trace: Option<String>,
    pub additional_data: HashMap<String, serde_json::Value>,
}

/// Request context information for logging
#[derive(Debug, Serialize)]
pub struct RequestContext {
    pub method: String,
    pub uri: String,
    pub headers: HashMap<String, String>,
    pub client_ip: String,
    pub user_agent: Option<String>,
}

impl RequestContext {
    pub fn from_request(request: &Request) -> Self {
        Self {
            method: request.method.clone(),
            uri: sanitize_uri(&request.uri),
            headers: sanitize_headers(&request.headers),
            client_ip: request.client_ip(),
            user_agent: request.user_agent().map(|s| s.to_string()),
        }
    }
}

/// Error logger with configurable output and formatting
pub struct ErrorLogger {
    config: LogConfig,
    _app_config: Arc<AppConfig>,
}

impl ErrorLogger {
    /// Create a new error logger
    pub fn new(config: LogConfig, app_config: Arc<AppConfig>) -> Self {
        Self {
            config,
            _app_config: app_config,
        }
    }

    /// Create logger from app configuration
    pub fn from_app_config(app_config: Arc<AppConfig>) -> Self {
        let log_config = LogConfig::from_app_config(&app_config);
        Self::new(log_config, app_config)
    }

    /// Log an error with optional request context
    pub fn log_error(
        &self,
        level: LogLevel,
        error: &Error,
        request: Option<&Request>,
        request_id: Option<&str>,
        additional_data: Option<HashMap<String, serde_json::Value>>,
    ) {
        if level < self.config.level {
            return; // Skip logging if below configured level
        }

        let entry = self.create_log_entry(level, error, request, request_id, additional_data);
        self.write_log_entry(&entry);
    }

    /// Log a simple message
    pub fn log_message(
        &self,
        level: LogLevel,
        message: &str,
        request_id: Option<&str>,
        additional_data: Option<HashMap<String, serde_json::Value>>,
    ) {
        if level < self.config.level {
            return;
        }

        let entry = LogEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            level: level.as_str().to_string(),
            message: message.to_string(),
            error_type: None,
            request_id: request_id.map(|s| s.to_string()),
            request_context: None,
            stack_trace: None,
            additional_data: additional_data.unwrap_or_default(),
        };

        self.write_log_entry(&entry);
    }

    /// Create a structured log entry
    fn create_log_entry(
        &self,
        level: LogLevel,
        error: &Error,
        request: Option<&Request>,
        request_id: Option<&str>,
        additional_data: Option<HashMap<String, serde_json::Value>>,
    ) -> LogEntry {
        let error_message = if level >= LogLevel::Error && !self.config.include_stack_trace {
            // Production-safe error message
            self.sanitize_error_message(error)
        } else {
            error.to_string()
        };

        LogEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            level: level.as_str().to_string(),
            message: error_message,
            error_type: Some(self.classify_error_type(error)),
            request_id: request_id.map(|s| s.to_string()),
            request_context: if self.config.include_request_context {
                request.map(RequestContext::from_request)
            } else {
                None
            },
            stack_trace: if self.config.include_stack_trace {
                Some(format!("{:?}", error))
            } else {
                None
            },
            additional_data: additional_data.unwrap_or_default(),
        }
    }

    /// Write log entry to configured output
    fn write_log_entry(&self, entry: &LogEntry) {
        match &self.config.output {
            LogOutput::Console => self.write_to_console(entry),
            LogOutput::File(path) => {
                if let Err(e) = self.write_to_file(entry, path) {
                    eprintln!("Failed to write to log file {}: {}", path, e);
                    self.write_to_console(entry); // Fallback to console
                }
            }
            LogOutput::Both(path) => {
                self.write_to_console(entry);
                if let Err(e) = self.write_to_file(entry, path) {
                    eprintln!("Failed to write to log file {}: {}", path, e);
                }
            }
            LogOutput::None => {} // Silent logging
        }
    }

    /// Write log entry to console
    fn write_to_console(&self, entry: &LogEntry) {
        // Structured console output
        let console_format = format!(
            "[{}] {} - {} {}{}{}",
            entry.timestamp,
            entry.level,
            entry.message,
            entry
                .request_id
                .as_ref()
                .map(|id| format!("(req: {}) ", id))
                .unwrap_or_default(),
            if let Some(ctx) = &entry.request_context {
                format!("({} {}) ", ctx.method, ctx.uri)
            } else {
                String::new()
            },
            if let Some(error_type) = &entry.error_type {
                format!("[{}]", error_type)
            } else {
                String::new()
            }
        );

        // Use log crate for proper level handling
        match entry.level.as_str() {
            "DEBUG" => log::debug!("{}", console_format),
            "INFO" => log::info!("{}", console_format),
            "WARN" => log::warn!("{}", console_format),
            "ERROR" => log::error!("{}", console_format),
            "CRITICAL" => log::error!("[CRITICAL] {}", console_format),
            _ => log::info!("{}", console_format),
        }
    }

    /// Write log entry to file
    fn write_to_file(&self, entry: &LogEntry, file_path: &str) -> Result<()> {
        // Check file size and rotate if necessary
        if let Some(max_size) = self.config.max_file_size {
            if let Ok(metadata) = std::fs::metadata(file_path) {
                if metadata.len() > max_size {
                    self.rotate_log_file(file_path)?;
                }
            }
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)
            .map_err(|e| Error::internal(format!("Failed to open log file: {}", e)))?;

        // Write JSON formatted log entry
        let json_line = serde_json::to_string(entry)
            .map_err(|e| Error::internal(format!("Failed to serialize log entry: {}", e)))?;

        writeln!(file, "{}", json_line)
            .map_err(|e| Error::internal(format!("Failed to write to log file: {}", e)))?;

        Ok(())
    }

    /// Rotate log files when they exceed max size
    fn rotate_log_file(&self, file_path: &str) -> Result<()> {
        let max_files = self.config.max_files.unwrap_or(5);

        // Rotate existing files
        for i in (1..max_files).rev() {
            let old_file = format!("{}.{}", file_path, i);
            let new_file = format!("{}.{}", file_path, i + 1);

            if std::fs::metadata(&old_file).is_ok() {
                std::fs::rename(&old_file, &new_file)
                    .map_err(|e| Error::internal(format!("Failed to rotate log file: {}", e)))?;
            }
        }

        // Move current file to .1
        if std::fs::metadata(file_path).is_ok() {
            let rotated_file = format!("{}.1", file_path);
            std::fs::rename(file_path, &rotated_file).map_err(|e| {
                Error::internal(format!("Failed to rotate current log file: {}", e))
            })?;
        }

        Ok(())
    }

    /// Sanitize error message for production
    fn sanitize_error_message(&self, error: &Error) -> String {
        let error_str = error.to_string();

        // Remove sensitive information patterns
        let patterns_to_remove = [
            r"password=\w+",
            r"token=[\w\-\.]+",
            r"key=[\w\-\.]+",
            r"secret=[\w\-\.]+",
            r"api_key=[\w\-\.]+",
        ];

        let mut sanitized = error_str.clone();
        for pattern in &patterns_to_remove {
            if let Ok(re) = regex::Regex::new(pattern) {
                sanitized = re.replace_all(&sanitized, "$1[REDACTED]").to_string();
            }
        }

        // Generic error messages for production
        match error {
            Error::Template(_) => "Template processing error".to_string(),
            Error::Json(_) => "Data processing error".to_string(),
            Error::Session(_) => "Session error".to_string(),
            Error::Validation(_) => "Validation error".to_string(),
            Error::Io(_) => "File system error".to_string(),
            _ => "Internal server error".to_string(),
        }
    }

    /// Classify error type for logging
    fn classify_error_type(&self, error: &Error) -> String {
        match error {
            Error::Http(_) => "HTTP".to_string(),
            Error::Json(_) => "JSON".to_string(),
            Error::Template(_) => "Template".to_string(),
            Error::RouteNotFound(_) => "Routing".to_string(),
            Error::ModelNotFound(_) => "Model".to_string(),
            Error::Session(_) => "Session".to_string(),
            Error::Validation(_) => "Validation".to_string(),
            Error::InvalidInput(_) => "Input".to_string(),
            Error::Io(_) => "IO".to_string(),
            Error::Internal(_) => "Internal".to_string(),
            Error::DatabaseConnection(_) => "DatabaseConnection".to_string(),
            Error::DatabaseQuery(_) => "DatabaseQuery".to_string(),
            Error::DatabaseTransaction(_) => "DatabaseTransaction".to_string(),
            Error::DatabaseMigration(_) => "DatabaseMigration".to_string(),
            Error::DatabasePool(_) => "DatabasePool".to_string(),
            Error::Network(_) => "Network".to_string(),
            Error::Authentication(_) => "Authentication".to_string(),
            Error::Authorization(_) => "Authorization".to_string(),
            Error::RateLimit(_) => "RateLimit".to_string(),
            Error::ExternalService { .. } => "ExternalService".to_string(),
            Error::Timeout(_) => "Timeout".to_string(),
            Error::WithContext { .. } => "Context".to_string(),
            Error::Redis(_) => "Redis".to_string(),
            Error::RedisPool(_) => "RedisPool".to_string(),
        }
    }
}

impl LogConfig {
    /// Create log configuration from app configuration
    pub fn from_app_config(_app_config: &AppConfig) -> Self {
        let mut config = LogConfig::default();

        // Read logging configuration from environment variables
        if let Ok(level_str) = std::env::var("RUSTF_LOG_LEVEL") {
            config.level = LogLevel::from_str(&level_str);
        }

        if let Ok(output_str) = std::env::var("RUSTF_LOG_OUTPUT") {
            config.output = match output_str.as_str() {
                "console" => LogOutput::Console,
                "none" => LogOutput::None,
                path if path.starts_with("file:") => {
                    LogOutput::File(path.strip_prefix("file:").unwrap().to_string())
                }
                path if path.starts_with("both:") => {
                    LogOutput::Both(path.strip_prefix("both:").unwrap().to_string())
                }
                _ => LogOutput::Console,
            };
        }

        if let Ok(stack_trace_str) = std::env::var("RUSTF_LOG_STACK_TRACE") {
            config.include_stack_trace = stack_trace_str == "true" || stack_trace_str == "1";
        }

        if let Ok(context_str) = std::env::var("RUSTF_LOG_REQUEST_CONTEXT") {
            config.include_request_context = context_str == "true" || context_str == "1";
        }

        config
    }
}

/// Sanitize URI for logging (remove sensitive query parameters)
fn sanitize_uri(uri: &str) -> String {
    if let Some((path, query)) = uri.split_once('?') {
        let sanitized_query = query
            .split('&')
            .map(|param| {
                if let Some((key, _)) = param.split_once('=') {
                    if ["password", "token", "key", "secret", "api_key"]
                        .contains(&key.to_lowercase().as_str())
                    {
                        format!("{}=[REDACTED]", key)
                    } else {
                        param.to_string()
                    }
                } else {
                    param.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("&");

        format!("{}?{}", path, sanitized_query)
    } else {
        uri.to_string()
    }
}

/// Sanitize headers for logging (remove sensitive headers)
fn sanitize_headers(headers: &HashMap<String, String>) -> HashMap<String, String> {
    let sensitive_headers = ["authorization", "cookie", "x-api-key", "x-auth-token"];

    headers
        .iter()
        .map(|(key, value)| {
            if sensitive_headers.contains(&key.to_lowercase().as_str()) {
                (key.clone(), "[REDACTED]".to_string())
            } else {
                (key.clone(), value.clone())
            }
        })
        .collect()
}

/// Global error logger instance
static mut GLOBAL_LOGGER: Option<ErrorLogger> = None;
static LOGGER_INIT: std::sync::Once = std::sync::Once::new();

/// Initialize global error logger
pub fn init_global_logger(config: LogConfig, app_config: Arc<AppConfig>) {
    LOGGER_INIT.call_once(|| unsafe {
        GLOBAL_LOGGER = Some(ErrorLogger::new(config, app_config));
    });
}

/// Get reference to global error logger
#[allow(static_mut_refs)]
pub fn global_logger() -> Option<&'static ErrorLogger> {
    // Safety: GLOBAL_LOGGER is only mutated once during initialization
    // and then only read afterwards
    unsafe { GLOBAL_LOGGER.as_ref() }
}

/// Convenience functions for global logging
pub fn log_error(error: &Error, request: Option<&Request>, request_id: Option<&str>) {
    if let Some(logger) = global_logger() {
        logger.log_error(LogLevel::Error, error, request, request_id, None);
    }
}

pub fn log_critical(error: &Error, request: Option<&Request>, request_id: Option<&str>) {
    if let Some(logger) = global_logger() {
        logger.log_error(LogLevel::Critical, error, request, request_id, None);
    }
}

pub fn log_warning(message: &str, request_id: Option<&str>) {
    if let Some(logger) = global_logger() {
        logger.log_message(LogLevel::Warn, message, request_id, None);
    }
}

pub fn log_info(message: &str, request_id: Option<&str>) {
    if let Some(logger) = global_logger() {
        logger.log_message(LogLevel::Info, message, request_id, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use std::collections::HashMap;

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("DEBUG"), LogLevel::Debug);
        assert_eq!(LogLevel::from_str("error"), LogLevel::Error);
        assert_eq!(LogLevel::from_str("CRITICAL"), LogLevel::Critical);
        assert_eq!(LogLevel::from_str("unknown"), LogLevel::Info);
    }

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Error > LogLevel::Warn);
        assert!(LogLevel::Critical > LogLevel::Error);
    }

    #[test]
    fn test_sanitize_uri() {
        assert_eq!(sanitize_uri("/path?normal=value"), "/path?normal=value");
        assert_eq!(
            sanitize_uri("/path?password=secret123"),
            "/path?password=[REDACTED]"
        );
        assert_eq!(
            sanitize_uri("/path?token=abc123&normal=value"),
            "/path?token=[REDACTED]&normal=value"
        );
    }

    #[test]
    fn test_sanitize_headers() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Authorization".to_string(), "Bearer token123".to_string());
        headers.insert("Cookie".to_string(), "session=abc123".to_string());

        let sanitized = sanitize_headers(&headers);

        assert_eq!(
            sanitized.get("Content-Type"),
            Some(&"application/json".to_string())
        );
        assert_eq!(
            sanitized.get("Authorization"),
            Some(&"[REDACTED]".to_string())
        );
        assert_eq!(sanitized.get("Cookie"), Some(&"[REDACTED]".to_string()));
    }

    #[test]
    fn test_error_classification() {
        let app_config = Arc::new(AppConfig::default());
        let logger = ErrorLogger::new(LogConfig::default(), app_config);

        let template_error = Error::template("Template not found");
        assert_eq!(logger.classify_error_type(&template_error), "Template");

        let validation_error = Error::validation("Invalid input");
        assert_eq!(logger.classify_error_type(&validation_error), "Validation");

        let json_error =
            Error::Json(serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err());
        assert_eq!(logger.classify_error_type(&json_error), "JSON");
    }

    #[test]
    fn test_log_config_from_env() {
        std::env::set_var("RUSTF_LOG_LEVEL", "ERROR");
        std::env::set_var("RUSTF_LOG_STACK_TRACE", "false");

        let app_config = AppConfig::default();
        let config = LogConfig::from_app_config(&app_config);

        assert_eq!(config.level, LogLevel::Error);
        assert_eq!(config.include_stack_trace, false);

        // Clean up
        std::env::remove_var("RUSTF_LOG_LEVEL");
        std::env::remove_var("RUSTF_LOG_STACK_TRACE");
    }

    #[tokio::test]
    async fn test_log_entry_creation() {
        let app_config = Arc::new(AppConfig::default());
        let logger = ErrorLogger::new(LogConfig::default(), app_config);

        let error = Error::template("Test error");
        let entry = logger.create_log_entry(LogLevel::Error, &error, None, Some("req-123"), None);

        assert_eq!(entry.level, "ERROR");
        assert!(
            entry.message.contains("Test error")
                || entry.message.contains("Template processing error")
        );
        assert_eq!(entry.request_id, Some("req-123".to_string()));
        assert_eq!(entry.error_type, Some("Template".to_string()));
    }
}
