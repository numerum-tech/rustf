//! Rate limiting middleware for RustF
//!
//! This middleware provides IP-based rate limiting to protect against abuse
//! and denial of service attacks.

use crate::context::Context;
use crate::error::Result;
use crate::http::Response;
use crate::middleware::{InboundAction, InboundMiddleware};
use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::json;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Rate limiting entry for tracking request counts
#[derive(Clone)]
struct RateLimitEntry {
    count: u32,
    window_start: u64,
}

/// Advanced rate limiting middleware with configurable windows and limits
#[derive(Clone)]
pub struct RateLimitMiddleware {
    /// Maximum requests per window
    max_requests: u32,
    /// Time window in seconds
    window_seconds: u64,
    /// Storage for rate limit entries
    storage: Arc<DashMap<String, RateLimitEntry>>,
    /// Paths to exclude from rate limiting
    excluded_paths: Vec<String>,
    /// Whether to use X-Forwarded-For header
    trust_proxy: bool,
}

impl RateLimitMiddleware {
    /// Create new rate limiter with specified requests per window
    pub fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            max_requests,
            window_seconds,
            storage: Arc::new(DashMap::new()),
            excluded_paths: vec![
                "/health".to_string(),
                "/metrics".to_string(),
                "/favicon.ico".to_string(),
            ],
            trust_proxy: true,
        }
    }

    /// Create rate limiter with 100 requests per minute (default)
    pub fn default() -> Self {
        Self::new(100, 60)
    }

    /// Create rate limiter for API endpoints (stricter)
    pub fn api() -> Self {
        Self::new(60, 60)
    }

    /// Create rate limiter for authentication endpoints (very strict)
    pub fn auth() -> Self {
        Self::new(5, 60)
    }

    /// Add paths to exclude from rate limiting
    pub fn exclude_paths(mut self, paths: Vec<&str>) -> Self {
        self.excluded_paths = paths.into_iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set whether to trust X-Forwarded-For header
    pub fn trust_proxy(mut self, trust: bool) -> Self {
        self.trust_proxy = trust;
        self
    }

    /// Create rate limiter from configuration file
    ///
    /// Reads configuration from `[middleware.rate_limit]` section in config.toml:
    ///
    /// ```toml
    /// [middleware.rate_limit]
    /// max_requests = 100
    /// window_seconds = 60
    /// excluded_paths = ["/health", "/metrics"]
    /// trust_proxy = true
    /// ```
    ///
    /// If configuration is not found, uses sensible defaults (100 req/min).
    pub fn from_config() -> Self {
        use crate::configuration::CONF;

        let max_requests = CONF::get("middleware.rate_limit.max_requests").unwrap_or(100);

        let window_seconds = CONF::get("middleware.rate_limit.window_seconds").unwrap_or(60);

        let mut middleware = Self::new(max_requests, window_seconds);

        // Optional: excluded paths
        if let Some(excluded) = CONF::get::<Vec<String>>("middleware.rate_limit.excluded_paths") {
            middleware.excluded_paths = excluded;
        }

        // Optional: trust proxy setting
        if let Some(trust) = CONF::get("middleware.rate_limit.trust_proxy") {
            middleware.trust_proxy = trust;
        }

        middleware
    }

    /// Get client identifier from request
    fn get_client_id(&self, ctx: &Context) -> String {
        if self.trust_proxy {
            // Try to get real IP from proxy headers
            ctx.req
                .headers
                .get("x-forwarded-for")
                .and_then(|h| h.split(',').next())
                .map(|s| s.trim().to_string())
                .or_else(|| ctx.req.headers.get("x-real-ip").cloned())
                .unwrap_or_else(|| ctx.ip())
        } else {
            ctx.ip()
        }
    }

    /// Check if path is excluded from rate limiting
    fn is_excluded(&self, path: &str) -> bool {
        self.excluded_paths
            .iter()
            .any(|excluded| path == excluded || path.starts_with(excluded))
    }

    /// Clean up old entries periodically
    fn cleanup_old_entries(&self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Remove entries older than 2x the window
        let cutoff = current_time - (self.window_seconds * 2);

        self.storage.retain(|_, entry| entry.window_start > cutoff);
    }
}

#[async_trait]
impl InboundMiddleware for RateLimitMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        // Skip rate limiting for excluded paths
        if self.is_excluded(&ctx.req.uri) {
            return Ok(InboundAction::Continue);
        }

        let client_id = self.get_client_id(ctx);
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Occasionally clean up old entries (1% chance)
        if rand::random::<f32>() < 0.01 {
            self.cleanup_old_entries();
        }

        // Check and update rate limit
        let mut entry = self
            .storage
            .entry(client_id.clone())
            .or_insert_with(|| RateLimitEntry {
                count: 0,
                window_start: current_time,
            });

        // Reset window if expired
        if current_time - entry.window_start >= self.window_seconds {
            entry.count = 0;
            entry.window_start = current_time;
        }

        // Increment request count
        entry.count += 1;

        // Check if limit exceeded
        if entry.count > self.max_requests {
            log::warn!(
                "Rate limit exceeded for client: {} ({}/{})",
                client_id,
                entry.count,
                self.max_requests
            );

            // Calculate retry after
            let retry_after = self.window_seconds - (current_time - entry.window_start);

            // Set rate limit error response using context
            ctx.set_response(Response::new(hyper::StatusCode::TOO_MANY_REQUESTS)
                .with_header("Content-Type", "application/json")
                .with_header("Retry-After", &retry_after.to_string())
                .with_header("X-RateLimit-Limit", &self.max_requests.to_string())
                .with_header("X-RateLimit-Remaining", "0")
                .with_header("X-RateLimit-Reset", &(entry.window_start + self.window_seconds).to_string())
                .with_body(json!({
                    "error": "rate_limit_exceeded",
                    "message": format!("Rate limit exceeded. Maximum {} requests per {} seconds", 
                                     self.max_requests, self.window_seconds),
                    "retry_after": retry_after
                }).to_string().into_bytes()));

            return Ok(InboundAction::Stop);
        }

        // Add rate limit headers to context for response
        let remaining = self.max_requests - entry.count;
        let _ = ctx.set("rate_limit_limit", self.max_requests);
        let _ = ctx.set("rate_limit_remaining", remaining);
        let _ = ctx.set("rate_limit_reset", entry.window_start + self.window_seconds);

        Ok(InboundAction::Continue)
    }

    fn name(&self) -> &'static str {
        "rate_limit"
    }

    fn priority(&self) -> i32 {
        -900 // Run very early, after CORS but before most middleware
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_creation() {
        let limiter = RateLimitMiddleware::default();
        assert_eq!(limiter.max_requests, 100);
        assert_eq!(limiter.window_seconds, 60);

        let api_limiter = RateLimitMiddleware::api();
        assert_eq!(api_limiter.max_requests, 60);

        let auth_limiter = RateLimitMiddleware::auth();
        assert_eq!(auth_limiter.max_requests, 5);
    }

    #[test]
    fn test_excluded_paths() {
        let limiter = RateLimitMiddleware::default().exclude_paths(vec!["/public", "/static"]);

        assert!(limiter.is_excluded("/public/file.js"));
        assert!(limiter.is_excluded("/static/image.png"));
        assert!(!limiter.is_excluded("/api/users"));
    }
}
