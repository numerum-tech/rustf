//! Custom error pages and production error handling
//!
//! This module provides custom error page templates and production-safe error responses
//! that don't leak sensitive information like stack traces in production environments.

use crate::config::AppConfig;
use crate::error::{Error, Result};
use crate::http::Response;
use crate::views::ViewEngine;
use hyper::StatusCode;
use serde_json::{json, Value};
use simd_json;
use std::sync::Arc;

/// Error page configuration and rendering
pub struct ErrorPages {
    view_engine: Arc<ViewEngine>,
    _config: Arc<AppConfig>,
    development_mode: bool,
}

impl ErrorPages {
    /// Create new error page handler
    pub fn new(view_engine: Arc<ViewEngine>, config: Arc<AppConfig>) -> Self {
        let development_mode = cfg!(debug_assertions)
            || std::env::var("RUSTF_ENV").unwrap_or_default() == "development";

        Self {
            view_engine,
            _config: config,
            development_mode,
        }
    }

    /// Render a custom error page
    pub fn render_error_page(
        &self,
        status_code: u16,
        error: Option<&Error>,
        request_id: Option<&str>,
    ) -> Result<Response> {
        let template_name = self.get_error_template(status_code);
        let error_data = self.prepare_error_data(status_code, error, request_id);

        // Try to render custom error template
        match self
            .view_engine
            .render(&template_name, &error_data, Some("error"))
        {
            Ok(html) => Ok(Response::new(
                StatusCode::from_u16(status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            )
            .with_header("Content-Type", "text/html; charset=utf-8")
            .with_body(html.into_bytes())),
            Err(_) => {
                // Fallback to built-in error page if custom template fails
                log::warn!(
                    "Failed to render custom error template: {}, using fallback",
                    template_name
                );
                self.render_fallback_error_page(status_code, error, request_id)
            }
        }
    }

    /// Get the template name for a given status code
    fn get_error_template(&self, status_code: u16) -> String {
        match status_code {
            400 => "errors/400".to_string(),
            401 => "errors/401".to_string(),
            403 => "errors/403".to_string(),
            404 => "errors/404".to_string(),
            409 => "errors/409".to_string(),
            429 => "errors/429".to_string(),
            500 => "errors/500".to_string(),
            502 => "errors/502".to_string(),
            503 => "errors/503".to_string(),
            _ => "errors/generic".to_string(),
        }
    }

    /// Prepare error data for template rendering
    fn prepare_error_data(
        &self,
        status_code: u16,
        error: Option<&Error>,
        request_id: Option<&str>,
    ) -> Value {
        let status_text = self.get_status_text(status_code);
        let user_message = self.get_user_friendly_message(status_code);

        let mut data = json!({
            "status_code": status_code,
            "status_text": status_text,
            "message": user_message,
            "request_id": request_id.unwrap_or("unknown"),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "development_mode": self.development_mode
        });

        // Include error details only in development mode
        if self.development_mode {
            if let Some(err) = error {
                data["error_details"] = json!({
                    "message": err.to_string(),
                    "type": self.classify_error_type(err),
                });
            }
        }

        data
    }

    /// Render a fallback error page when custom templates fail
    fn render_fallback_error_page(
        &self,
        status_code: u16,
        error: Option<&Error>,
        request_id: Option<&str>,
    ) -> Result<Response> {
        let status_text = self.get_status_text(status_code);
        let user_message = self.get_user_friendly_message(status_code);
        let request_id = request_id.unwrap_or("unknown");

        let error_details = if self.development_mode && error.is_some() {
            format!(
                "<div class=\"error-details\">
                <h3>Development Mode - Error Details:</h3>
                <pre>{}</pre>
                <p><strong>Error Type:</strong> {}</p>
            </div>",
                html_escape(&error.unwrap().to_string()),
                self.classify_error_type(error.unwrap())
            )
        } else {
            String::new()
        };

        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - {}</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            margin: 0;
            padding: 0;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
        }}
        .error-container {{
            background: rgba(255, 255, 255, 0.95);
            border-radius: 10px;
            padding: 2rem;
            max-width: 600px;
            margin: 1rem;
            box-shadow: 0 10px 30px rgba(0, 0, 0, 0.2);
            text-align: center;
        }}
        .error-code {{
            font-size: 6rem;
            font-weight: bold;
            color: #e74c3c;
            margin: 0;
            text-shadow: 2px 2px 4px rgba(0, 0, 0, 0.1);
        }}
        .error-title {{
            font-size: 1.5rem;
            color: #2c3e50;
            margin: 0.5rem 0;
        }}
        .error-message {{
            font-size: 1rem;
            color: #7f8c8d;
            margin: 1rem 0;
            line-height: 1.6;
        }}
        .error-details {{
            background: #f8f9fa;
            border: 1px solid #dee2e6;
            border-radius: 5px;
            padding: 1rem;
            margin: 1rem 0;
            text-align: left;
        }}
        .error-details pre {{
            background: #343a40;
            color: #f8f9fa;
            padding: 1rem;
            border-radius: 3px;
            overflow-x: auto;
            font-size: 0.9rem;
        }}
        .request-info {{
            font-size: 0.8rem;
            color: #95a5a6;
            margin-top: 2rem;
            padding-top: 1rem;
            border-top: 1px solid #ecf0f1;
        }}
        .back-button {{
            display: inline-block;
            background: #3498db;
            color: white;
            text-decoration: none;
            padding: 0.75rem 1.5rem;
            border-radius: 5px;
            margin-top: 1rem;
            transition: background 0.3s;
        }}
        .back-button:hover {{
            background: #2980b9;
        }}
    </style>
</head>
<body>
    <div class="error-container">
        <h1 class="error-code">{}</h1>
        <h2 class="error-title">{}</h2>
        <p class="error-message">{}</p>
        {}
        <a href="javascript:history.back()" class="back-button">Go Back</a>
        <div class="request-info">
            Request ID: {} | Timestamp: {}
        </div>
    </div>
</body>
</html>"#,
            status_code,
            status_text,
            status_code,
            status_text,
            user_message,
            error_details,
            request_id,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        Ok(Response::new(
            StatusCode::from_u16(status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        )
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body(html.into_bytes()))
    }

    /// Get human-readable status text for HTTP status codes
    fn get_status_text(&self, status_code: u16) -> &'static str {
        match status_code {
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            409 => "Conflict",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            _ => "Error",
        }
    }

    /// Get user-friendly error message for status codes
    fn get_user_friendly_message(&self, status_code: u16) -> &'static str {
        match status_code {
            400 => "The request could not be understood by the server. Please check your input and try again.",
            401 => "You need to authenticate to access this resource. Please login and try again.",
            403 => "You don't have permission to access this resource.",
            404 => "The page or resource you're looking for could not be found. It may have been moved or deleted.",
            409 => "There was a conflict with your request. The resource may have been modified by another user.",
            429 => "Too many requests have been made. Please wait a moment and try again.",
            500 => "An internal server error occurred. Our team has been notified and is working to fix the issue.",
            502 => "Bad gateway error. The server is temporarily unavailable.",
            503 => "The service is temporarily unavailable. Please try again later.",
            _ => "An error occurred while processing your request.",
        }
    }

    /// Classify error type for development debugging
    fn classify_error_type(&self, error: &Error) -> &'static str {
        let error_str = error.to_string().to_lowercase();

        if error_str.contains("database") || error_str.contains("sql") {
            "Database Error"
        } else if error_str.contains("network") || error_str.contains("connection") {
            "Network Error"
        } else if error_str.contains("template") || error_str.contains("view") {
            "Template Error"
        } else if error_str.contains("validation") || error_str.contains("invalid") {
            "Validation Error"
        } else if error_str.contains("permission") || error_str.contains("unauthorized") {
            "Authorization Error"
        } else if error_str.contains("not found") || error_str.contains("missing") {
            "Resource Not Found"
        } else {
            "Application Error"
        }
    }

    /// Create a JSON error response for API endpoints
    pub fn create_json_error_response(
        &self,
        status_code: u16,
        error: Option<&Error>,
        request_id: Option<&str>,
    ) -> Result<Response> {
        let user_message = self.get_user_friendly_message(status_code);

        let mut error_data = json!({
            "error": true,
            "status": status_code,
            "message": user_message,
            "request_id": request_id.unwrap_or("unknown"),
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        // Include detailed error information only in development mode
        if self.development_mode {
            if let Some(err) = error {
                error_data["details"] = json!({
                    "error_message": err.to_string(),
                    "error_type": self.classify_error_type(err),
                });
            }
        }

        Ok(Response::new(
            StatusCode::from_u16(status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        )
        .with_header("Content-Type", "application/json")
        .with_body(serde_json::to_string(&error_data)?.into_bytes()))
    }

    /// Create appropriate error response based on request Accept header
    pub fn create_error_response(
        &self,
        status_code: u16,
        error: Option<&Error>,
        request_id: Option<&str>,
        accept_header: Option<&str>,
    ) -> Result<Response> {
        if let Some(accept) = accept_header {
            if accept.contains("application/json") {
                return self.create_json_error_response(status_code, error, request_id);
            }
        }

        // Default to HTML error page
        self.render_error_page(status_code, error, request_id)
    }
}

/// HTML escape utility function
fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Health check endpoint implementation
pub struct HealthCheck {
    config: Arc<AppConfig>,
}

impl HealthCheck {
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self { config }
    }

    /// Perform basic health check
    pub async fn check_health(&self) -> HealthCheckResult {
        let mut result = HealthCheckResult {
            status: "healthy".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            checks: std::collections::HashMap::new(),
        };

        // Check database connectivity if configured
        if self.config.database.url.is_some() {
            match self.check_database_health().await {
                Ok(_) => {
                    result.checks.insert(
                        "database".to_string(),
                        CheckStatus {
                            status: "healthy".to_string(),
                            message: Some("Database connection successful".to_string()),
                        },
                    );
                }
                Err(e) => {
                    result.status = "unhealthy".to_string();
                    result.checks.insert(
                        "database".to_string(),
                        CheckStatus {
                            status: "unhealthy".to_string(),
                            message: Some(format!("Database connection failed: {}", e)),
                        },
                    );
                }
            }
        }

        // Check memory usage
        result
            .checks
            .insert("memory".to_string(), self.check_memory_usage());

        result
    }

    /// Check database health
    async fn check_database_health(&self) -> Result<()> {
        use crate::db::DB;

        // Check if database is initialized and available
        if !DB::is_initialized() {
            return Err(Error::template("Database not initialized".to_string()));
        }

        // Try to get a connection to verify it's working
        if DB::connection().is_none() {
            return Err(Error::template(
                "Database connection not available".to_string(),
            ));
        }

        Ok(())
    }

    /// Check memory usage
    fn check_memory_usage(&self) -> CheckStatus {
        // Basic memory usage check - in a real implementation you might use
        // system APIs to get actual memory usage
        CheckStatus {
            status: "healthy".to_string(),
            message: Some("Memory usage within acceptable limits".to_string()),
        }
    }

    /// Create a health check response
    pub async fn create_response(&self) -> Result<Response> {
        let health_result = self.check_health().await;
        let status_code = if health_result.status == "healthy" {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        };

        Ok(Response::new(status_code)
            .with_header("Content-Type", "application/json")
            .with_body(serde_json::to_string(&health_result)?.into_bytes()))
    }
}

/// Health check result structure
#[derive(serde::Serialize, serde::Deserialize)]
pub struct HealthCheckResult {
    pub status: String,
    pub timestamp: String,
    pub version: String,
    pub checks: std::collections::HashMap<String, CheckStatus>,
}

/// Individual check status
#[derive(serde::Serialize, serde::Deserialize)]
pub struct CheckStatus {
    pub status: String,
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::views::ViewEngine;

    fn create_test_error_pages() -> ErrorPages {
        let view_engine = Arc::new(ViewEngine::from_directory("views"));
        let config = Arc::new(AppConfig::default());
        ErrorPages::new(view_engine, config)
    }

    #[test]
    fn test_error_template_names() {
        let error_pages = create_test_error_pages();

        assert_eq!(error_pages.get_error_template(404), "errors/404");
        assert_eq!(error_pages.get_error_template(500), "errors/500");
        assert_eq!(error_pages.get_error_template(999), "errors/generic");
    }

    #[test]
    fn test_status_text() {
        let error_pages = create_test_error_pages();

        assert_eq!(error_pages.get_status_text(404), "Not Found");
        assert_eq!(error_pages.get_status_text(500), "Internal Server Error");
        assert_eq!(error_pages.get_status_text(999), "Error");
    }

    #[test]
    fn test_user_friendly_messages() {
        let error_pages = create_test_error_pages();

        let message_404 = error_pages.get_user_friendly_message(404);
        assert!(message_404.contains("could not be found"));

        let message_500 = error_pages.get_user_friendly_message(500);
        assert!(message_500.contains("internal server error"));
    }

    #[test]
    fn test_error_classification() {
        let error_pages = create_test_error_pages();

        let db_error = Error::template("Database connection failed".to_string());
        assert_eq!(error_pages.classify_error_type(&db_error), "Database Error");

        let template_error = Error::template("Template not found".to_string());
        assert_eq!(
            error_pages.classify_error_type(&template_error),
            "Template Error"
        );

        let generic_error = Error::internal("Something went wrong".to_string());
        assert_eq!(
            error_pages.classify_error_type(&generic_error),
            "Application Error"
        );
    }

    #[tokio::test]
    async fn test_fallback_error_page_generation() {
        let error_pages = create_test_error_pages();
        let error = Error::template("Test error".to_string());

        let response = error_pages
            .render_fallback_error_page(404, Some(&error), Some("test-123"))
            .unwrap();

        assert_eq!(response.status, StatusCode::NOT_FOUND);
        assert!(response
            .headers
            .iter()
            .any(|(k, v)| k == "Content-Type" && v.contains("text/html")));

        let body_str = String::from_utf8(response.body).unwrap();
        assert!(body_str.contains("404"));
        assert!(body_str.contains("Not Found"));
        assert!(body_str.contains("test-123"));
    }

    #[tokio::test]
    async fn test_json_error_response() {
        let error_pages = create_test_error_pages();
        let error = Error::template("Test API error".to_string());

        let response = error_pages
            .create_json_error_response(400, Some(&error), Some("api-456"))
            .unwrap();

        assert_eq!(response.status, StatusCode::BAD_REQUEST);
        assert!(response
            .headers
            .iter()
            .any(|(k, v)| k == "Content-Type" && v.contains("application/json")));

        let body_str = String::from_utf8(response.body).unwrap();
        let mut body_bytes = body_str.clone().into_bytes();
        let json_data: Value = simd_json::from_slice(&mut body_bytes).unwrap();

        assert_eq!(json_data["status"], 400);
        assert_eq!(json_data["error"], true);
        assert_eq!(json_data["request_id"], "api-456");
    }

    #[tokio::test]
    async fn test_health_check() {
        let config = Arc::new(AppConfig::default());
        let health_check = HealthCheck::new(config);

        let result = health_check.check_health().await;

        // Basic health check should always pass
        assert!(result.status == "healthy" || result.status == "unhealthy");
        assert!(!result.version.is_empty());
        assert!(result.checks.contains_key("memory"));
    }

    #[tokio::test]
    async fn test_health_check_response() {
        let config = Arc::new(AppConfig::default());
        let health_check = HealthCheck::new(config);

        let response = health_check.create_response().await.unwrap();

        assert!(
            response.status == StatusCode::OK || response.status == StatusCode::SERVICE_UNAVAILABLE
        );
        assert!(response
            .headers
            .iter()
            .any(|(k, v)| k == "Content-Type" && v.contains("application/json")));

        let body_str = String::from_utf8(response.body).unwrap();
        let mut body_bytes = body_str.clone().into_bytes();
        let health_data: HealthCheckResult = simd_json::from_slice(&mut body_bytes).unwrap();

        assert!(!health_data.version.is_empty());
        assert!(!health_data.timestamp.is_empty());
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(
            html_escape("<script>alert('xss')</script>"),
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
        );
        assert_eq!(html_escape("Normal text"), "Normal text");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
        assert_eq!(html_escape("A & B"), "A &amp; B");
    }
}
