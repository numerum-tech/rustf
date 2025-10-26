//! Request logging middleware for RustF
//!
//! This middleware logs incoming requests with timing information and response status.
//! It demonstrates proper dual-phase middleware implementation.

use crate::context::Context;
use crate::error::Result;
use crate::middleware::{InboundAction, InboundMiddleware, OutboundMiddleware};
use async_trait::async_trait;
use std::time::Instant;

/// HTTP request logging middleware
///
/// Logs all incoming requests with method, path, response status, and timing.
/// This middleware always continues the chain (never stops execution).
#[derive(Clone)]
pub struct LoggingMiddleware {
    pub name: String,
}

impl LoggingMiddleware {
    /// Create a new logging middleware
    pub fn new() -> Self {
        Self {
            name: "request_logger".to_string(),
        }
    }

    /// Create a logging middleware with a custom name
    pub fn with_name(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InboundMiddleware for LoggingMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        let start_time = Instant::now();
        let method = ctx.req.method.clone();
        let path = ctx.req.uri.clone();
        let ip = ctx.ip();

        log::info!("→ {} {} from {}", method, path, ip);

        // Store timing information for outbound phase
        let _ = ctx.set("logging_start_time", start_time);
        let _ = ctx.set("logging_method", method);
        let _ = ctx.set("logging_path", path);

        // We want to process the response to log timing
        Ok(InboundAction::Capture)
    }

    fn name(&self) -> &'static str {
        "logging"
    }

    fn priority(&self) -> i32 {
        -1000 // Very high priority (runs first)
    }
}

#[async_trait]
impl OutboundMiddleware for LoggingMiddleware {
    async fn process_response(&self, ctx: &mut Context) -> Result<()> {
        if let Some(start_time) = ctx.get::<Instant>("logging_start_time") {
            let elapsed = start_time.elapsed();
            let method = ctx
                .get::<String>("logging_method").cloned()
                .unwrap_or_else(String::new);
            let path = ctx
                .get::<String>("logging_path").cloned()
                .unwrap_or_else(String::new);

            let status = ctx.res.as_ref().map(|r| r.status.as_u16()).unwrap_or(500);

            log::info!("← {} {} {} in {:?}", method, path, status, elapsed);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::http::{Request, Response};
    use crate::models::ModelRegistry;
    use crate::session::{Session, SessionStore};
    use crate::views::ViewEngine;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_logging_middleware() {
        let middleware = LoggingMiddleware::new();

        // Create test context
        let mut request = Request::default();
        request.method = "GET".to_string();
        request.uri = "/test".to_string();

        let session_store = SessionStore::new();
        let session = session_store.get_or_create("test").await.unwrap();
        let views = Arc::new(ViewEngine::from_directory("views"));
        let config = Arc::new(AppConfig::default());

        let mut ctx = Context::new(request, views);

        // Test inbound processing
        let action = middleware.process_request(&mut ctx).await.unwrap();
        assert!(matches!(action, InboundAction::Capture));

        // Verify context has timing data
        assert!(ctx.get::<Instant>("logging_start_time").is_some());
        assert!(ctx.get::<String>("logging_method").is_some());
        assert!(ctx.get::<String>("logging_path").is_some());

        // Test outbound processing (Context already has response)
        middleware.process_response(&mut ctx).await.unwrap();
    }
}
