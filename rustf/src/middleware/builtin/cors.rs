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
        let allow_origins =
            CONF::get::<Vec<String>>("middleware.cors.allow_origins").unwrap_or_default();

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
        // Short-circuit ONLY genuine browser preflights. A CORS preflight is an
        // OPTIONS request that carries BOTH `Origin` and
        // `Access-Control-Request-Method`. Plain `OPTIONS` requests (e.g. an
        // app-defined OPTIONS route, or a same-origin capabilities query) must
        // still reach their handler — gating on method alone would force every
        // OPTIONS to 204 and make those routes unreachable.
        if ctx.req.method == "OPTIONS"
            && ctx.req.headers.contains_key("origin")
            && ctx
                .req
                .headers
                .contains_key("access-control-request-method")
        {
            if self.determine_allowed_origin(ctx).is_none() {
                log::debug!("Rejecting CORS preflight from disallowed origin");
                ctx.status(hyper::StatusCode::FORBIDDEN);
                return Ok(InboundAction::Stop);
            }

            log::debug!("Handling CORS preflight request for {}", ctx.req.uri);
            // Set OK status - headers will be added in outbound phase
            ctx.status(hyper::StatusCode::NO_CONTENT);
            return Ok(InboundAction::Stop);
        }

        // All other requests (including non-preflight OPTIONS) continue to the
        // handler and get CORS headers added in the outbound phase.
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
        let Some(allowed_origin) = self.determine_allowed_origin(ctx) else {
            return Ok(());
        };

        // Enforce the spec invariant at emission, not just in the constructors:
        // `Access-Control-Allow-Origin: *` and `Access-Control-Allow-Credentials:
        // true` are mutually exclusive. A hand-built `CorsConfig` (via
        // `with_config`) or a builder ordering like
        // `new().allow_credentials(true).allow_origin("*")` can produce this
        // invalid combination, which the per-constructor guards miss. Emitting
        // it is invalid (browsers reject it), so we send NO CORS headers for
        // this response — safely blocking the cross-origin read and forcing the
        // app to configure explicit origins for credentialed requests.
        if self.config.allow_credentials && allowed_origin == "*" {
            log::warn!(
                "CORS: refusing to emit `Access-Control-Allow-Origin: *` together with \
                 credentials. Configure explicit origins (allow_origins) for credentialed \
                 CORS. No CORS headers sent for this response."
            );
            return Ok(());
        }

        if let Some(response) = ctx.res.as_mut() {
            // Add CORS headers to the response
            Self::upsert_header(
                &mut response.headers,
                "Access-Control-Allow-Origin".to_string(),
                allowed_origin.clone(),
            );

            // Add Vary: Origin for dynamic origin handling (security best practice)
            if !self.config.allow_origins.is_empty() || self.config.allow_credentials {
                Self::merge_vary_header(&mut response.headers, "Origin");
            }

            if !self.config.allow_methods.is_empty() {
                let methods = self.config.allow_methods.join(", ");
                Self::upsert_header(
                    &mut response.headers,
                    "Access-Control-Allow-Methods".to_string(),
                    methods,
                );
            }

            if !self.config.allow_headers.is_empty() {
                let headers = self.config.allow_headers.join(", ");
                Self::upsert_header(
                    &mut response.headers,
                    "Access-Control-Allow-Headers".to_string(),
                    headers,
                );
            }

            if self.config.allow_credentials {
                Self::upsert_header(
                    &mut response.headers,
                    "Access-Control-Allow-Credentials".to_string(),
                    "true".to_string(),
                );
            }

            if let Some(max_age) = self.config.max_age {
                Self::upsert_header(
                    &mut response.headers,
                    "Access-Control-Max-Age".to_string(),
                    max_age.to_string(),
                );
            }
        }

        Ok(())
    }
}

impl CorsMiddleware {
    /// Determine which origin to allow based on request and configuration
    fn determine_allowed_origin(&self, ctx: &Context) -> Option<String> {
        if self.config.allow_origin.is_empty() && self.config.allow_origins.is_empty() {
            return None;
        }

        // If wildcard is set and no additional origins, use wildcard
        if self.config.allow_origin == "*" && self.config.allow_origins.is_empty() {
            return Some("*".to_string());
        }

        // If multiple origins configured, validate against request Origin header
        if !self.config.allow_origins.is_empty() {
            if let Some(request_origin) = ctx.req.headers.get("origin") {
                // Check if request origin is in the allowed list
                if self
                    .config
                    .allow_origins
                    .iter()
                    .any(|o| o == request_origin)
                {
                    return Some(request_origin.clone());
                }
            }
            // Request origin not allowed - omit CORS headers entirely.
            return None;
        }

        // Explicit single origin configuration: if the request carries an
        // Origin header, emit ACAO only when it matches the configured origin.
        if let Some(request_origin) = ctx.req.headers.get("origin") {
            if request_origin != &self.config.allow_origin {
                return None;
            }
        }

        // Single origin configured
        Some(self.config.allow_origin.clone())
    }

    fn upsert_header(headers: &mut Vec<(String, String)>, name: String, value: String) {
        headers.retain(|(existing, _)| !existing.eq_ignore_ascii_case(&name));
        headers.push((name, value));
    }

    fn merge_vary_header(headers: &mut Vec<(String, String)>, value: &str) {
        if let Some((_, existing)) = headers
            .iter_mut()
            .find(|(name, _)| name.eq_ignore_ascii_case("vary"))
        {
            let already_present = existing
                .split(',')
                .any(|part| part.trim().eq_ignore_ascii_case(value));
            if !already_present {
                if !existing.is_empty() {
                    existing.push_str(", ");
                }
                existing.push_str(value);
            }
            return;
        }

        headers.push(("Vary".to_string(), value.to_string()));
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

        // Create a GENUINE preflight: OPTIONS + Origin + Access-Control-Request-Method
        let mut request = Request::default();
        request.method = "OPTIONS".to_string();
        request.uri = "/api/test".to_string();
        request
            .headers
            .insert("origin".to_string(), "https://example.com".to_string());
        request.headers.insert(
            "access-control-request-method".to_string(),
            "POST".to_string(),
        );

        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);

        // Process preflight request
        let action = middleware.process_request(&mut ctx).await.unwrap();

        // Preflight should short-circuit routing while still allowing
        // outbound processing in the application chain.
        assert!(matches!(action, InboundAction::Stop));

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
        let middleware = CorsMiddleware::new().allow_origins(vec![
            "https://app1.example.com".to_string(),
            "https://app2.example.com".to_string(),
        ]);

        // Test with allowed origin
        let mut request = Request::default();
        request.method = "GET".to_string();
        request
            .headers
            .insert("origin".to_string(), "https://app1.example.com".to_string());

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
        let middleware = CorsMiddleware::new().allow_origins(vec![
            "https://app1.example.com".to_string(),
            "https://app2.example.com".to_string(),
        ]);

        // Test with disallowed origin
        let mut request = Request::default();
        request.method = "GET".to_string();
        request
            .headers
            .insert("origin".to_string(), "https://evil.com".to_string());

        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);

        middleware.process_response(&mut ctx).await.unwrap();

        if let Some(response) = &ctx.res {
            let has_origin_header = response
                .headers
                .iter()
                .any(|(k, _)| k == "Access-Control-Allow-Origin");
            assert!(!has_origin_header, "disallowed origins must not emit ACAO");
        } else {
            panic!("Expected response in context");
        }
    }

    #[tokio::test]
    async fn test_cors_wildcard_with_credentials_validation() {
        // Attempting to set credentials with wildcard should clear origin
        let middleware = CorsMiddleware::new().allow_credentials(true);

        let mut request = Request::default();
        request.method = "GET".to_string();

        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);

        middleware.process_response(&mut ctx).await.unwrap();

        if let Some(response) = &ctx.res {
            let has_origin_header = response
                .headers
                .iter()
                .any(|(k, _)| k == "Access-Control-Allow-Origin");
            assert!(
                !has_origin_header,
                "wildcard + credentials must not emit ACAO"
            );
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
            let vary_header = response.headers.iter().find(|(k, _)| k == "Vary");
            assert!(vary_header.is_none());
        } else {
            panic!("Expected response in context");
        }
    }

    // Helper: run outbound and return whether the response carries a given header.
    async fn emits_header(middleware: &CorsMiddleware, header: &str) -> bool {
        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(Request::default(), views);
        ctx.set_response(crate::http::Response::ok());
        middleware.process_response(&mut ctx).await.unwrap();
        ctx.res
            .as_ref()
            .unwrap()
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case(header))
    }

    // The `*` + credentials invariant must be enforced at emission, not just in
    // the constructors — `with_config` bypasses the constructor guards entirely.
    #[tokio::test]
    async fn wildcard_with_credentials_via_with_config_emits_no_cors_headers() {
        let mut cfg = CorsConfig::default();
        cfg.allow_origin = "*".to_string();
        cfg.allow_credentials = true;
        let mw = CorsMiddleware::with_config(cfg);

        assert!(
            !emits_header(&mw, "Access-Control-Allow-Origin").await,
            "must not emit ACAO:* with credentials"
        );
        assert!(
            !emits_header(&mw, "Access-Control-Allow-Credentials").await,
            "must not emit credentials with wildcard origin"
        );
    }

    // Builder ordering that re-sets `*` after enabling credentials also bypasses
    // the constructor guard; emission-time enforcement still catches it.
    #[tokio::test]
    async fn wildcard_with_credentials_via_builder_order_emits_no_cors_headers() {
        let mw = CorsMiddleware::new()
            .allow_credentials(true)
            .allow_origin("*");

        assert!(!emits_header(&mw, "Access-Control-Allow-Origin").await);
        assert!(!emits_header(&mw, "Access-Control-Allow-Credentials").await);
    }

    // A plain OPTIONS request (no Origin / Access-Control-Request-Method) is NOT
    // a preflight — it must continue to the route handler, not be forced to 204.
    #[tokio::test]
    async fn plain_options_is_not_treated_as_preflight() {
        let mw = CorsMiddleware::new();
        let mut req = Request::default();
        req.method = "OPTIONS".to_string();
        req.uri = "/api/widgets".to_string();
        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(req, views);

        let action = mw.process_request(&mut ctx).await.unwrap();
        // Must Capture (continue to handler), NOT Stop.
        assert!(matches!(action, InboundAction::Capture));
    }

    // OPTIONS with Origin but WITHOUT Access-Control-Request-Method is also not a
    // preflight (e.g. a same-origin or manual OPTIONS call).
    #[tokio::test]
    async fn options_with_origin_but_no_request_method_is_not_preflight() {
        let mw = CorsMiddleware::new();
        let mut req = Request::default();
        req.method = "OPTIONS".to_string();
        req.headers
            .insert("origin".to_string(), "https://example.com".to_string());
        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(req, views);

        let action = mw.process_request(&mut ctx).await.unwrap();
        assert!(matches!(action, InboundAction::Capture));
    }

    #[tokio::test]
    async fn disallowed_preflight_returns_forbidden() {
        let mw = CorsMiddleware::new().allow_origins(vec!["https://good.example".to_string()]);
        let mut req = Request::default();
        req.method = "OPTIONS".to_string();
        req.headers
            .insert("origin".to_string(), "https://evil.example".to_string());
        req.headers.insert(
            "access-control-request-method".to_string(),
            "POST".to_string(),
        );
        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(req, views);

        let action = mw.process_request(&mut ctx).await.unwrap();
        assert!(matches!(action, InboundAction::Stop));
        assert_eq!(
            ctx.res.as_ref().map(|res| res.status),
            Some(hyper::StatusCode::FORBIDDEN)
        );
    }

    #[tokio::test]
    async fn existing_cors_headers_are_replaced_without_duplicates() {
        let mw = CorsMiddleware::new().allow_origins(vec!["https://app1.example.com".to_string()]);
        let mut req = Request::default();
        req.method = "GET".to_string();
        req.headers
            .insert("origin".to_string(), "https://app1.example.com".to_string());

        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(req, views);
        if let Some(response) = ctx.res.as_mut() {
            response.headers.push((
                "Access-Control-Allow-Origin".to_string(),
                "https://stale.example.com".to_string(),
            ));
            response
                .headers
                .push(("Vary".to_string(), "Accept-Encoding".to_string()));
        }

        mw.process_response(&mut ctx).await.unwrap();

        let response = ctx.res.as_ref().unwrap();
        let acao_values: Vec<_> = response
            .headers
            .iter()
            .filter(|(name, _)| name.eq_ignore_ascii_case("access-control-allow-origin"))
            .map(|(_, value)| value.as_str())
            .collect();
        assert_eq!(acao_values, vec!["https://app1.example.com"]);

        let vary_values: Vec<_> = response
            .headers
            .iter()
            .filter(|(name, _)| name.eq_ignore_ascii_case("vary"))
            .map(|(_, value)| value.as_str())
            .collect();
        assert_eq!(vary_values, vec!["Accept-Encoding, Origin"]);
    }
}
