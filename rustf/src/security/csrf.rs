use crate::context::Context;
use crate::error::Result;
use crate::middleware::{InboundAction, InboundMiddleware};
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashSet;

/// CSRF protection configuration
#[derive(Debug, Clone)]
pub struct CsrfConfig {
    /// Routes to exempt from CSRF protection (supports glob patterns)
    pub exempt_routes: Vec<String>,
    /// HTTP methods that require CSRF protection
    pub protected_methods: HashSet<String>,
    /// Whether to enable CSRF protection globally
    pub enabled: bool,
    /// Custom error message for CSRF failures
    pub error_message: String,
    /// Redirect URL on CSRF failure (if None, returns error response)
    pub redirect_on_failure: Option<String>,
    /// Flash message key for CSRF errors
    pub flash_error_key: String,
}

impl Default for CsrfConfig {
    fn default() -> Self {
        let mut protected_methods = HashSet::new();
        protected_methods.insert("POST".to_string());
        protected_methods.insert("PUT".to_string());
        protected_methods.insert("PATCH".to_string());
        protected_methods.insert("DELETE".to_string());

        Self {
            exempt_routes: vec!["/api/*".to_string()],
            protected_methods,
            enabled: true,
            error_message: "CSRF token validation failed. Please try again.".to_string(),
            redirect_on_failure: None,
            flash_error_key: "error_msg".to_string(),
        }
    }
}

impl CsrfConfig {
    /// Create new CSRF configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Add route to exemption list
    pub fn exempt<S: Into<String>>(mut self, route: S) -> Self {
        self.exempt_routes.push(route.into());
        self
    }

    /// Set custom error message
    pub fn error_message<S: Into<String>>(mut self, message: S) -> Self {
        self.error_message = message.into();
        self
    }

    /// Set redirect URL for CSRF failures
    pub fn redirect_on_failure<S: Into<String>>(mut self, url: S) -> Self {
        self.redirect_on_failure = Some(url.into());
        self
    }

    /// Disable CSRF protection
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Add custom protected HTTP method
    pub fn protect_method<S: Into<String>>(mut self, method: S) -> Self {
        self.protected_methods.insert(method.into().to_uppercase());
        self
    }

    /// Remove HTTP method from protection
    pub fn exempt_method<S: Into<String>>(mut self, method: S) -> Self {
        self.protected_methods.remove(&method.into().to_uppercase());
        self
    }
}

/// CSRF protection middleware
#[derive(Clone)]
pub struct CsrfMiddleware {
    config: CsrfConfig,
}

impl CsrfMiddleware {
    /// Create new CSRF middleware with default configuration
    pub fn new() -> Self {
        Self {
            config: CsrfConfig::default(),
        }
    }

    /// Create CSRF middleware with custom configuration
    pub fn with_config(config: CsrfConfig) -> Self {
        Self { config }
    }

    /// Create CSRF middleware from configuration file
    ///
    /// Reads configuration from `[middleware.csrf]` section in config.toml:
    ///
    /// ```toml
    /// [middleware.csrf]
    /// exempt_routes = ["/api/*", "/webhook/*"]
    /// error_message = "CSRF validation failed"
    /// redirect_on_failure = "/error"
    /// enabled = true
    /// ```
    ///
    /// If configuration is not found, uses sensible defaults.
    pub fn from_config() -> Self {
        use crate::configuration::CONF;

        let mut config = CsrfConfig::default();

        // Load exempt routes
        if let Some(exempt_routes) = CONF::get::<Vec<String>>("middleware.csrf.exempt_routes") {
            config.exempt_routes = exempt_routes;
        }

        // Load error message
        if let Some(error_message) = CONF::get::<String>("middleware.csrf.error_message") {
            config.error_message = error_message;
        }

        // Load redirect URL
        if let Some(redirect_url) = CONF::get::<String>("middleware.csrf.redirect_on_failure") {
            config.redirect_on_failure = Some(redirect_url);
        }

        // Load enabled flag
        if let Some(enabled) = CONF::get("middleware.csrf.enabled") {
            config.enabled = enabled;
        }

        // Load flash error key
        if let Some(flash_key) = CONF::get::<String>("middleware.csrf.flash_error_key") {
            config.flash_error_key = flash_key;
        }

        Self::with_config(config)
    }

    /// Check if HTTP method requires CSRF protection
    fn requires_protection(&self, method: &str) -> bool {
        self.config
            .protected_methods
            .contains(&method.to_uppercase())
    }

    /// Check if route is exempt from CSRF protection
    fn is_route_exempt(&self, path: &str) -> bool {
        for exempt_pattern in &self.config.exempt_routes {
            if self.matches_pattern(path, exempt_pattern) {
                return true;
            }
        }
        false
    }

    /// Simple glob pattern matching for routes
    fn matches_pattern(&self, path: &str, pattern: &str) -> bool {
        if let Some(prefix) = pattern.strip_suffix("/*") {
            // For /* pattern, path must start with prefix AND have a separator after it
            // OR be exactly the prefix followed by a slash
            if let Some(remaining) = path.strip_prefix(prefix) {
                // Must be followed by / or be exactly the prefix
                remaining.starts_with('/') || remaining.is_empty()
            } else {
                false
            }
        } else if pattern.contains('*') {
            // More complex glob patterns could be implemented here
            // For now, just handle the /* suffix case
            false
        } else {
            path == pattern
        }
    }

    /// Handle CSRF validation failure
    fn handle_csrf_failure(&self, ctx: &mut Context) -> Result<InboundAction> {
        // Set flash error message if session exists
        let _ = ctx.flash(
            &self.config.flash_error_key,
            self.config.error_message.as_str(),
        );

        // Redirect if configured, otherwise return error response
        if let Some(redirect_url) = &self.config.redirect_on_failure {
            ctx.redirect(redirect_url)?;
        } else {
            // Check if this is an API request (JSON response expected)
            let is_api_request = ctx
                .header("accept")
                .map(|accept| accept.contains("application/json"))
                .unwrap_or(false)
                || ctx
                    .header("content-type")
                    .map(|ct| ct.contains("application/json"))
                    .unwrap_or(false);

            if is_api_request {
                ctx.json(json!({
                    "error": "csrf_token_invalid",
                    "message": self.config.error_message
                }))?;
            } else {
                // Return HTML error page
                ctx.throw403(Some(&self.config.error_message))?;
            }
        }

        Ok(InboundAction::Stop)
    }
}

impl Default for CsrfMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InboundMiddleware for CsrfMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        // Skip if CSRF protection is disabled
        if !self.config.enabled {
            return Ok(InboundAction::Continue);
        }

        // Skip if HTTP method doesn't require protection
        if !self.requires_protection(&ctx.req.method) {
            return Ok(InboundAction::Continue);
        }

        // Skip if route is exempt
        if self.is_route_exempt(ctx.path()) {
            return Ok(InboundAction::Continue);
        }

        // Verify CSRF token (using default token ID)
        if !ctx.verify_csrf(None)? {
            return self.handle_csrf_failure(ctx);
        }

        Ok(InboundAction::Continue)
    }

    fn name(&self) -> &'static str {
        "csrf"
    }

    fn priority(&self) -> i32 {
        -25 // After auth but before regular routes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csrf_config_default() {
        let config = CsrfConfig::default();

        assert!(config.enabled);
        assert!(config.protected_methods.contains("POST"));
        assert!(config.protected_methods.contains("PUT"));
        assert!(config.protected_methods.contains("PATCH"));
        assert!(config.protected_methods.contains("DELETE"));
        assert!(!config.protected_methods.contains("GET"));
        assert_eq!(config.exempt_routes, vec!["/api/*"]);
    }

    #[test]
    fn test_csrf_config_builder() {
        let config = CsrfConfig::new()
            .exempt("/webhook/*")
            .exempt("/public/upload")
            .error_message("Custom error")
            .redirect_on_failure("/login")
            .protect_method("CUSTOM")
            .exempt_method("DELETE");

        assert!(config.exempt_routes.contains(&"/webhook/*".to_string()));
        assert!(config.exempt_routes.contains(&"/public/upload".to_string()));
        assert_eq!(config.error_message, "Custom error");
        assert_eq!(config.redirect_on_failure, Some("/login".to_string()));
        assert!(config.protected_methods.contains("CUSTOM"));
        assert!(!config.protected_methods.contains("DELETE"));
    }

    #[test]
    fn test_pattern_matching() {
        let middleware = CsrfMiddleware::new();

        // Test /* pattern
        assert!(middleware.matches_pattern("/api/users", "/api/*"));
        assert!(middleware.matches_pattern("/api/v1/posts", "/api/*"));
        assert!(!middleware.matches_pattern("/public/api", "/api/*"));

        // Test exact match
        assert!(middleware.matches_pattern("/webhook", "/webhook"));
        assert!(!middleware.matches_pattern("/webhooks", "/webhook"));
    }

    #[test]
    fn test_requires_protection() {
        let middleware = CsrfMiddleware::new();

        // Protected methods
        assert!(middleware.requires_protection("POST"));
        assert!(middleware.requires_protection("put")); // case insensitive
        assert!(middleware.requires_protection("PATCH"));
        assert!(middleware.requires_protection("DELETE"));

        // Safe methods
        assert!(!middleware.requires_protection("GET"));
        assert!(!middleware.requires_protection("HEAD"));
        assert!(!middleware.requires_protection("OPTIONS"));
        assert!(!middleware.requires_protection("TRACE"));
    }

    #[test]
    fn test_route_exemption() {
        let middleware = CsrfMiddleware::new();

        // Default /api/* exemption
        assert!(middleware.is_route_exempt("/api/users"));
        assert!(middleware.is_route_exempt("/api/v1/posts"));
        assert!(!middleware.is_route_exempt("/app/api"));
        assert!(!middleware.is_route_exempt("/users"));
    }

    #[test]
    fn test_custom_exemptions() {
        let config = CsrfConfig::new()
            .exempt("/webhook/*")
            .exempt("/public/upload");
        let middleware = CsrfMiddleware::with_config(config);

        // Custom exemptions
        assert!(middleware.is_route_exempt("/webhook/github"));
        assert!(middleware.is_route_exempt("/public/upload"));
        assert!(!middleware.is_route_exempt("/webhookx")); // Different from /webhook
        assert!(!middleware.is_route_exempt("/public/upload/file"));

        // Default exemption still works
        assert!(middleware.is_route_exempt("/api/test"));
    }
}
