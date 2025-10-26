//! Input validation and sanitization utilities
//!
//! This module provides comprehensive input validation and sanitization functions
//! to prevent injection attacks, XSS, and other input-based vulnerabilities.

use crate::error::{Error, Result};
use regex::Regex;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// HTML escaping utility for XSS prevention
pub struct HtmlEscaper;

impl Default for HtmlEscaper {
    fn default() -> Self {
        Self::new()
    }
}

impl HtmlEscaper {
    pub fn new() -> Self {
        Self
    }

    /// Escape HTML characters to prevent XSS
    pub fn escape(&self, input: &str) -> String {
        input
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }
}

/// Security configuration for input handling
pub struct SecurityConfig {
    pub max_input_length: usize,
    pub allow_html: bool,
    pub sanitize_sql: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_input_length: 10000,
            allow_html: false,
            sanitize_sql: true,
        }
    }
}

impl SecurityConfig {
    /// Sanitize input according to security configuration
    pub fn sanitize_input(&self, input: &str, max_length: usize) -> Result<String> {
        // Limit input length
        let truncated = if input.len() > max_length {
            &input[..max_length]
        } else {
            input
        };

        let mut sanitized = truncated.to_string();

        // Remove dangerous HTML if not allowed
        if !self.allow_html {
            sanitized = HtmlEscaper::new().escape(&sanitized);
        }

        // Sanitize SQL injection patterns
        if self.sanitize_sql {
            let dangerous_patterns = [
                "DROP TABLE",
                "DELETE FROM",
                "INSERT INTO",
                "UPDATE SET",
                "SELECT * FROM",
                "UNION SELECT",
                "'; --",
                "\" OR \"1\"=\"1",
                "<script",
                "</script>",
                "javascript:",
                "onerror=",
                "onload=",
            ];

            for pattern in &dangerous_patterns {
                sanitized = sanitized.replace(pattern, "");
            }
        }

        Ok(sanitized)
    }
}

/// Validation rule for input fields
#[derive(Clone, Debug)]
pub struct ValidationRule {
    pub field_name: String,
    pub required: bool,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<Regex>,
    pub custom_validator: Option<fn(&str) -> bool>,
    pub sanitizer: Option<fn(&str) -> String>,
}

impl ValidationRule {
    /// Create a new validation rule
    pub fn new(field_name: &str) -> Self {
        Self {
            field_name: field_name.to_string(),
            required: false,
            min_length: None,
            max_length: None,
            pattern: None,
            custom_validator: None,
            sanitizer: None,
        }
    }

    /// Mark field as required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Set minimum length
    pub fn min_length(mut self, min: usize) -> Self {
        self.min_length = Some(min);
        self
    }

    /// Set maximum length
    pub fn max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    /// Set length range
    pub fn length_range(mut self, min: usize, max: usize) -> Self {
        self.min_length = Some(min);
        self.max_length = Some(max);
        self
    }

    /// Set validation pattern
    pub fn pattern(mut self, pattern: Regex) -> Self {
        self.pattern = Some(pattern);
        self
    }

    /// Set custom validator function
    pub fn custom_validator(mut self, validator: fn(&str) -> bool) -> Self {
        self.custom_validator = Some(validator);
        self
    }

    /// Set sanitizer function
    pub fn sanitizer(mut self, sanitizer: fn(&str) -> String) -> Self {
        self.sanitizer = Some(sanitizer);
        self
    }

    /// Email validation rule
    pub fn email() -> Self {
        lazy_static::lazy_static! {
            static ref EMAIL_REGEX: Regex = Regex::new(
                r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$"
            ).unwrap();
        }

        Self::new("email")
            .max_length(254)
            .pattern(EMAIL_REGEX.clone())
    }

    /// URL validation rule
    pub fn url() -> Self {
        Self::new("url")
            .max_length(2048)
            .custom_validator(|value| url::Url::parse(value).is_ok())
    }

    /// Username validation rule (alphanumeric + underscore/hyphen)
    pub fn username() -> Self {
        lazy_static::lazy_static! {
            static ref USERNAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
        }

        Self::new("username")
            .length_range(3, 32)
            .pattern(USERNAME_REGEX.clone())
    }

    /// Password validation rule
    pub fn password() -> Self {
        Self::new("password")
            .length_range(8, 128)
            .custom_validator(|value| {
                // Check for at least one uppercase, lowercase, digit, and special char
                let has_upper = value.chars().any(|c| c.is_uppercase());
                let has_lower = value.chars().any(|c| c.is_lowercase());
                let has_digit = value.chars().any(|c| c.is_numeric());
                let has_special = value
                    .chars()
                    .any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c));

                has_upper && has_lower && has_digit && has_special
            })
    }

    /// Phone number validation rule
    pub fn phone() -> Self {
        lazy_static::lazy_static! {
            static ref PHONE_REGEX: Regex = Regex::new(r"^\+?[1-9]\d{1,14}$").unwrap();
        }

        Self::new("phone")
            .length_range(10, 15)
            .pattern(PHONE_REGEX.clone())
            .sanitizer(|value| {
                value
                    .chars()
                    .filter(|c| c.is_numeric() || *c == '+')
                    .collect()
            })
    }

    /// Filename validation rule
    pub fn filename() -> Self {
        lazy_static::lazy_static! {
            static ref FILENAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9._-]+$").unwrap();
        }

        Self::new("filename")
            .length_range(1, 255)
            .pattern(FILENAME_REGEX.clone())
            .sanitizer(|value| {
                value
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || "._-".contains(c) {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect()
            })
    }
}

/// Validation error details
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub code: String,
}

impl ValidationError {
    pub fn new(field: &str, message: &str, code: &str) -> Self {
        Self {
            field: field.to_string(),
            message: message.to_string(),
            code: code.to_string(),
        }
    }
}

/// Input validator with multiple rules
pub struct InputValidator {
    rules: Vec<ValidationRule>,
}

impl Default for InputValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl InputValidator {
    /// Create a new input validator
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a validation rule
    pub fn add_rule(mut self, rule: ValidationRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Validate input data
    pub fn validate(&self, data: &HashMap<String, String>) -> Result<HashMap<String, String>> {
        let mut errors = Vec::new();
        let mut validated_data = HashMap::new();

        for rule in &self.rules {
            let value = data.get(&rule.field_name);

            // Check if required field is present
            if rule.required && (value.is_none() || value.unwrap().is_empty()) {
                errors.push(ValidationError::new(
                    &rule.field_name,
                    "This field is required",
                    "required",
                ));
                continue;
            }

            // Skip validation if field is not present and not required
            let value = match value {
                Some(v) if !v.is_empty() => v,
                _ => continue,
            };

            // Apply sanitizer first if present
            let sanitized_value = if let Some(sanitizer) = rule.sanitizer {
                sanitizer(value)
            } else {
                value.clone()
            };

            // Length validation
            if let Some(min_len) = rule.min_length {
                if sanitized_value.len() < min_len {
                    errors.push(ValidationError::new(
                        &rule.field_name,
                        &format!("Must be at least {} characters long", min_len),
                        "min_length",
                    ));
                    continue;
                }
            }

            if let Some(max_len) = rule.max_length {
                if sanitized_value.len() > max_len {
                    errors.push(ValidationError::new(
                        &rule.field_name,
                        &format!("Must be no more than {} characters long", max_len),
                        "max_length",
                    ));
                    continue;
                }
            }

            // Pattern validation
            if let Some(pattern) = &rule.pattern {
                if !pattern.is_match(&sanitized_value) {
                    errors.push(ValidationError::new(
                        &rule.field_name,
                        "Invalid format",
                        "pattern",
                    ));
                    continue;
                }
            }

            // Custom validation
            if let Some(validator) = rule.custom_validator {
                if !validator(&sanitized_value) {
                    errors.push(ValidationError::new(
                        &rule.field_name,
                        "Validation failed",
                        "custom",
                    ));
                    continue;
                }
            }

            validated_data.insert(rule.field_name.clone(), sanitized_value);
        }

        if !errors.is_empty() {
            let error_messages: Vec<String> = errors
                .iter()
                .map(|e| format!("{}: {}", e.field, e.message))
                .collect();
            return Err(Error::template(format!(
                "Validation failed: {}",
                error_messages.join(", ")
            )));
        }

        Ok(validated_data)
    }
}

/// CSRF token generator and validator
pub struct CsrfProtection {
    secret_key: String,
    token_lifetime: u64, // seconds
}

impl CsrfProtection {
    /// Create new CSRF protection with secret key
    pub fn new(secret_key: &str) -> Self {
        Self {
            secret_key: secret_key.to_string(),
            token_lifetime: 3600, // 1 hour default
        }
    }

    /// Generate a CSRF token
    pub fn generate_token(&self, session_id: &str) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Simple token generation - in production, use proper cryptographic functions
        let data = format!("{}:{}:{}", session_id, timestamp, self.secret_key);
        let hash = self.simple_hash(&data);

        format!("{}:{}", timestamp, hash)
    }

    /// Validate a CSRF token
    pub fn validate_token(&self, token: &str, session_id: &str) -> bool {
        let parts: Vec<&str> = token.split(':').collect();
        if parts.len() != 2 {
            return false;
        }

        let timestamp: u64 = match parts[0].parse() {
            Ok(t) => t,
            Err(_) => return false,
        };

        let provided_hash = parts[1];

        // Check if token is expired
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now - timestamp > self.token_lifetime {
            return false;
        }

        // Verify hash
        let data = format!("{}:{}:{}", session_id, timestamp, self.secret_key);
        let expected_hash = self.simple_hash(&data);

        provided_hash == expected_hash
    }

    /// Simple hash function (use proper crypto in production)
    fn simple_hash(&self, data: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// Rate limiting for input validation
pub struct RateLimiter {
    requests: HashMap<String, Vec<u64>>,
    max_requests: usize,
    window_seconds: u64,
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(max_requests: usize, window_seconds: u64) -> Self {
        Self {
            requests: HashMap::new(),
            max_requests,
            window_seconds,
        }
    }

    /// Check if request is allowed
    pub fn is_allowed(&mut self, identifier: &str) -> bool {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let window_start = now - self.window_seconds;

        // Get or create entry for this identifier
        let timestamps = self
            .requests
            .entry(identifier.to_string())
            .or_default();

        // Remove old timestamps
        timestamps.retain(|&timestamp| timestamp >= window_start);

        // Check if limit exceeded
        if timestamps.len() >= self.max_requests {
            return false;
        }

        // Add current timestamp
        timestamps.push(now);
        true
    }

    /// Clean up old entries
    pub fn cleanup(&mut self) {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let window_start = now - self.window_seconds * 2; // Keep extra buffer

        self.requests.retain(|_, timestamps| {
            timestamps.retain(|&timestamp| timestamp >= window_start);
            !timestamps.is_empty()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_rules() {
        let mut data = HashMap::new();
        data.insert("email".to_string(), "user@example.com".to_string());
        data.insert("username".to_string(), "testuser123".to_string());
        data.insert("password".to_string(), "SecureP@ss123".to_string());

        let validator = InputValidator::new()
            .add_rule(ValidationRule::email().required())
            .add_rule(ValidationRule::username().required())
            .add_rule(ValidationRule::password().required());

        let result = validator.validate(&data);
        assert!(result.is_ok());

        let validated = result.unwrap();
        assert_eq!(validated.get("email").unwrap(), "user@example.com");
        assert_eq!(validated.get("username").unwrap(), "testuser123");
    }

    #[test]
    fn test_validation_errors() {
        let mut data = HashMap::new();
        data.insert("email".to_string(), "invalid-email".to_string());
        data.insert("password".to_string(), "weak".to_string());

        let validator = InputValidator::new()
            .add_rule(ValidationRule::email().required())
            .add_rule(ValidationRule::password().required());

        let result = validator.validate(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_csrf_protection() {
        let csrf = CsrfProtection::new("secret_key");
        let session_id = "test_session";

        let token = csrf.generate_token(session_id);
        assert!(csrf.validate_token(&token, session_id));
        assert!(!csrf.validate_token(&token, "different_session"));
        assert!(!csrf.validate_token("invalid_token", session_id));
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(3, 60); // 3 requests per minute

        // First 3 requests should be allowed
        assert!(limiter.is_allowed("user1"));
        assert!(limiter.is_allowed("user1"));
        assert!(limiter.is_allowed("user1"));

        // Fourth request should be blocked
        assert!(!limiter.is_allowed("user1"));

        // Different user should be allowed
        assert!(limiter.is_allowed("user2"));
    }
}
