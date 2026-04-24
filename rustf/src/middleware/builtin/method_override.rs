//! HTTP method override middleware.
//!
//! HTML forms only support GET and POST natively. The convention (used by
//! Rails, Express, Phoenix, etc.) is for a POST request to carry a
//! `_method=PUT` or `_method=DELETE` field (in form body or query string),
//! which is upgraded to the real method before routing. This middleware
//! does that rewrite.
//!
//! Only runs when the incoming request is a POST. The override must be
//! one of `PUT`, `PATCH`, `DELETE` — anything else is ignored.
//!
//! # Enable
//! ```rust,ignore
//! use rustf::middleware::builtin::MethodOverrideMiddleware;
//! registry.register_inbound("method_override", MethodOverrideMiddleware);
//! ```
//! Or via the builder helper:
//! ```rust,ignore
//! RustF::new().with_method_override()
//! ```

use crate::context::Context;
use crate::error::Result;
use crate::middleware::{InboundAction, InboundMiddleware};
use async_trait::async_trait;

#[derive(Clone, Default)]
pub struct MethodOverrideMiddleware;

impl MethodOverrideMiddleware {
    pub fn new() -> Self {
        Self
    }

    fn is_valid_override(method: &str) -> bool {
        matches!(
            method.to_ascii_uppercase().as_str(),
            "PUT" | "PATCH" | "DELETE"
        )
    }

    fn extract_override(ctx: &mut Context) -> Option<String> {
        // Query string wins over body (cheaper to check, and HTML forms
        // that want to override typically put it in the body).
        if let Some(v) = ctx.req.query.get("_method") {
            if Self::is_valid_override(v) {
                return Some(v.to_ascii_uppercase());
            }
        }

        // Body form — only parse for content-types that carry forms to
        // avoid fighting with JSON APIs that might coincidentally contain
        // a `_method` field.
        let ct = ctx
            .req
            .headers
            .get("content-type")
            .map(|s| s.as_str())
            .unwrap_or("");
        if !ct.contains("application/x-www-form-urlencoded") {
            return None;
        }

        if let Ok(form) = ctx.body_form() {
            if let Some(v) = form.get("_method") {
                if Self::is_valid_override(v) {
                    return Some(v.to_ascii_uppercase());
                }
            }
        }
        None
    }
}

#[async_trait]
impl InboundMiddleware for MethodOverrideMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        if !ctx.req.method.eq_ignore_ascii_case("POST") {
            return Ok(InboundAction::Continue);
        }
        if let Some(overridden) = Self::extract_override(ctx) {
            ctx.req.method = overridden;
        }
        Ok(InboundAction::Continue)
    }

    fn name(&self) -> &'static str {
        "method_override"
    }

    fn priority(&self) -> i32 {
        // Run early — before any router or auth middleware so they see the
        // real effective method.
        -500
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::Request;
    use crate::views::ViewEngine;
    use std::sync::Arc;

    fn ctx_with(method: &str, content_type: Option<&str>, body: &[u8]) -> Context {
        let mut req = Request::default();
        req.method = method.to_string();
        if let Some(ct) = content_type {
            req.headers.insert("content-type".to_string(), ct.to_string());
        }
        if !body.is_empty() {
            // Request doesn't expose body field; we stash via set_body (test-only).
            req.set_body(body.to_vec());
        }
        let views = Arc::new(ViewEngine::from_directory("views"));
        Context::new(req, views)
    }

    #[tokio::test]
    async fn rewrites_post_to_put_from_form_body() {
        let mut ctx = ctx_with(
            "POST",
            Some("application/x-www-form-urlencoded"),
            b"_method=PUT&name=foo",
        );
        MethodOverrideMiddleware.process_request(&mut ctx).await.unwrap();
        assert_eq!(ctx.req.method, "PUT");
    }

    #[tokio::test]
    async fn rewrites_post_to_delete_from_form_body() {
        let mut ctx = ctx_with(
            "POST",
            Some("application/x-www-form-urlencoded"),
            b"_method=DELETE",
        );
        MethodOverrideMiddleware.process_request(&mut ctx).await.unwrap();
        assert_eq!(ctx.req.method, "DELETE");
    }

    #[tokio::test]
    async fn leaves_post_alone_when_no_override_field() {
        let mut ctx = ctx_with(
            "POST",
            Some("application/x-www-form-urlencoded"),
            b"name=foo",
        );
        MethodOverrideMiddleware.process_request(&mut ctx).await.unwrap();
        assert_eq!(ctx.req.method, "POST");
    }

    #[tokio::test]
    async fn leaves_get_alone_entirely() {
        let mut ctx = ctx_with("GET", None, b"");
        MethodOverrideMiddleware.process_request(&mut ctx).await.unwrap();
        assert_eq!(ctx.req.method, "GET");
    }

    #[tokio::test]
    async fn ignores_invalid_override() {
        let mut ctx = ctx_with(
            "POST",
            Some("application/x-www-form-urlencoded"),
            b"_method=TRACE",
        );
        MethodOverrideMiddleware.process_request(&mut ctx).await.unwrap();
        assert_eq!(ctx.req.method, "POST");
    }

    #[tokio::test]
    async fn reads_from_query_string() {
        let mut req = Request::default();
        req.method = "POST".to_string();
        req.query.insert("_method".to_string(), "PATCH".to_string());
        let views = Arc::new(ViewEngine::from_directory("views"));
        let mut ctx = Context::new(req, views);
        MethodOverrideMiddleware.process_request(&mut ctx).await.unwrap();
        assert_eq!(ctx.req.method, "PATCH");
    }

    #[tokio::test]
    async fn ignores_body_for_non_form_content_types() {
        // A JSON body that happens to have {"_method": "DELETE"} must not
        // trigger the override.
        let mut ctx = ctx_with(
            "POST",
            Some("application/json"),
            br#"{"_method":"DELETE"}"#,
        );
        MethodOverrideMiddleware.process_request(&mut ctx).await.unwrap();
        assert_eq!(ctx.req.method, "POST");
    }
}
