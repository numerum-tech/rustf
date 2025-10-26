#[cfg(test)]
mod tests {
    use hyper::StatusCode;
    use rustf::prelude::*;
    use std::sync::Arc;

    #[test]
    fn test_json_preserves_headers() {
        let request = rustf::http::Request::new("GET", "/test", "HTTP/1.1");
        let views = Arc::new(rustf::views::ViewEngine::new());
        let mut ctx = Context::new(request, views);

        // Add headers simulating middleware
        ctx.add_header("X-Custom-Header", "test-value");
        ctx.add_header("X-Request-ID", "req-123");

        // Call json which should preserve headers
        ctx.json(serde_json::json!({"test": "data"})).unwrap();

        let response = ctx.get_response().unwrap();

        // Check headers are preserved
        assert!(response
            .headers
            .iter()
            .any(|(n, v)| n == "X-Custom-Header" && v == "test-value"));
        assert!(response
            .headers
            .iter()
            .any(|(n, v)| n == "X-Request-ID" && v == "req-123"));
        assert!(response
            .headers
            .iter()
            .any(|(n, _)| n.to_lowercase() == "content-type"));
    }

    #[test]
    fn test_error_preserves_headers() {
        let request = rustf::http::Request::new("GET", "/test", "HTTP/1.1");
        let views = Arc::new(rustf::views::ViewEngine::new());
        let mut ctx = Context::new(request, views);

        // Add headers
        ctx.add_header("X-Error-Context", "validation");
        ctx.add_header("X-Trace-ID", "trace-456");

        // Call error which should preserve headers
        ctx.throw404(Some("Not found")).unwrap();

        let response = ctx.get_response().unwrap();

        // Check headers are preserved
        assert!(response
            .headers
            .iter()
            .any(|(n, v)| n == "X-Error-Context" && v == "validation"));
        assert!(response
            .headers
            .iter()
            .any(|(n, v)| n == "X-Trace-ID" && v == "trace-456"));
        assert_eq!(response.status, StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_redirect_preserves_headers() {
        let request = rustf::http::Request::new("GET", "/test", "HTTP/1.1");
        let views = Arc::new(rustf::views::ViewEngine::new());
        let mut ctx = Context::new(request, views);

        // Add headers
        ctx.add_header("X-Session-ID", "sess-789");
        ctx.add_header("X-Redirect-Reason", "auth-required");

        // Call redirect which should preserve headers
        ctx.redirect("/login").unwrap();

        let response = ctx.get_response().unwrap();

        // Check headers are preserved
        assert!(response
            .headers
            .iter()
            .any(|(n, v)| n == "X-Session-ID" && v == "sess-789"));
        assert!(response
            .headers
            .iter()
            .any(|(n, v)| n == "X-Redirect-Reason" && v == "auth-required"));
        assert!(response
            .headers
            .iter()
            .any(|(n, v)| n == "Location" && v == "/login"));
        assert_eq!(response.status, StatusCode::FOUND);
    }

    #[test]
    fn test_text_preserves_headers() {
        let request = rustf::http::Request::new("GET", "/test", "HTTP/1.1");
        let views = Arc::new(rustf::views::ViewEngine::new());
        let mut ctx = Context::new(request, views);

        // Add headers
        ctx.add_header("X-Cache-Control", "no-cache");
        ctx.add_header("X-Processing-Time", "125ms");

        // Call text which should preserve headers
        ctx.text("Plain text response").unwrap();

        let response = ctx.get_response().unwrap();

        // Check headers are preserved
        assert!(response
            .headers
            .iter()
            .any(|(n, v)| n == "X-Cache-Control" && v == "no-cache"));
        assert!(response
            .headers
            .iter()
            .any(|(n, v)| n == "X-Processing-Time" && v == "125ms"));
        assert!(response
            .headers
            .iter()
            .any(|(n, v)| n.to_lowercase() == "content-type" && v.contains("text/plain")));
    }

    #[test]
    fn test_html_preserves_headers() {
        let request = rustf::http::Request::new("GET", "/test", "HTTP/1.1");
        let views = Arc::new(rustf::views::ViewEngine::new());
        let mut ctx = Context::new(request, views);

        // Add headers
        ctx.add_header("X-Frame-Options", "DENY");
        ctx.add_header("X-Content-Type-Options", "nosniff");

        // Call html which should preserve headers
        ctx.html("<h1>Test</h1>").unwrap();

        let response = ctx.get_response().unwrap();

        // Check security headers are preserved
        assert!(response
            .headers
            .iter()
            .any(|(n, v)| n == "X-Frame-Options" && v == "DENY"));
        assert!(response
            .headers
            .iter()
            .any(|(n, v)| n == "X-Content-Type-Options" && v == "nosniff"));
        assert!(response
            .headers
            .iter()
            .any(|(n, v)| n.to_lowercase() == "content-type" && v.contains("text/html")));
    }
}
