//! Security validation middleware for RustF
//!
//! This middleware provides security-focused validation to detect and block
//! common attack patterns like SQL injection, XSS, and path traversal.

use crate::context::Context;
use crate::error::Result;
use crate::middleware::{InboundAction, InboundMiddleware};
use async_trait::async_trait;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::json;

/// Common SQL injection patterns
static SQL_INJECTION_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)(\bunion\b.*\bselect\b|\bselect\b.*\bfrom\b|\binsert\b.*\binto\b)")
            .expect("ValidationMiddleware: Invalid SQL injection pattern regex"),
        Regex::new(r"(?i)(\bdrop\b.*\btable\b|\bdelete\b.*\bfrom\b|\bupdate\b.*\bset\b)")
            .expect("ValidationMiddleware: Invalid SQL injection pattern regex"),
        Regex::new(r"(?i)(exec\s*\(|execute\s*\(|xp_cmdshell)")
            .expect("ValidationMiddleware: Invalid SQL injection pattern regex"),
        Regex::new(r"(?i)(script\s*>|javascript:|onerror\s*=|onload\s*=)")
            .expect("ValidationMiddleware: Invalid SQL injection pattern regex"),
        Regex::new(r"(?i)(--|\#|\/\*|\*\/|@@|@)")
            .expect("ValidationMiddleware: Invalid SQL injection pattern regex"),
    ]
});

/// XSS attack patterns
static XSS_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)<script[^>]*>.*?</script>")
            .expect("ValidationMiddleware: Invalid XSS pattern regex"),
        Regex::new(r"(?i)javascript:").expect("ValidationMiddleware: Invalid XSS pattern regex"),
        Regex::new(r"(?i)on\w+\s*=").expect("ValidationMiddleware: Invalid XSS pattern regex"),
        Regex::new(r"(?i)<iframe[^>]*>").expect("ValidationMiddleware: Invalid XSS pattern regex"),
        Regex::new(r"(?i)<object[^>]*>").expect("ValidationMiddleware: Invalid XSS pattern regex"),
        Regex::new(r"(?i)<embed[^>]*>").expect("ValidationMiddleware: Invalid XSS pattern regex"),
        Regex::new(r"(?i)<applet[^>]*>").expect("ValidationMiddleware: Invalid XSS pattern regex"),
    ]
});

/// Path traversal patterns
static PATH_TRAVERSAL_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"\.\.[\\/]")
            .expect("ValidationMiddleware: Invalid path traversal pattern regex"),
        Regex::new(r"\.\.%2[fF]")
            .expect("ValidationMiddleware: Invalid path traversal pattern regex"),
        Regex::new(r"%2e%2e").expect("ValidationMiddleware: Invalid path traversal pattern regex"),
        Regex::new(r"(?i)(etc\/passwd|windows\/system)")
            .expect("ValidationMiddleware: Invalid path traversal pattern regex"),
    ]
});

/// Validation middleware configuration
#[derive(Clone)]
pub struct ValidationConfig {
    /// Check for SQL injection patterns
    pub check_sql_injection: bool,
    /// Check for XSS patterns
    pub check_xss: bool,
    /// Check for path traversal
    pub check_path_traversal: bool,
    /// Maximum allowed parameter length
    pub max_param_length: usize,
    /// Paths to exclude from validation
    pub excluded_paths: Vec<String>,
    /// Log blocked attempts
    pub log_violations: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            check_sql_injection: true,
            check_xss: true,
            check_path_traversal: true,
            max_param_length: 10000,
            excluded_paths: vec![
                "/api/".to_string(), // API endpoints might have different validation
            ],
            log_violations: true,
        }
    }
}

/// Validation middleware for detecting common attack patterns
#[derive(Clone)]
pub struct ValidationMiddleware {
    config: ValidationConfig,
}

impl ValidationMiddleware {
    /// Create new validation middleware with default config
    pub fn new() -> Self {
        Self {
            config: ValidationConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ValidationConfig) -> Self {
        Self { config }
    }

    /// Create lenient validation (fewer checks)
    pub fn lenient() -> Self {
        Self {
            config: ValidationConfig {
                check_sql_injection: true,
                check_xss: false,
                check_path_traversal: true,
                max_param_length: 50000,
                excluded_paths: vec![],
                log_violations: true,
            },
        }
    }

    /// Create strict validation (all checks)
    pub fn strict() -> Self {
        Self {
            config: ValidationConfig {
                check_sql_injection: true,
                check_xss: true,
                check_path_traversal: true,
                max_param_length: 5000,
                excluded_paths: vec![],
                log_violations: true,
            },
        }
    }

    /// Check if path is excluded from validation
    fn is_excluded(&self, path: &str) -> bool {
        self.config
            .excluded_paths
            .iter()
            .any(|excluded| path.starts_with(excluded))
    }

    /// Validate a single parameter value
    fn validate_param(&self, name: &str, value: &str) -> Option<String> {
        // Check length
        if value.len() > self.config.max_param_length {
            return Some(format!("Parameter '{}' exceeds maximum length", name));
        }

        // Check SQL injection
        if self.config.check_sql_injection {
            for pattern in SQL_INJECTION_PATTERNS.iter() {
                if pattern.is_match(value) {
                    return Some(format!(
                        "SQL injection pattern detected in parameter '{}'",
                        name
                    ));
                }
            }
        }

        // Check XSS
        if self.config.check_xss {
            for pattern in XSS_PATTERNS.iter() {
                if pattern.is_match(value) {
                    return Some(format!("XSS pattern detected in parameter '{}'", name));
                }
            }
        }

        // Check path traversal
        if self.config.check_path_traversal {
            for pattern in PATH_TRAVERSAL_PATTERNS.iter() {
                if pattern.is_match(value) {
                    return Some(format!(
                        "Path traversal pattern detected in parameter '{}'",
                        name
                    ));
                }
            }
        }

        None
    }

    /// Extract and validate all parameters from request
    fn validate_request(&self, ctx: &Context) -> Option<String> {
        // Validate URL path
        if let Some(error) = self.validate_param("path", &ctx.req.uri) {
            return Some(error);
        }

        // Validate query parameters
        if let Some(query) = ctx.req.uri.split('?').nth(1) {
            for pair in query.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    let decoded_value = urlencoding::decode(value).unwrap_or_default();
                    if let Some(error) = self.validate_param(key, &decoded_value) {
                        return Some(error);
                    }
                }
            }
        }

        // Validate headers (selective)
        for (name, value) in &ctx.req.headers {
            // Only validate certain headers that might contain user input
            if name == "referer" || name == "user-agent" || name.starts_with("x-") {
                if let Some(error) = self.validate_param(name, value) {
                    return Some(error);
                }
            }
        }

        None
    }
}

impl Default for ValidationMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InboundMiddleware for ValidationMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        // Skip validation for excluded paths
        if self.is_excluded(&ctx.req.uri) {
            return Ok(InboundAction::Continue);
        }

        // Validate the request
        if let Some(error) = self.validate_request(ctx) {
            if self.config.log_violations {
                log::warn!("Input validation failed: {} from {}", error, ctx.ip());
            }

            // Use context helper to set error response
            ctx.status(hyper::StatusCode::BAD_REQUEST);
            ctx.json(json!({
                "error": "invalid_input",
                "message": "Request contains invalid or potentially malicious input",
                "details": error
            }))?;

            return Ok(InboundAction::Stop);
        }

        Ok(InboundAction::Continue)
    }

    fn name(&self) -> &'static str {
        "input_validation"
    }

    fn priority(&self) -> i32 {
        -800 // Run early, after rate limiting
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_injection_detection() {
        let validator = ValidationMiddleware::new();

        // Should detect SQL injection
        assert!(validator
            .validate_param("q", "'; DROP TABLE users; --")
            .is_some());
        assert!(validator
            .validate_param("q", "1' UNION SELECT * FROM users")
            .is_some());

        // Should allow normal input
        assert!(validator
            .validate_param("q", "normal search query")
            .is_none());
    }

    #[test]
    fn test_xss_detection() {
        let validator = ValidationMiddleware::new();

        // Should detect XSS
        assert!(validator
            .validate_param("input", "<script>alert('xss')</script>")
            .is_some());
        assert!(validator
            .validate_param("input", "javascript:alert(1)")
            .is_some());

        // Should allow normal HTML entities
        assert!(validator
            .validate_param("input", "5 < 10 && 10 > 5")
            .is_none());
    }

    #[test]
    fn test_path_traversal_detection() {
        let validator = ValidationMiddleware::new();

        // Should detect path traversal
        assert!(validator
            .validate_param("file", "../../etc/passwd")
            .is_some());
        assert!(validator
            .validate_param("file", "..\\windows\\system32")
            .is_some());

        // Should allow normal paths
        assert!(validator
            .validate_param("file", "documents/file.txt")
            .is_none());
    }
}
