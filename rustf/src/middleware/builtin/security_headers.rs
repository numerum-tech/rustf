//! Security headers middleware for RustF
//!
//! This middleware adds comprehensive security headers to all HTTP responses,
//! providing defense-in-depth against common web vulnerabilities.

use crate::context::Context;
use crate::error::Result;
use crate::middleware::OutboundMiddleware;
use crate::security::headers::SecurityHeaders;
use async_trait::async_trait;

/// Security headers middleware that adds comprehensive security headers to responses
#[derive(Clone)]
pub struct SecurityHeadersMiddleware {
    headers: SecurityHeaders,
    enabled: bool,
}

impl SecurityHeadersMiddleware {
    /// Create new security headers middleware with default secure headers
    pub fn new() -> Self {
        Self {
            headers: SecurityHeaders::new(),
            enabled: true,
        }
    }

    /// Create security headers middleware with custom configuration
    pub fn with_headers(headers: SecurityHeaders) -> Self {
        Self {
            headers,
            enabled: true,
        }
    }

    /// Create security headers middleware for development (less strict)
    pub fn development() -> Self {
        Self {
            headers: SecurityHeaders::development(),
            enabled: true,
        }
    }

    /// Create strict security headers middleware for production
    pub fn strict() -> Self {
        Self {
            headers: SecurityHeaders::strict(),
            enabled: true,
        }
    }

    /// Disable security headers (not recommended)
    pub fn disabled() -> Self {
        Self {
            headers: SecurityHeaders::new(),
            enabled: false,
        }
    }
}

impl Default for SecurityHeadersMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OutboundMiddleware for SecurityHeadersMiddleware {
    async fn process_response(&self, ctx: &mut Context) -> Result<()> {
        if self.enabled {
            if let Some(response) = ctx.res.as_mut() {
                self.headers.apply_to_headers(&mut response.headers);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_headers_middleware_creation() {
        let middleware = SecurityHeadersMiddleware::new();
        assert!(middleware.enabled);

        let dev_middleware = SecurityHeadersMiddleware::development();
        assert!(dev_middleware.enabled);

        let strict_middleware = SecurityHeadersMiddleware::strict();
        assert!(strict_middleware.enabled);

        let disabled_middleware = SecurityHeadersMiddleware::disabled();
        assert!(!disabled_middleware.enabled);
    }

    #[tokio::test]
    async fn test_security_headers_application() {
        use crate::http::Request;
        use crate::views::ViewEngine;
        use std::sync::Arc;

        let middleware = SecurityHeadersMiddleware::new();
        let request = Request::default();
        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);

        // Context already has default OK response
        middleware.process_response(&mut ctx).await.unwrap();

        // Headers should be added (actual headers depend on SecurityHeaders implementation)
        // This test assumes SecurityHeaders adds at least some headers
    }
}
