//! Security headers management for RustF framework
//!
//! This module provides comprehensive security headers to protect against
//! various web vulnerabilities including XSS, clickjacking, and content sniffing.

use crate::http::Response;
use std::collections::HashMap;

/// Content Security Policy configuration
#[derive(Clone, Debug)]
pub struct ContentSecurityPolicy {
    pub default_src: Vec<String>,
    pub script_src: Vec<String>,
    pub style_src: Vec<String>,
    pub img_src: Vec<String>,
    pub connect_src: Vec<String>,
    pub font_src: Vec<String>,
    pub object_src: Vec<String>,
    pub media_src: Vec<String>,
    pub frame_src: Vec<String>,
    pub child_src: Vec<String>,
    pub worker_src: Vec<String>,
    pub frame_ancestors: Vec<String>,
    pub base_uri: Vec<String>,
    pub form_action: Vec<String>,
    pub upgrade_insecure_requests: bool,
    pub block_all_mixed_content: bool,
}

impl Default for ContentSecurityPolicy {
    fn default() -> Self {
        Self {
            default_src: vec!["'self'".to_string()],
            script_src: vec!["'self'".to_string()],
            style_src: vec!["'self'".to_string(), "'unsafe-inline'".to_string()],
            img_src: vec!["'self'".to_string(), "data:".to_string()],
            connect_src: vec!["'self'".to_string()],
            font_src: vec!["'self'".to_string()],
            object_src: vec!["'none'".to_string()],
            media_src: vec!["'self'".to_string()],
            frame_src: vec!["'none'".to_string()],
            child_src: vec!["'self'".to_string()],
            worker_src: vec!["'self'".to_string()],
            frame_ancestors: vec!["'none'".to_string()],
            base_uri: vec!["'self'".to_string()],
            form_action: vec!["'self'".to_string()],
            upgrade_insecure_requests: true,
            block_all_mixed_content: true,
        }
    }
}

impl ContentSecurityPolicy {
    /// Create a new CSP with default secure settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a strict CSP for maximum security
    pub fn strict() -> Self {
        Self {
            default_src: vec!["'none'".to_string()],
            script_src: vec!["'self'".to_string()],
            style_src: vec!["'self'".to_string()],
            img_src: vec!["'self'".to_string()],
            connect_src: vec!["'self'".to_string()],
            font_src: vec!["'self'".to_string()],
            object_src: vec!["'none'".to_string()],
            media_src: vec!["'none'".to_string()],
            frame_src: vec!["'none'".to_string()],
            child_src: vec!["'none'".to_string()],
            worker_src: vec!["'self'".to_string()],
            frame_ancestors: vec!["'none'".to_string()],
            base_uri: vec!["'none'".to_string()],
            form_action: vec!["'self'".to_string()],
            upgrade_insecure_requests: true,
            block_all_mixed_content: true,
        }
    }

    /// Add a source to script-src
    pub fn allow_script_src(mut self, src: &str) -> Self {
        self.script_src.push(src.to_string());
        self
    }

    /// Add a source to style-src
    pub fn allow_style_src(mut self, src: &str) -> Self {
        self.style_src.push(src.to_string());
        self
    }

    /// Add a source to img-src
    pub fn allow_img_src(mut self, src: &str) -> Self {
        self.img_src.push(src.to_string());
        self
    }

    /// Allow inline scripts (not recommended)
    pub fn allow_inline_scripts(mut self) -> Self {
        if !self.script_src.contains(&"'unsafe-inline'".to_string()) {
            self.script_src.push("'unsafe-inline'".to_string());
        }
        self
    }

    /// Allow eval() in scripts (not recommended)
    pub fn allow_eval(mut self) -> Self {
        if !self.script_src.contains(&"'unsafe-eval'".to_string()) {
            self.script_src.push("'unsafe-eval'".to_string());
        }
        self
    }

    /// Convert to CSP header value
    pub fn to_header_value(&self) -> String {
        let mut directives = Vec::new();

        if !self.default_src.is_empty() {
            directives.push(format!("default-src {}", self.default_src.join(" ")));
        }
        if !self.script_src.is_empty() {
            directives.push(format!("script-src {}", self.script_src.join(" ")));
        }
        if !self.style_src.is_empty() {
            directives.push(format!("style-src {}", self.style_src.join(" ")));
        }
        if !self.img_src.is_empty() {
            directives.push(format!("img-src {}", self.img_src.join(" ")));
        }
        if !self.connect_src.is_empty() {
            directives.push(format!("connect-src {}", self.connect_src.join(" ")));
        }
        if !self.font_src.is_empty() {
            directives.push(format!("font-src {}", self.font_src.join(" ")));
        }
        if !self.object_src.is_empty() {
            directives.push(format!("object-src {}", self.object_src.join(" ")));
        }
        if !self.media_src.is_empty() {
            directives.push(format!("media-src {}", self.media_src.join(" ")));
        }
        if !self.frame_src.is_empty() {
            directives.push(format!("frame-src {}", self.frame_src.join(" ")));
        }
        if !self.child_src.is_empty() {
            directives.push(format!("child-src {}", self.child_src.join(" ")));
        }
        if !self.worker_src.is_empty() {
            directives.push(format!("worker-src {}", self.worker_src.join(" ")));
        }
        if !self.frame_ancestors.is_empty() {
            directives.push(format!(
                "frame-ancestors {}",
                self.frame_ancestors.join(" ")
            ));
        }
        if !self.base_uri.is_empty() {
            directives.push(format!("base-uri {}", self.base_uri.join(" ")));
        }
        if !self.form_action.is_empty() {
            directives.push(format!("form-action {}", self.form_action.join(" ")));
        }

        if self.upgrade_insecure_requests {
            directives.push("upgrade-insecure-requests".to_string());
        }
        if self.block_all_mixed_content {
            directives.push("block-all-mixed-content".to_string());
        }

        directives.join("; ")
    }
}

/// Security headers configuration
#[derive(Clone, Debug)]
pub struct SecurityHeaders {
    pub csp: Option<ContentSecurityPolicy>,
    pub hsts_max_age: Option<u32>,
    pub hsts_include_subdomains: bool,
    pub hsts_preload: bool,
    pub x_frame_options: Option<String>,
    pub x_content_type_options: bool,
    pub x_xss_protection: Option<String>,
    pub referrer_policy: Option<String>,
    pub permissions_policy: Option<String>,
    pub cross_origin_embedder_policy: Option<String>,
    pub cross_origin_opener_policy: Option<String>,
    pub cross_origin_resource_policy: Option<String>,
    pub custom_headers: HashMap<String, String>,
}

impl Default for SecurityHeaders {
    fn default() -> Self {
        Self {
            csp: Some(ContentSecurityPolicy::default()),
            hsts_max_age: Some(31536000), // 1 year
            hsts_include_subdomains: true,
            hsts_preload: true,
            x_frame_options: Some("DENY".to_string()),
            x_content_type_options: true,
            x_xss_protection: Some("1; mode=block".to_string()),
            referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
            permissions_policy: Some("camera=(), microphone=(), geolocation=()".to_string()),
            cross_origin_embedder_policy: Some("require-corp".to_string()),
            cross_origin_opener_policy: Some("same-origin".to_string()),
            cross_origin_resource_policy: Some("same-origin".to_string()),
            custom_headers: HashMap::new(),
        }
    }
}

impl SecurityHeaders {
    /// Create new security headers with secure defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Create security headers for development (less strict)
    pub fn development() -> Self {
        Self {
            csp: Some(ContentSecurityPolicy::new().allow_inline_scripts()),
            hsts_max_age: None, // Don't use HSTS in development
            hsts_include_subdomains: false,
            hsts_preload: false,
            x_frame_options: Some("SAMEORIGIN".to_string()),
            x_content_type_options: true,
            x_xss_protection: Some("1; mode=block".to_string()),
            referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
            permissions_policy: None,
            cross_origin_embedder_policy: None,
            cross_origin_opener_policy: None,
            cross_origin_resource_policy: None,
            custom_headers: HashMap::new(),
        }
    }

    /// Create strict security headers for production
    pub fn strict() -> Self {
        Self {
            csp: Some(ContentSecurityPolicy::strict()),
            hsts_max_age: Some(63072000), // 2 years
            hsts_include_subdomains: true,
            hsts_preload: true,
            x_frame_options: Some("DENY".to_string()),
            x_content_type_options: true,
            x_xss_protection: Some("1; mode=block".to_string()),
            referrer_policy: Some("no-referrer".to_string()),
            permissions_policy: Some(
                "camera=(), microphone=(), geolocation=(), payment=(), usb=()".to_string(),
            ),
            cross_origin_embedder_policy: Some("require-corp".to_string()),
            cross_origin_opener_policy: Some("same-origin".to_string()),
            cross_origin_resource_policy: Some("same-origin".to_string()),
            custom_headers: HashMap::new(),
        }
    }

    /// Set Content Security Policy
    pub fn csp(mut self, csp: ContentSecurityPolicy) -> Self {
        self.csp = Some(csp);
        self
    }

    /// Disable Content Security Policy
    pub fn no_csp(mut self) -> Self {
        self.csp = None;
        self
    }

    /// Set HSTS max age
    pub fn hsts(mut self, max_age: u32, include_subdomains: bool, preload: bool) -> Self {
        self.hsts_max_age = Some(max_age);
        self.hsts_include_subdomains = include_subdomains;
        self.hsts_preload = preload;
        self
    }

    /// Disable HSTS
    pub fn no_hsts(mut self) -> Self {
        self.hsts_max_age = None;
        self
    }

    /// Set X-Frame-Options
    pub fn x_frame_options(mut self, value: &str) -> Self {
        self.x_frame_options = Some(value.to_string());
        self
    }

    /// Add custom header
    pub fn custom_header(mut self, name: &str, value: &str) -> Self {
        self.custom_headers
            .insert(name.to_string(), value.to_string());
        self
    }

    /// Apply security headers to a response
    pub fn apply_to_response(&self, mut response: Response) -> Response {
        // Content Security Policy
        if let Some(csp) = &self.csp {
            response = response.with_header("Content-Security-Policy", &csp.to_header_value());
        }

        // HTTP Strict Transport Security
        if let Some(max_age) = self.hsts_max_age {
            let mut hsts_value = format!("max-age={}", max_age);
            if self.hsts_include_subdomains {
                hsts_value.push_str("; includeSubDomains");
            }
            if self.hsts_preload {
                hsts_value.push_str("; preload");
            }
            response = response.with_header("Strict-Transport-Security", &hsts_value);
        }

        // X-Frame-Options
        if let Some(xfo) = &self.x_frame_options {
            response = response.with_header("X-Frame-Options", xfo);
        }

        // X-Content-Type-Options
        if self.x_content_type_options {
            response = response.with_header("X-Content-Type-Options", "nosniff");
        }

        // X-XSS-Protection
        if let Some(xss) = &self.x_xss_protection {
            response = response.with_header("X-XSS-Protection", xss);
        }

        // Referrer Policy
        if let Some(referrer) = &self.referrer_policy {
            response = response.with_header("Referrer-Policy", referrer);
        }

        // Permissions Policy
        if let Some(permissions) = &self.permissions_policy {
            response = response.with_header("Permissions-Policy", permissions);
        }

        // Cross-Origin Embedder Policy
        if let Some(coep) = &self.cross_origin_embedder_policy {
            response = response.with_header("Cross-Origin-Embedder-Policy", coep);
        }

        // Cross-Origin Opener Policy
        if let Some(coop) = &self.cross_origin_opener_policy {
            response = response.with_header("Cross-Origin-Opener-Policy", coop);
        }

        // Cross-Origin Resource Policy
        if let Some(corp) = &self.cross_origin_resource_policy {
            response = response.with_header("Cross-Origin-Resource-Policy", corp);
        }

        // Custom headers
        for (name, value) in &self.custom_headers {
            response = response.with_header(name, value);
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csp_default() {
        let csp = ContentSecurityPolicy::default();
        let header_value = csp.to_header_value();

        assert!(header_value.contains("default-src 'self'"));
        assert!(header_value.contains("script-src 'self'"));
        assert!(header_value.contains("object-src 'none'"));
        assert!(header_value.contains("upgrade-insecure-requests"));
    }

    #[test]
    fn test_csp_strict() {
        let csp = ContentSecurityPolicy::strict();
        let header_value = csp.to_header_value();

        assert!(header_value.contains("default-src 'none'"));
        assert!(header_value.contains("script-src 'self'"));
        assert!(header_value.contains("frame-ancestors 'none'"));
    }

    #[test]
    fn test_csp_customization() {
        let csp = ContentSecurityPolicy::new()
            .allow_script_src("https://cdn.example.com")
            .allow_style_src("https://fonts.googleapis.com")
            .allow_inline_scripts();

        let header_value = csp.to_header_value();

        assert!(header_value.contains("https://cdn.example.com"));
        assert!(header_value.contains("https://fonts.googleapis.com"));
        assert!(header_value.contains("'unsafe-inline'"));
    }

    #[test]
    fn test_security_headers_default() {
        let headers = SecurityHeaders::default();
        let response = Response::ok();
        let response_with_headers = headers.apply_to_response(response);

        // Check that headers are applied (would need to examine the response in a real implementation)
        // This is a placeholder test
        assert!(true);
    }

    #[test]
    fn test_security_headers_development() {
        let headers = SecurityHeaders::development();
        assert!(headers.hsts_max_age.is_none());
        assert!(headers.csp.is_some());
    }

    #[test]
    fn test_security_headers_strict() {
        let headers = SecurityHeaders::strict();
        assert_eq!(headers.hsts_max_age, Some(63072000));
        assert_eq!(headers.x_frame_options, Some("DENY".to_string()));
        assert_eq!(headers.referrer_policy, Some("no-referrer".to_string()));
    }

    #[test]
    fn test_custom_headers() {
        let headers = SecurityHeaders::new()
            .custom_header("X-Custom-Header", "custom-value")
            .custom_header("X-API-Version", "v1.0");

        assert_eq!(
            headers.custom_headers.get("X-Custom-Header"),
            Some(&"custom-value".to_string())
        );
        assert_eq!(
            headers.custom_headers.get("X-API-Version"),
            Some(&"v1.0".to_string())
        );
    }
}
