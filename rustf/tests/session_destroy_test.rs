use rustf::middleware::builtin::SessionMiddleware;
use rustf::middleware::traits::{InboundMiddleware, OutboundMiddleware};
use rustf::prelude::*;
use rustf::session::manager::SessionConfig;
use rustf::views::ViewEngine;
use std::sync::Arc;

fn create_test_context(path: &str) -> Context {
    let view_engine = Arc::new(ViewEngine::new());
    let request = Request::new("GET", path, "HTTP/1.1");
    Context::new(request, view_engine)
}

#[tokio::test]
async fn session_destroy_emits_deletion_cookie() {
    let middleware = SessionMiddleware::new(SessionConfig::new());

    let mut ctx = create_test_context("/");
    let action = middleware.process_request(&mut ctx).await.unwrap();
    assert!(matches!(action, InboundAction::Capture));
    assert!(ctx.session_arc().is_some());

    ctx.session_destroy();
    middleware.process_response(&mut ctx).await.unwrap();

    let response = ctx.res.as_ref().unwrap();
    let destroy_cookie = response
        .headers
        .iter()
        .find(|(name, value)| name == "Set-Cookie" && value.contains("Max-Age=0"));

    assert!(destroy_cookie.is_some(), "expected destroy Set-Cookie header");
}
