//! Security utilities and protection mechanisms for RustF framework
//!
//! This module provides essential security functions to protect against common
//! web application vulnerabilities including path traversal, XSS, CSRF, and more.
//!
//! ## Features
//! - Path traversal protection for static file serving
//! - HTML escaping and XSS prevention
//! - Input validation and sanitization
//! - Secure file handling utilities
//! - Security headers management

use crate::error::{Error, Result};
use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub mod csrf;
pub mod error_handling;
pub mod headers;
pub mod static_files;
pub mod validation;

// Re-export commonly used types for convenience
pub use csrf::{CsrfConfig, CsrfMiddleware};

/// Security configuration for the framework
#[derive(Clone, Debug)]
pub struct SecurityConfig {
    /// Enable path traversal protection
    pub enable_path_protection: bool,
    /// Enable HTML escaping by default
    pub enable_html_escaping: bool,
    /// Enable CSRF protection
    pub enable_csrf_protection: bool,
    /// Maximum allowed path depth for static files
    pub max_path_depth: usize,
    /// Allowed file extensions for static serving
    pub allowed_extensions: HashSet<String>,
    /// Blocked file extensions (takes precedence over allowed)
    pub blocked_extensions: HashSet<String>,
    /// Enable security headers
    pub enable_security_headers: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        let mut allowed_extensions = HashSet::new();
        allowed_extensions.extend(
            [
                "html", "css", "js", "json", "txt", "md", "png", "jpg", "jpeg", "gif", "svg",
                "ico", "woff", "woff2", "ttf", "eot", "pdf", "zip",
            ]
            .iter()
            .map(|s| s.to_string()),
        );

        let mut blocked_extensions = HashSet::new();
        blocked_extensions.extend(
            [
                "exe", "bat", "cmd", "com", "scr", "pif", "vbs", "js", "jar", "sh", "ps1", "php",
                "asp", "aspx", "jsp", "pl", "py", "rb",
            ]
            .iter()
            .map(|s| s.to_string()),
        );

        Self {
            enable_path_protection: true,
            enable_html_escaping: true,
            enable_csrf_protection: true,
            max_path_depth: 10,
            allowed_extensions,
            blocked_extensions,
            enable_security_headers: true,
        }
    }
}

/// Secure path validation and canonicalization
pub struct PathValidator {
    base_path: PathBuf,
    max_depth: usize,
    allowed_extensions: HashSet<String>,
    blocked_extensions: HashSet<String>,
}

impl PathValidator {
    /// Create a new path validator with the given base directory
    pub fn new(base_path: impl AsRef<Path>, config: &SecurityConfig) -> Result<Self> {
        let base_path = base_path
            .as_ref()
            .canonicalize()
            .map_err(|e| Error::template(format!("Invalid base path: {}", e)))?;

        Ok(Self {
            base_path,
            max_depth: config.max_path_depth,
            allowed_extensions: config.allowed_extensions.clone(),
            blocked_extensions: config.blocked_extensions.clone(),
        })
    }

    /// Validate and resolve a requested path securely
    ///
    /// This function prevents path traversal attacks by:
    /// 1. Canonicalizing the requested path
    /// 2. Ensuring it stays within the base directory
    /// 3. Checking file extension against allow/block lists
    /// 4. Validating path depth
    pub fn validate_path(&self, requested_path: &str) -> Result<PathBuf> {
        // Remove leading slash and normalize
        let requested_path = requested_path.trim_start_matches('/');

        // Check for obvious path traversal attempts
        if requested_path.contains("..") {
            return Err(Error::template(
                "Path traversal attempt detected".to_string(),
            ));
        }

        // Build the full path
        let full_path = self.base_path.join(requested_path);

        // Canonicalize to resolve any remaining path manipulation
        let canonical_path = match full_path.canonicalize() {
            Ok(path) => path,
            Err(_) => {
                // File doesn't exist or path is invalid
                return Err(Error::template(
                    "File not found or invalid path".to_string(),
                ));
            }
        };

        // Ensure the canonical path is still within our base directory
        if !canonical_path.starts_with(&self.base_path) {
            return Err(Error::template(
                "Path traversal attempt blocked".to_string(),
            ));
        }

        // Check path depth
        let relative_path = canonical_path
            .strip_prefix(&self.base_path)
            .map_err(|_| Error::template("Path validation error".to_string()))?;

        let depth = relative_path.components().count();
        if depth > self.max_depth {
            return Err(Error::template(format!(
                "Path depth {} exceeds maximum {}",
                depth, self.max_depth
            )));
        }

        // Validate file extension
        if let Some(extension) = canonical_path.extension().and_then(|s| s.to_str()) {
            let ext = extension.to_lowercase();

            // Check blocked extensions first (takes precedence)
            if self.blocked_extensions.contains(&ext) {
                return Err(Error::template(format!("File type '{}' is blocked", ext)));
            }

            // Check allowed extensions if not empty
            if !self.allowed_extensions.is_empty() && !self.allowed_extensions.contains(&ext) {
                return Err(Error::template(format!(
                    "File type '{}' is not allowed",
                    ext
                )));
            }
        }

        Ok(canonical_path)
    }

    /// Simple check if a path would be safe (without file system access)
    pub fn is_safe_path(&self, requested_path: &str) -> bool {
        // Check for obvious path traversal attempts
        if requested_path.contains("..") {
            return false;
        }

        // Check if path would be within base directory
        let requested_path = requested_path.trim_start_matches('/');
        let full_path = self.base_path.join(requested_path);

        // Basic path validation without file system access
        !requested_path.is_empty()
            && !requested_path.contains("../")
            && !requested_path.contains("..\\")
            && !requested_path.starts_with('/')
            && full_path.starts_with(&self.base_path)
    }

    /// Check if a file is safe to serve based on content analysis
    pub fn is_safe_file(&self, path: &Path) -> Result<bool> {
        // Check if file exists and is a regular file
        let metadata = std::fs::metadata(path)
            .map_err(|e| Error::template(format!("Cannot read file metadata: {}", e)))?;

        if !metadata.is_file() {
            return Ok(false);
        }

        // Check file size (prevent serving extremely large files)
        const MAX_STATIC_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB
        if metadata.len() > MAX_STATIC_FILE_SIZE {
            return Err(Error::template(
                "File too large to serve safely".to_string(),
            ));
        }

        // Additional content-based checks could be added here
        // For example: MIME type validation, virus scanning, etc.

        Ok(true)
    }
}

/// HTML escaping utility to prevent XSS attacks
pub struct HtmlEscaper;

impl HtmlEscaper {
    /// Escape HTML special characters to prevent XSS
    pub fn escape(input: &str) -> String {
        input
            .replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace("\"", "&quot;")
            .replace("'", "&#x27;")
            .replace("/", "&#x2F;")
    }

    /// Escape HTML attributes
    pub fn escape_attribute(input: &str) -> String {
        input
            .replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace("\"", "&quot;")
            .replace("'", "&#x27;")
            .replace("=", "&#x3D;")
            .replace("`", "&#x60;")
    }

    /// Escape for JavaScript context
    pub fn escape_js(input: &str) -> String {
        input
            .replace("\\", "\\\\")
            .replace("\"", "\\\"")
            .replace("'", "\\'")
            .replace("\n", "\\n")
            .replace("\r", "\\r")
            .replace("\t", "\\t")
            .replace("\u{2028}", "\\u2028")
            .replace("\u{2029}", "\\u2029")
            .replace("<", "\\u003C")
            .replace(">", "\\u003E")
    }

    /// Escape for CSS context
    pub fn escape_css(input: &str) -> String {
        let mut result = String::with_capacity(input.len() * 2);
        for c in input.chars() {
            match c {
                '"' => result.push_str("\\22 "),
                '\'' => result.push_str("\\27 "),
                '\\' => result.push_str("\\5C "),
                '\n' => result.push_str("\\A "),
                '\r' => result.push_str("\\D "),
                '\t' => result.push_str("\\9 "),
                c if c.is_control() || c as u32 > 0x7F => {
                    result.push_str(&format!("\\{:X} ", c as u32));
                }
                c => result.push(c),
            }
        }
        result
    }
}

/// Input validation utilities
pub struct InputValidator;

impl InputValidator {
    /// Validate email format
    pub fn is_valid_email(email: &str) -> bool {
        lazy_static::lazy_static! {
            static ref EMAIL_REGEX: Regex = Regex::new(
                r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"
            ).unwrap();
        }

        email.len() <= 254 && EMAIL_REGEX.is_match(email)
    }

    /// Validate URL format
    pub fn is_valid_url(url: &str) -> bool {
        url::Url::parse(url).is_ok()
    }

    /// Sanitize filename for safe storage
    pub fn sanitize_filename(filename: &str) -> String {
        let mut result = String::with_capacity(filename.len());

        for c in filename.chars() {
            match c {
                // Allow alphanumeric, dots, hyphens, underscores
                c if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' => {
                    result.push(c);
                }
                // Replace spaces with underscores
                ' ' => result.push('_'),
                // Skip other characters
                _ => {}
            }
        }

        // Ensure filename isn't empty and doesn't start/end with dots
        let result = result.trim_matches('.');
        if result.is_empty() {
            "file".to_string()
        } else {
            result.to_string()
        }
    }

    /// Validate that input contains only safe characters
    pub fn is_safe_input(input: &str, allow_html: bool) -> bool {
        if !allow_html {
            // Check for HTML/script injection
            let dangerous_patterns = [
                "<script",
                "</script>",
                "javascript:",
                "onclick=",
                "onerror=",
                "onload=",
                "eval(",
                "expression(",
                "vbscript:",
                "data:",
            ];

            let input_lower = input.to_lowercase();
            for pattern in &dangerous_patterns {
                if input_lower.contains(pattern) {
                    return false;
                }
            }
        }

        // Check for SQL injection patterns
        let sql_patterns = [
            "union select",
            "drop table",
            "delete from",
            "insert into",
            "update set",
            "exec ",
            "execute ",
            "sp_",
            "xp_",
        ];

        let input_lower = input.to_lowercase();
        for pattern in &sql_patterns {
            if input_lower.contains(pattern) {
                return false;
            }
        }

        true
    }

    /// Validate and sanitize user input
    pub fn sanitize_input(input: &str, max_length: usize, allow_html: bool) -> Result<String> {
        if input.len() > max_length {
            return Err(Error::template(format!(
                "Input too long: {} > {}",
                input.len(),
                max_length
            )));
        }

        if !Self::is_safe_input(input, allow_html) {
            return Err(Error::template(
                "Input contains potentially dangerous content".to_string(),
            ));
        }

        let sanitized = if allow_html {
            input.to_string()
        } else {
            HtmlEscaper::escape(input)
        };

        Ok(sanitized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_path_traversal_protection() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create a safe file
        let safe_file = base_path.join("safe.txt");
        fs::write(&safe_file, "safe content").unwrap();

        let config = SecurityConfig::default();
        let validator = PathValidator::new(base_path, &config).unwrap();

        // Test safe path
        assert!(validator.validate_path("safe.txt").is_ok());

        // Test path traversal attempts
        assert!(validator.validate_path("../../../etc/passwd").is_err());
        assert!(validator
            .validate_path("..\\..\\windows\\system32")
            .is_err());
        assert!(validator.validate_path("./../../secret").is_err());
        assert!(validator
            .validate_path("safe.txt/../../../etc/passwd")
            .is_err());
    }

    #[test]
    fn test_html_escaping() {
        let dangerous_html = "<script>alert('xss')</script>";
        let escaped = HtmlEscaper::escape(dangerous_html);
        assert_eq!(
            escaped,
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;&#x2F;script&gt;"
        );

        let js_payload = "'; alert('xss'); //";
        let js_escaped = HtmlEscaper::escape_js(js_payload);
        assert_eq!(js_escaped, "\\'; alert(\\'xss\\'); //");
    }

    #[test]
    fn test_input_validation() {
        // Test email validation
        assert!(InputValidator::is_valid_email("user@example.com"));
        assert!(!InputValidator::is_valid_email("invalid-email"));
        assert!(!InputValidator::is_valid_email("user@"));

        // Test filename sanitization
        assert_eq!(
            InputValidator::sanitize_filename("file name.txt"),
            "file_name.txt"
        );
        assert_eq!(
            InputValidator::sanitize_filename("../../../etc/passwd"),
            "etcpasswd"
        );
        assert_eq!(InputValidator::sanitize_filename("con.txt"), "con.txt");

        // Test dangerous input detection
        assert!(!InputValidator::is_safe_input(
            "<script>alert('xss')</script>",
            false
        ));
        assert!(!InputValidator::is_safe_input(
            "'; DROP TABLE users; --",
            false
        ));
        assert!(InputValidator::is_safe_input("Hello, World!", false));
    }

    #[test]
    fn test_security_config() {
        let config = SecurityConfig::default();
        assert!(config.enable_path_protection);
        assert!(config.enable_html_escaping);
        assert!(config.blocked_extensions.contains("exe"));
        assert!(config.allowed_extensions.contains("html"));
    }
}
