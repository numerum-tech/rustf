//! Content Security Policy (CSP) middleware for RustF
//!
//! This middleware provides CSP header management with nonce generation
//! for inline scripts and styles.

use crate::context::Context;
use crate::error::Result;
use crate::middleware::{InboundAction, InboundMiddleware, OutboundMiddleware};
use crate::security::headers::ContentSecurityPolicy;
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use rand::Rng;

/// Generate a random nonce for CSP
fn generate_nonce() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..16).map(|_| rng.gen()).collect();
    general_purpose::STANDARD.encode(&bytes)
}

/// CSP middleware configuration
#[derive(Clone)]
pub struct CspConfig {
    /// Base CSP policy
    pub policy: ContentSecurityPolicy,
    /// Whether to generate nonces for inline scripts
    pub use_nonces: bool,
    /// Report-only mode (doesn't block, only reports violations)
    pub report_only: bool,
    /// Report URI for CSP violations (deprecated - use report_to instead)
    pub report_uri: Option<String>,
    /// Report-To endpoint name (CSP Level 3)
    pub report_to: Option<String>,
}

impl Default for CspConfig {
    fn default() -> Self {
        Self {
            policy: ContentSecurityPolicy::default(),
            use_nonces: true,
            report_only: false,
            report_uri: None,
            report_to: None,
        }
    }
}

/// Content Security Policy middleware with nonce support
#[derive(Clone)]
pub struct CspMiddleware {
    config: CspConfig,
}

impl CspMiddleware {
    /// Create new CSP middleware with default policy
    pub fn new() -> Self {
        Self {
            config: CspConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: CspConfig) -> Self {
        Self { config }
    }

    /// Create with custom CSP policy
    pub fn with_policy(policy: ContentSecurityPolicy) -> Self {
        Self {
            config: CspConfig {
                policy,
                ..Default::default()
            },
        }
    }

    /// Create permissive CSP (good for development)
    pub fn permissive() -> Self {
        Self {
            config: CspConfig {
                policy: ContentSecurityPolicy::new().allow_inline_scripts(),
                use_nonces: false,
                report_only: true,
                report_uri: None,
                report_to: None,
            },
        }
    }

    /// Create strict CSP (good for production)
    pub fn strict() -> Self {
        Self {
            config: CspConfig {
                policy: ContentSecurityPolicy::strict(),
                use_nonces: true,
                report_only: false,
                report_uri: None,
                report_to: None,
            },
        }
    }

    /// Set report-only mode
    pub fn report_only(mut self, report_only: bool) -> Self {
        self.config.report_only = report_only;
        self
    }

    /// Set report URI (deprecated - use with_report_to instead)
    pub fn with_report_uri(mut self, uri: &str) -> Self {
        self.config.report_uri = Some(uri.to_string());
        self
    }

    /// Set report-to endpoint name (CSP Level 3)
    pub fn with_report_to(mut self, endpoint: &str) -> Self {
        self.config.report_to = Some(endpoint.to_string());
        self
    }
}

impl Default for CspMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InboundMiddleware for CspMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        if self.config.use_nonces {
            // Generate nonces for this request
            let script_nonce = generate_nonce();
            let style_nonce = generate_nonce();

            // Store nonces in context for use in templates
            let _ = ctx.set("csp_script_nonce", script_nonce.clone());
            let _ = ctx.set("csp_style_nonce", style_nonce.clone());

            // Store for outbound phase
            let _ = ctx.set("csp_nonces", (script_nonce, style_nonce));
        }

        // We need to process the response to add CSP header
        Ok(InboundAction::Capture)
    }

    fn name(&self) -> &'static str {
        "csp"
    }

    fn priority(&self) -> i32 {
        -700 // Run after input validation
    }
}

#[async_trait]
impl OutboundMiddleware for CspMiddleware {
    async fn process_response(&self, ctx: &mut Context) -> Result<()> {
        let mut policy = self.config.policy.clone();

        // Add nonces if generated
        if self.config.use_nonces {
            if let Some((script_nonce, style_nonce)) = ctx.get::<(String, String)>("csp_nonces") {
                // Add nonces to the policy - only append if directives exist
                // Don't auto-add 'self' to respect user's custom policy
                if !policy.script_src.is_empty() {
                    policy.script_src.push(format!("'nonce-{}'", script_nonce));
                }

                if !policy.style_src.is_empty() {
                    policy.style_src.push(format!("'nonce-{}'", style_nonce));
                }
            }
        }

        // Build CSP header value with reporting directives if configured
        let mut csp_value = policy.to_header_value();

        // Add report-to (modern CSP Level 3)
        if let Some(ref endpoint) = self.config.report_to {
            csp_value.push_str(&format!("; report-to {}", endpoint));
        }

        // Add report-uri for backward compatibility
        if let Some(ref uri) = self.config.report_uri {
            csp_value.push_str(&format!("; report-uri {}", uri));
        }

        // Add appropriate header
        let header_name = if self.config.report_only {
            "Content-Security-Policy-Report-Only"
        } else {
            "Content-Security-Policy"
        };

        if let Some(response) = ctx.res.as_mut() {
            response.headers.push((header_name.to_string(), csp_value));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{Request, Response};
    use crate::views::ViewEngine;
    use std::sync::Arc;

    #[test]
    fn test_csp_middleware_creation() {
        let middleware = CspMiddleware::new();
        assert!(middleware.config.use_nonces);
        assert!(!middleware.config.report_only);

        let permissive = CspMiddleware::permissive();
        assert!(!permissive.config.use_nonces);
        assert!(permissive.config.report_only);

        let strict = CspMiddleware::strict();
        assert!(strict.config.use_nonces);
        assert!(!strict.config.report_only);
    }

    #[test]
    fn test_nonce_generation() {
        let nonce1 = generate_nonce();
        let nonce2 = generate_nonce();

        // Nonces should be unique
        assert_ne!(nonce1, nonce2);

        // Nonces should be base64 encoded
        assert!(nonce1
            .chars()
            .all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '='));
    }

    #[tokio::test]
    async fn test_csp_with_nonces() {
        let middleware = CspMiddleware::new();
        let request = Request::default();
        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);

        // Process request should generate nonces
        let action = middleware.process_request(&mut ctx).await.unwrap();
        assert!(matches!(action, InboundAction::Capture));

        // Check nonces were stored
        assert!(ctx.get::<String>("csp_script_nonce").is_some());
        assert!(ctx.get::<String>("csp_style_nonce").is_some());

        // Process response should add CSP header (Context already has response)
        middleware.process_response(&mut ctx).await.unwrap();

        // Check that CSP header was added
        if let Some(response) = &ctx.res {
            let has_csp = response.headers.iter().any(|(k, _)| {
                k == "Content-Security-Policy" || k == "Content-Security-Policy-Report-Only"
            });
            assert!(has_csp);
        } else {
            panic!("Expected response in context");
        }
    }

    #[tokio::test]
    async fn test_csp_report_only_mode() {
        let middleware = CspMiddleware::new().report_only(true);
        let request = Request::default();
        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);

        middleware.process_request(&mut ctx).await.unwrap();
        middleware.process_response(&mut ctx).await.unwrap();

        // Verify report-only header is used
        if let Some(response) = &ctx.res {
            let has_report_only = response
                .headers
                .iter()
                .any(|(k, _)| k == "Content-Security-Policy-Report-Only");
            assert!(has_report_only);

            let has_enforcing = response
                .headers
                .iter()
                .any(|(k, _)| k == "Content-Security-Policy");
            assert!(!has_enforcing);
        } else {
            panic!("Expected response in context");
        }
    }

    #[tokio::test]
    async fn test_csp_with_report_uri() {
        let middleware = CspMiddleware::new().with_report_uri("/csp-violations");
        let request = Request::default();
        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);

        middleware.process_request(&mut ctx).await.unwrap();
        middleware.process_response(&mut ctx).await.unwrap();

        // Verify report-uri is in header
        if let Some(response) = &ctx.res {
            let csp_header = response
                .headers
                .iter()
                .find(|(k, _)| k == "Content-Security-Policy")
                .map(|(_, v)| v);

            assert!(csp_header.is_some());
            assert!(csp_header.unwrap().contains("report-uri /csp-violations"));
        } else {
            panic!("Expected response in context");
        }
    }

    #[tokio::test]
    async fn test_csp_with_report_to() {
        let middleware = CspMiddleware::new().with_report_to("csp-endpoint");
        let request = Request::default();
        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);

        middleware.process_request(&mut ctx).await.unwrap();
        middleware.process_response(&mut ctx).await.unwrap();

        // Verify report-to is in header
        if let Some(response) = &ctx.res {
            let csp_header = response
                .headers
                .iter()
                .find(|(k, _)| k == "Content-Security-Policy")
                .map(|(_, v)| v);

            assert!(csp_header.is_some());
            assert!(csp_header.unwrap().contains("report-to csp-endpoint"));
        } else {
            panic!("Expected response in context");
        }
    }

    #[tokio::test]
    async fn test_csp_without_nonces() {
        let middleware = CspMiddleware::permissive();
        let request = Request::default();
        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);

        middleware.process_request(&mut ctx).await.unwrap();

        // Nonces should not be generated
        assert!(ctx.get::<String>("csp_script_nonce").is_none());
        assert!(ctx.get::<String>("csp_style_nonce").is_none());

        middleware.process_response(&mut ctx).await.unwrap();

        // CSP header should still be added
        if let Some(response) = &ctx.res {
            let has_csp = response
                .headers
                .iter()
                .any(|(k, _)| k == "Content-Security-Policy-Report-Only");
            assert!(has_csp);
        } else {
            panic!("Expected response in context");
        }
    }

    #[tokio::test]
    async fn test_csp_nonce_format() {
        let middleware = CspMiddleware::strict();
        let request = Request::default();
        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);

        middleware.process_request(&mut ctx).await.unwrap();
        middleware.process_response(&mut ctx).await.unwrap();

        // Verify nonce format in CSP header
        if let Some(response) = &ctx.res {
            let csp_header = response
                .headers
                .iter()
                .find(|(k, _)| k == "Content-Security-Policy")
                .map(|(_, v)| v);

            assert!(csp_header.is_some());
            let header_value = csp_header.unwrap();

            // Check for nonce directives
            assert!(header_value.contains("'nonce-"));
        } else {
            panic!("Expected response in context");
        }
    }

    #[tokio::test]
    async fn test_csp_respects_custom_policy() {
        // Create custom policy with script-src containing only CDN (no 'self')
        let custom_policy = ContentSecurityPolicy {
            default_src: vec![],
            script_src: vec!["https://trusted.cdn.com".to_string()],
            style_src: vec!["https://trusted.cdn.com".to_string()],
            img_src: vec![],
            font_src: vec![],
            connect_src: vec![],
            frame_src: vec![],
            object_src: vec![],
            media_src: vec![],
            child_src: vec![],
            worker_src: vec![],
            frame_ancestors: vec![],
            base_uri: vec![],
            form_action: vec![],
            upgrade_insecure_requests: false,
            block_all_mixed_content: false,
        };

        let middleware = CspMiddleware::with_policy(custom_policy);

        let request = Request::default();
        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(request, views);

        middleware.process_request(&mut ctx).await.unwrap();
        middleware.process_response(&mut ctx).await.unwrap();

        // Verify nonce was appended but no 'self' was auto-added
        if let Some(response) = &ctx.res {
            let csp_header = response
                .headers
                .iter()
                .find(|(k, _)| k == "Content-Security-Policy")
                .map(|(_, v)| v);

            assert!(csp_header.is_some());
            let header_value = csp_header.unwrap();

            // Should contain nonce since script-src exists
            assert!(header_value.contains("'nonce-"));
            // Should contain the trusted CDN
            assert!(header_value.contains("https://trusted.cdn.com"));
            // Should NOT have auto-added 'self' (proves our fix works)
            assert!(!header_value.contains("'self'"));
        } else {
            panic!("Expected response in context");
        }
    }
}
