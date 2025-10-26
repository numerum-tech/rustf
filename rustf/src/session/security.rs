//! Secure session management for RustF framework
//!
//! This module provides security hardening for session management including:
//! - Cryptographically secure session ID generation
//! - Session fixation protection with ID regeneration
//! - CSRF token generation and validation
//! - Rate limiting for session operations
//! - Session hijacking detection
//! - Secure cookie configuration

use crate::error::{Error, Result};
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Secure session configuration with hardened defaults
#[derive(Clone, Debug)]
pub struct SecureSessionConfig {
    /// Session ID length in bytes (default: 32 for 256-bit security)
    pub session_id_length: usize,
    /// Enable session fixation protection
    pub enable_fixation_protection: bool,
    /// Enable CSRF protection
    pub enable_csrf_protection: bool,
    /// Session timeout in seconds (default: 30 minutes)
    pub session_timeout: u64,
    /// Absolute session timeout in seconds (default: 8 hours)
    pub absolute_timeout: u64,
    /// Maximum sessions per IP (rate limiting)
    pub max_sessions_per_ip: usize,
    /// Cookie security settings
    pub cookie_config: SecureCookieConfig,
    /// Enable session hijacking detection
    pub enable_hijacking_detection: bool,
}

/// Secure cookie configuration
#[derive(Clone, Debug)]
pub struct SecureCookieConfig {
    /// Cookie name for session ID
    pub name: String,
    /// Secure flag (HTTPS only)
    pub secure: bool,
    /// HttpOnly flag (prevent JavaScript access)
    pub http_only: bool,
    /// SameSite attribute
    pub same_site: SameSitePolicy,
    /// Domain restriction
    pub domain: Option<String>,
    /// Path restriction
    pub path: String,
}

/// SameSite cookie policy
#[derive(Clone, Debug)]
pub enum SameSitePolicy {
    Strict,
    Lax,
    None,
}

impl Default for SecureSessionConfig {
    fn default() -> Self {
        Self {
            session_id_length: 32, // 256-bit session ID
            enable_fixation_protection: true,
            enable_csrf_protection: true,
            session_timeout: 30 * 60,      // 30 minutes
            absolute_timeout: 8 * 60 * 60, // 8 hours
            max_sessions_per_ip: 10,
            cookie_config: SecureCookieConfig::default(),
            enable_hijacking_detection: true,
        }
    }
}

impl Default for SecureCookieConfig {
    fn default() -> Self {
        Self {
            name: "RUSTF_SESSION".to_string(),
            secure: true, // Default to secure in production
            http_only: true,
            same_site: SameSitePolicy::Lax,
            domain: None,
            path: "/".to_string(),
        }
    }
}

/// Secure session ID generator
pub struct SecureSessionIdGenerator {
    length: usize,
}

impl SecureSessionIdGenerator {
    /// Create a new secure session ID generator
    pub fn new(length: usize) -> Self {
        if length < 16 {
            panic!("Session ID length must be at least 16 bytes for security");
        }
        Self { length }
    }

    /// Generate a cryptographically secure session ID
    pub fn generate(&self) -> String {
        let mut rng = thread_rng();
        let mut bytes = vec![0u8; self.length];
        rng.fill(&mut bytes[..]);

        // Use URL-safe base64 encoding
        base64_encode_url_safe(&bytes)
    }

    /// Validate session ID format and security
    pub fn validate_session_id(&self, session_id: &str) -> bool {
        // Check length (base64 encoded should be longer than raw bytes)
        if session_id.len() < self.length {
            return false;
        }

        // Check for valid base64 URL-safe characters
        session_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    }
}

/// CSRF token generator and validator
pub struct CsrfProtection {
    token_length: usize,
}

impl CsrfProtection {
    /// Create new CSRF protection with specified token length
    pub fn new(token_length: usize) -> Self {
        Self { token_length }
    }

    /// Generate a new CSRF token
    pub fn generate_token(&self) -> String {
        let mut rng = thread_rng();
        let mut bytes = vec![0u8; self.token_length];
        rng.fill(&mut bytes[..]);
        base64_encode_url_safe(&bytes)
    }

    /// Validate CSRF token
    pub fn validate_token(&self, session_token: &str, request_token: &str) -> bool {
        // Constant-time comparison to prevent timing attacks
        if session_token.len() != request_token.len() {
            return false;
        }

        let mut result = 0u8;
        for (a, b) in session_token.bytes().zip(request_token.bytes()) {
            result |= a ^ b;
        }
        result == 0
    }
}

/// Session fixation protection
pub struct FixationProtection {
    id_generator: SecureSessionIdGenerator,
}

impl FixationProtection {
    /// Create new fixation protection
    pub fn new(session_id_length: usize) -> Self {
        Self {
            id_generator: SecureSessionIdGenerator::new(session_id_length),
        }
    }

    /// Regenerate session ID for fixation protection
    pub fn regenerate_id(&self, old_id: &str) -> String {
        log::info!(
            "Regenerating session ID for fixation protection: {}",
            &old_id[..8.min(old_id.len())]
        );
        self.id_generator.generate()
    }

    /// Check if session ID regeneration is needed
    pub fn should_regenerate(&self, session_age: u64, last_regeneration: u64) -> bool {
        // Regenerate every 4 hours or after authentication events
        let regeneration_interval = 4 * 60 * 60; // 4 hours
        session_age - last_regeneration > regeneration_interval
    }
}

/// Session hijacking detection
pub struct HijackingDetection {
    enabled: bool,
}

impl HijackingDetection {
    /// Create new hijacking detection
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Check for potential session hijacking
    pub fn detect_hijacking(
        &self,
        session_info: &SessionSecurityInfo,
        request_info: &RequestSecurityInfo,
    ) -> HijackingRisk {
        if !self.enabled {
            return HijackingRisk::None;
        }

        let mut risk_score = 0;

        // Check IP address changes
        if session_info.original_ip != request_info.client_ip {
            risk_score += 3;
        }

        // Check User-Agent changes
        if session_info.user_agent != request_info.user_agent {
            risk_score += 2;
        }

        // Check for suspicious geographic location changes (simplified)
        if self.is_suspicious_location_change(&session_info.original_ip, &request_info.client_ip) {
            risk_score += 4;
        }

        // Check session age
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let session_age = now - session_info.created_at;
        if session_age > 24 * 60 * 60 {
            // 24 hours
            risk_score += 1;
        }

        match risk_score {
            0..=2 => HijackingRisk::None,
            3..=5 => HijackingRisk::Low,
            6..=8 => HijackingRisk::Medium,
            _ => HijackingRisk::High,
        }
    }

    /// Simplified geographic location change detection
    fn is_suspicious_location_change(&self, original_ip: &str, current_ip: &str) -> bool {
        // In a real implementation, this would use GeoIP databases
        // For now, just check if IPs are completely different
        original_ip != current_ip
            && !self.is_private_ip(original_ip)
            && !self.is_private_ip(current_ip)
    }

    /// Check if IP is private/local
    fn is_private_ip(&self, ip: &str) -> bool {
        ip.starts_with("192.168.")
            || ip.starts_with("10.")
            || ip.starts_with("172.")
            || ip == "127.0.0.1"
            || ip == "::1"
    }
}

/// Session security information
#[derive(Clone, Debug)]
pub struct SessionSecurityInfo {
    pub original_ip: String,
    pub user_agent: String,
    pub created_at: u64,
    pub last_regeneration: u64,
    pub csrf_token: Option<String>,
    pub login_time: Option<u64>,
}

/// Request security information
#[derive(Clone, Debug)]
pub struct RequestSecurityInfo {
    pub client_ip: String,
    pub user_agent: String,
    pub csrf_token: Option<String>,
}

/// Hijacking risk levels
#[derive(Clone, Debug, PartialEq)]
pub enum HijackingRisk {
    None,
    Low,
    Medium,
    High,
}

/// Session rate limiter to prevent abuse
pub struct SessionRateLimiter {
    max_sessions_per_ip: usize,
    session_counts: HashMap<String, usize>,
    last_cleanup: u64,
}

impl SessionRateLimiter {
    /// Create new session rate limiter
    pub fn new(max_sessions_per_ip: usize) -> Self {
        Self {
            max_sessions_per_ip,
            session_counts: HashMap::new(),
            last_cleanup: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Check if IP can create new session
    pub fn can_create_session(&mut self, ip: &str) -> bool {
        self.cleanup_old_entries();

        let count = self.session_counts.get(ip).unwrap_or(&0);
        *count < self.max_sessions_per_ip
    }

    /// Record new session for IP
    pub fn record_session(&mut self, ip: &str) {
        let count = self.session_counts.entry(ip.to_string()).or_insert(0);
        *count += 1;
    }

    /// Remove session for IP
    pub fn remove_session(&mut self, ip: &str) {
        if let Some(count) = self.session_counts.get_mut(ip) {
            if *count > 0 {
                *count -= 1;
            }
            if *count == 0 {
                self.session_counts.remove(ip);
            }
        }
    }

    /// Cleanup old entries (simple time-based cleanup)
    fn cleanup_old_entries(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if now - self.last_cleanup > 300 {
            // Cleanup every 5 minutes
            // In a real implementation, this would track creation times
            self.session_counts.clear();
            self.last_cleanup = now;
        }
    }
}

/// Enhanced secure session manager
pub struct SecureSessionManager {
    config: SecureSessionConfig,
    id_generator: SecureSessionIdGenerator,
    csrf_protection: CsrfProtection,
    fixation_protection: FixationProtection,
    hijacking_detection: HijackingDetection,
    rate_limiter: SessionRateLimiter,
}

impl SecureSessionManager {
    /// Create new secure session manager
    pub fn new(config: SecureSessionConfig) -> Self {
        let id_generator = SecureSessionIdGenerator::new(config.session_id_length);
        let csrf_protection = CsrfProtection::new(32); // 256-bit CSRF tokens
        let fixation_protection = FixationProtection::new(config.session_id_length);
        let hijacking_detection = HijackingDetection::new(config.enable_hijacking_detection);
        let rate_limiter = SessionRateLimiter::new(config.max_sessions_per_ip);

        Self {
            config,
            id_generator,
            csrf_protection,
            fixation_protection,
            hijacking_detection,
            rate_limiter,
        }
    }

    /// Generate secure session ID
    pub fn generate_session_id(&self) -> String {
        self.id_generator.generate()
    }

    /// Validate session ID format
    pub fn validate_session_id(&self, session_id: &str) -> bool {
        self.id_generator.validate_session_id(session_id)
    }

    /// Generate CSRF token
    pub fn generate_csrf_token(&self) -> String {
        self.csrf_protection.generate_token()
    }

    /// Validate CSRF token
    pub fn validate_csrf_token(&self, session_token: &str, request_token: &str) -> bool {
        self.csrf_protection
            .validate_token(session_token, request_token)
    }

    /// Check for session hijacking
    pub fn detect_hijacking(
        &self,
        session_info: &SessionSecurityInfo,
        request_info: &RequestSecurityInfo,
    ) -> HijackingRisk {
        self.hijacking_detection
            .detect_hijacking(session_info, request_info)
    }

    /// Check if session needs ID regeneration
    pub fn should_regenerate_id(&self, session_age: u64, last_regeneration: u64) -> bool {
        if !self.config.enable_fixation_protection {
            return false;
        }
        self.fixation_protection
            .should_regenerate(session_age, last_regeneration)
    }

    /// Regenerate session ID
    pub fn regenerate_session_id(&self, old_id: &str) -> String {
        self.fixation_protection.regenerate_id(old_id)
    }

    /// Check rate limits for new session
    pub fn can_create_session(&mut self, ip: &str) -> bool {
        self.rate_limiter.can_create_session(ip)
    }

    /// Record new session creation
    pub fn record_session_creation(&mut self, ip: &str) {
        self.rate_limiter.record_session(ip);
    }

    /// Remove session from rate limiter
    pub fn remove_session(&mut self, ip: &str) {
        self.rate_limiter.remove_session(ip);
    }

    /// Create secure cookie value
    pub fn create_cookie_header(&self, session_id: &str) -> String {
        let mut cookie = format!("{}={}", self.config.cookie_config.name, session_id);

        if self.config.cookie_config.http_only {
            cookie.push_str("; HttpOnly");
        }

        if self.config.cookie_config.secure {
            cookie.push_str("; Secure");
        }

        match self.config.cookie_config.same_site {
            SameSitePolicy::Strict => cookie.push_str("; SameSite=Strict"),
            SameSitePolicy::Lax => cookie.push_str("; SameSite=Lax"),
            SameSitePolicy::None => cookie.push_str("; SameSite=None"),
        }

        cookie.push_str(&format!("; Path={}", self.config.cookie_config.path));

        if let Some(domain) = &self.config.cookie_config.domain {
            cookie.push_str(&format!("; Domain={}", domain));
        }

        // Add Max-Age for session timeout
        cookie.push_str(&format!("; Max-Age={}", self.config.session_timeout));

        cookie
    }

    /// Validate session security
    pub fn validate_session_security(
        &self,
        session_info: &SessionSecurityInfo,
        request_info: &RequestSecurityInfo,
    ) -> Result<()> {
        // Check CSRF token if enabled
        if self.config.enable_csrf_protection {
            if let (Some(session_token), Some(request_token)) =
                (&session_info.csrf_token, &request_info.csrf_token)
            {
                if !self.validate_csrf_token(session_token, request_token) {
                    return Err(Error::template("CSRF token validation failed".to_string()));
                }
            } else {
                return Err(Error::template("CSRF token missing".to_string()));
            }
        }

        // Check for hijacking
        let hijacking_risk = self.detect_hijacking(session_info, request_info);
        match hijacking_risk {
            HijackingRisk::High => {
                log::warn!("High hijacking risk detected for session");
                return Err(Error::template(
                    "Suspicious session activity detected".to_string(),
                ));
            }
            HijackingRisk::Medium => {
                log::warn!("Medium hijacking risk detected for session");
                // Could require additional authentication here
            }
            _ => {}
        }

        Ok(())
    }
}

/// Simple base64 URL-safe encoding
fn base64_encode_url_safe(bytes: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::new();

    for chunk in bytes.chunks(3) {
        let b1 = chunk[0];
        let b2 = chunk.get(1).copied().unwrap_or(0);
        let b3 = chunk.get(2).copied().unwrap_or(0);

        let n = ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32);

        result.push(CHARS[((n >> 18) & 63) as usize] as char);
        result.push(CHARS[((n >> 12) & 63) as usize] as char);

        if chunk.len() > 1 {
            result.push(CHARS[((n >> 6) & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            result.push(CHARS[(n & 63) as usize] as char);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_session_id_generation() {
        let generator = SecureSessionIdGenerator::new(32);

        let id1 = generator.generate();
        let id2 = generator.generate();

        // Should be different
        assert_ne!(id1, id2);

        // Should be valid format
        assert!(generator.validate_session_id(&id1));
        assert!(generator.validate_session_id(&id2));

        // Should have reasonable length
        assert!(id1.len() >= 32);
    }

    #[test]
    fn test_csrf_protection() {
        let csrf = CsrfProtection::new(32);

        let token1 = csrf.generate_token();
        let token2 = csrf.generate_token();

        // Tokens should be different
        assert_ne!(token1, token2);

        // Valid token should validate
        assert!(csrf.validate_token(&token1, &token1));

        // Different tokens should not validate
        assert!(!csrf.validate_token(&token1, &token2));

        // Modified token should not validate
        let modified = format!("{}x", &token1[..token1.len() - 1]);
        assert!(!csrf.validate_token(&token1, &modified));
    }

    #[test]
    fn test_hijacking_detection() {
        let detection = HijackingDetection::new(true);

        let session_info = SessionSecurityInfo {
            original_ip: "192.168.1.100".to_string(),
            user_agent: "Mozilla/5.0 (Test)".to_string(),
            created_at: 1000000,
            last_regeneration: 1000000,
            csrf_token: None,
            login_time: None,
        };

        // Same IP and User-Agent should be safe
        let request_info = RequestSecurityInfo {
            client_ip: "192.168.1.100".to_string(),
            user_agent: "Mozilla/5.0 (Test)".to_string(),
            csrf_token: None,
        };

        assert_eq!(
            detection.detect_hijacking(&session_info, &request_info),
            HijackingRisk::None
        );

        // Different IP should increase risk
        let request_info_diff_ip = RequestSecurityInfo {
            client_ip: "192.168.1.200".to_string(),
            user_agent: "Mozilla/5.0 (Test)".to_string(),
            csrf_token: None,
        };

        let risk = detection.detect_hijacking(&session_info, &request_info_diff_ip);
        assert!(risk != HijackingRisk::None);
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = SessionRateLimiter::new(2);

        // Should allow first session
        assert!(limiter.can_create_session("192.168.1.1"));
        limiter.record_session("192.168.1.1");

        // Should allow second session
        assert!(limiter.can_create_session("192.168.1.1"));
        limiter.record_session("192.168.1.1");

        // Should block third session
        assert!(!limiter.can_create_session("192.168.1.1"));

        // Should allow for different IP
        assert!(limiter.can_create_session("192.168.1.2"));

        // Removing session should allow new one
        limiter.remove_session("192.168.1.1");
        assert!(limiter.can_create_session("192.168.1.1"));
    }

    #[test]
    fn test_base64_url_safe_encoding() {
        let bytes = b"Hello, World!";
        let encoded = base64_encode_url_safe(bytes);

        // Should not contain + or / characters
        assert!(!encoded.contains('+'));
        assert!(!encoded.contains('/'));

        // Should contain URL-safe characters
        assert!(encoded
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }
}
