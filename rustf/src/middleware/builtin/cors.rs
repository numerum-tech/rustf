//! CORS (Cross-Origin Resource Sharing) middleware for RustF
//!
//! This middleware handles CORS headers and preflight requests, demonstrating
//! dual-phase middleware that can both modify responses and stop the chain for preflight.

use crate::context::Context;
use crate::error::Result;
use crate::middleware::{InboundAction, InboundMiddleware, OutboundMiddleware};
use async_trait::async_trait;

/// CORS middleware configuration
#[derive(Clone)]
pub struct CorsConfig {
    pub allow_origin: String,
    /// Additional allowed origins (for multi-origin support)
    pub allow_origins: Vec<String>,
    pub allow_methods: Vec<String>,
    pub allow_headers: Vec<String>,
    pub allow_credentials: bool,
    pub max_age: Option<u32>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allow_origin: "*".to_string(),
            allow_origins: Vec::new(),
            allow_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allow_headers: vec![
                "Content-Type".to_string(),
                "Authorization".to_string(),
                "X-Requested-With".to_string(),
            ],
            allow_credentials: false,
            max_age: Some(86400), // 24 hours
        }
    }
}

/// CORS middleware
///
/// Handles CORS headers and preflight OPTIONS requests.
/// Demonstrates dual-phase middleware that can stop the chain (for preflight) or modify responses.
#[derive(Clone)]
pub struct CorsMiddleware {
    config: CorsConfig,
}

impl CorsMiddleware {
    /// Create CORS middleware with default configuration
    pub fn new() -> Self {
        Self {
            config: CorsConfig::default(),
        }
    }

    /// Create CORS middleware with custom configuration
    pub fn with_config(config: CorsConfig) -> Self {
        Self { config }
    }

    /// Builder method to set allowed origin
    pub fn allow_origin(mut self, origin: &str) -> Self {
        self.config.allow_origin = origin.to_string();
        self
    }

    /// Builder method to set multiple allowed origins
    pub fn allow_origins(mut self, origins: Vec<String>) -> Self {
        self.config.allow_origins = origins;
        self
    }

    /// Builder method to set allowed methods
    pub fn allow_methods(mut self, methods: Vec<&str>) -> Self {
        self.config.allow_methods = methods.into_iter().map(|s| s.to_string()).collect();
        self
    }

    /// Builder method to set allowed headers
    pub fn allow_headers(mut self, headers: Vec<String>) -> Self {
        self.config.allow_headers = headers;
        self
    }

    /// Builder method to set credentials support
    pub fn allow_credentials(mut self, allow: bool) -> Self {
        self.config.allow_credentials = allow;

        // Validate: cannot use wildcard with credentials
        if allow && self.config.allow_origin == "*" {
            log::warn!(
                "CORS Security Warning: Access-Control-Allow-Credentials: true cannot be used with \
                Access-Control-Allow-Origin: *. Setting allow_origin to empty (will block all requests). \
                Please specify explicit origins."
            );
            self.config.allow_origin = String::new();
        }

        self
    }

    /// Create CORS middleware from configuration file
    ///
    /// Reads configuration from `[middleware.cors]` section in config.toml:
    ///
    /// ```toml
    /// [middleware.cors]
    /// allow_origin = "*"
    /// allow_methods = ["GET", "POST", "PUT", "DELETE", "OPTIONS"]
    /// allow_headers = ["Content-Type", "Authorization"]
    /// allow_credentials = false
    /// max_age = 86400
    /// ```
    ///
    /// If configuration is not found, uses sensible defaults.
    pub fn from_config() -> Self {
        use crate::configuration::CONF;

        let mut allow_origin =
            CONF::get("middleware.cors.allow_origin").unwrap_or_else(|| "*".to_string());

        let allow_methods = CONF::get::<Vec<String>>("middleware.cors.allow_methods")
            .unwrap_or_else(|| {
                vec![
                    "GET".to_string(),
                    "POST".to_string(),
                    "PUT".to_string(),
                    "DELETE".to_string(),
                    "OPTIONS".to_string(),
                ]
            });

        let allow_headers = CONF::get::<Vec<String>>("middleware.cors.allow_headers")
            .unwrap_or_else(|| {
                vec![
                    "Content-Type".to_string(),
                    "Authorization".to_string(),
                    "X-Requested-With".to_string(),
                ]
            });

        let allow_credentials = CONF::get("middleware.cors.allow_credentials").unwrap_or(false);

        // Validate: cannot use wildcard with credentials
        if allow_credentials && allow_origin == "*" {
            log::warn!(
                "CORS Security Warning: Access-Control-Allow-Credentials: true cannot be used with \
                Access-Control-Allow-Origin: *. Setting allow_origin to empty (will block all requests). \
                Please specify explicit origins in config."
            );
            allow_origin = String::new();
        }

        let max_age = CONF::get::<u32>("middleware.cors.max_age");

        // Support multiple origins from config
        let allow_origins = CONF::get::<Vec<String>>("middleware.cors.allow_origins")
            .unwrap_or_default();

        let config = CorsConfig {
            allow_origin,
            allow_origins,
            allow_methods,
            allow_headers,
            allow_credentials,
            max_age: max_age.or(Some(86400)),
        };

        Self::with_config(config)
    }

}

impl Default for CorsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InboundMiddleware for CorsMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        // Handle preflight OPTIONS requests
        if ctx.req.method == "OPTIONS" {
            log::debug!("Handling CORS preflight request for {}", ctx.req.uri);
            // Set OK status - headers will be added in outbound phase
            ctx.status(hyper::StatusCode::NO_CONTENT);
        }

        // All requests (including OPTIONS) should go through outbound phase for CORS headers
        Ok(InboundAction::Capture)
    }

    fn name(&self) -> &'static str {
        "cors"
    }

    fn priority(&self) -> i32 {
        -600 // High priority (runs early, but after logging)
    }
}

#[async_trait]
impl OutboundMiddleware for CorsMiddleware {
    async fn process_response(&self, ctx: &mut Context) -> Result<()> {
        // Determine which origin to allow before getting mutable reference
        let allowed_origin = self.determine_allowed_origin(ctx);

        if let Some(response) = ctx.res.as_mut() {
            // Add CORS headers to the response
            response.headers.push((
                "Access-Control-Allow-Origin".to_string(),
                allowed_origin.clone(),
            ));

            // Add Vary: Origin for dynamic origin handling (security best practice)
            if !self.config.allow_origins.is_empty() || self.config.allow_credentials {
                response.headers.push((
                    "Vary".to_string(),
                    "Origin".to_string(),
                ));
            }

            if !self.config.allow_methods.is_empty() {
                let methods = self.config.allow_methods.join(", ");
                response
                    .headers
                    .push(("Access-Control-Allow-Methods".to_string(), methods));
            }

            if !self.config.allow_headers.is_empty() {
                let headers = self.config.allow_headers.join(", ");
                response
                    .headers
                    .push(("Access-Control-Allow-Headers".to_string(), headers));
            }

            if self.config.allow_credentials {
                response.headers.push((
                    "Access-Control-Allow-Credentials".to_string(),
                    "true".to_string(),
                ));
            }

            if let Some(max_age) = self.config.max_age {
                response
                    .headers
                    .push(("Access-Control-Max-Age".to_string(), max_age.to_string()));
            }
        }

        Ok(())
    }
}

impl CorsMiddleware {
    /// Determine which origin to allow based on request and configuration
    fn determine_allowed_origin(&self, ctx: &Context) -> String {
        // If wildcard is set and no additional origins, use wildcard
        if self.config.allow_origin == "*" && self.config.allow_origins.is_empty() {
            return "*".to_string();
        }

        // If multiple origins configured, validate against request Origin header
        if !self.config.allow_origins.is_empty() {
            if let Some(request_origin) = ctx.req.headers.get("origin") {
                // Check if request origin is in the allowed list
                if self.config.allow_origins.iter().any(|o| o == request_origin) {
                    return request_origin.clone();
                }
            }
            // Request origin not allowed - return empty (will block CORS)
            return String::new();
        }

        // Single origin configured
        self.config.allow_origin.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::Request;
    use crate::views::ViewEngine;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_cors_preflight() {
        let middleware = CorsMiddleware::new();

        // Create OPTIONS request (preflight)
        let mut request = Request::default();
        request.method = "OPTIONS".to_string();
        request.uri = "/api/test".to_string();

        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);

        // Process preflight request
        let action = middleware.process_request(&mut ctx).await.unwrap();

        // Should now use Capture action (goes through outbound phase)
        assert!(matches!(action, InboundAction::Capture));

        // Process outbound to add CORS headers
        middleware.process_response(&mut ctx).await.unwrap();

        // Verify CORS headers were added
        if let Some(response) = &ctx.res {
            let has_cors_origin = response
                .headers
                .iter()
                .any(|(k, _)| k == "Access-Control-Allow-Origin");
            assert!(has_cors_origin);

            // Verify status is 204 No Content for preflight
            assert_eq!(response.status, hyper::StatusCode::NO_CONTENT);
        } else {
            panic!("Expected response in context");
        }
    }

    #[tokio::test]
    async fn test_cors_regular_request() {
        let middleware = CorsMiddleware::new();

        // Create regular GET request
        let mut request = Request::default();
        request.method = "GET".to_string();
        request.uri = "/api/data".to_string();

        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);

        // Process regular request
        let action = middleware.process_request(&mut ctx).await.unwrap();

        // Should capture for processing response
        assert!(matches!(action, InboundAction::Capture));

        // Test outbound processing
        // Context already has default OK response
        middleware.process_response(&mut ctx).await.unwrap();

        // Check that CORS headers were added
        if let Some(response) = &ctx.res {
            let has_cors_origin = response
                .headers
                .iter()
                .any(|(k, _)| k == "Access-Control-Allow-Origin");
            assert!(has_cors_origin);
        } else {
            panic!("Expected response in context");
        }
    }

    #[tokio::test]
    async fn test_cors_custom_config() {
        let config = CorsConfig {
            allow_origin: "https://example.com".to_string(),
            allow_origins: Vec::new(),
            allow_methods: vec!["GET".to_string(), "POST".to_string()],
            allow_headers: vec!["Content-Type".to_string()],
            allow_credentials: true,
            max_age: Some(3600),
        };

        let middleware = CorsMiddleware::with_config(config);

        let mut request = Request::default();
        request.method = "GET".to_string();

        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);

        // Context already has default OK response
        middleware.process_response(&mut ctx).await.unwrap();

        // Verify custom headers
        if let Some(response) = &ctx.res {
            let origin_header = response
                .headers
                .iter()
                .find(|(k, _)| k == "Access-Control-Allow-Origin")
                .map(|(_, v)| v.as_str());
            assert_eq!(origin_header, Some("https://example.com"));

            let credentials_header = response
                .headers
                .iter()
                .find(|(k, _)| k == "Access-Control-Allow-Credentials")
                .map(|(_, v)| v.as_str());
            assert_eq!(credentials_header, Some("true"));

            // Verify Vary: Origin header is present for credentialed requests
            let vary_header = response
                .headers
                .iter()
                .find(|(k, _)| k == "Vary")
                .map(|(_, v)| v.as_str());
            assert_eq!(vary_header, Some("Origin"));
        } else {
            panic!("Expected response in context");
        }
    }

    #[tokio::test]
    async fn test_cors_multiple_origins() {
        let middleware = CorsMiddleware::new()
            .allow_origins(vec![
                "https://app1.example.com".to_string(),
                "https://app2.example.com".to_string(),
            ]);

        // Test with allowed origin
        let mut request = Request::default();
        request.method = "GET".to_string();
        request.headers.insert(
            "origin".to_string(),
            "https://app1.example.com".to_string(),
        );

        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);

        middleware.process_response(&mut ctx).await.unwrap();

        if let Some(response) = &ctx.res {
            let origin_header = response
                .headers
                .iter()
                .find(|(k, _)| k == "Access-Control-Allow-Origin")
                .map(|(_, v)| v.as_str());
            assert_eq!(origin_header, Some("https://app1.example.com"));

            // Verify Vary: Origin header
            let vary_header = response
                .headers
                .iter()
                .find(|(k, _)| k == "Vary")
                .map(|(_, v)| v.as_str());
            assert_eq!(vary_header, Some("Origin"));
        } else {
            panic!("Expected response in context");
        }
    }

    #[tokio::test]
    async fn test_cors_multiple_origins_rejected() {
        let middleware = CorsMiddleware::new()
            .allow_origins(vec![
                "https://app1.example.com".to_string(),
                "https://app2.example.com".to_string(),
            ]);

        // Test with disallowed origin
        let mut request = Request::default();
        request.method = "GET".to_string();
        request.headers.insert(
            "origin".to_string(),
            "https://evil.com".to_string(),
        );

        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);

        middleware.process_response(&mut ctx).await.unwrap();

        if let Some(response) = &ctx.res {
            let origin_header = response
                .headers
                .iter()
                .find(|(k, _)| k == "Access-Control-Allow-Origin")
                .map(|(_, v)| v.as_str());
            // Should return empty string (blocks CORS)
            assert_eq!(origin_header, Some(""));
        } else {
            panic!("Expected response in context");
        }
    }

    #[tokio::test]
    async fn test_cors_wildcard_with_credentials_validation() {
        // Attempting to set credentials with wildcard should clear origin
        let middleware = CorsMiddleware::new()
            .allow_credentials(true);

        let mut request = Request::default();
        request.method = "GET".to_string();

        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);

        middleware.process_response(&mut ctx).await.unwrap();

        if let Some(response) = &ctx.res {
            let origin_header = response
                .headers
                .iter()
                .find(|(k, _)| k == "Access-Control-Allow-Origin")
                .map(|(_, v)| v.as_str());
            // Origin should be empty (blocked due to security validation)
            assert_eq!(origin_header, Some(""));
        } else {
            panic!("Expected response in context");
        }
    }

    #[tokio::test]
    async fn test_cors_wildcard_without_credentials() {
        // Wildcard without credentials should work
        let middleware = CorsMiddleware::new();

        let mut request = Request::default();
        request.method = "GET".to_string();

        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);

        middleware.process_response(&mut ctx).await.unwrap();

        if let Some(response) = &ctx.res {
            let origin_header = response
                .headers
                .iter()
                .find(|(k, _)| k == "Access-Control-Allow-Origin")
                .map(|(_, v)| v.as_str());
            assert_eq!(origin_header, Some("*"));

            // No Vary header for wildcard
            let vary_header = response
                .headers
                .iter()
                .find(|(k, _)| k == "Vary");
            assert!(vary_header.is_none());
        } else {
            panic!("Expected response in context");
        }
    }
}
