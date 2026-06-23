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

    assert!(
        destroy_cookie.is_some(),
        "expected destroy Set-Cookie header"
    );
}

#[tokio::test]
async fn login_rotates_session_id_before_response_cookie_is_issued() {
    let mut config = SessionConfig::new();
    config.secure = false;
    let middleware = SessionMiddleware::new(config);

    let mut ctx = create_test_context("/");
    let action = middleware.process_request(&mut ctx).await.unwrap();
    assert!(matches!(action, InboundAction::Capture));

    let original_id = ctx
        .session_arc()
        .expect("session should exist")
        .id()
        .to_string();

    ctx.login(42).unwrap();
    middleware.process_response(&mut ctx).await.unwrap();

    let rotated_id = ctx
        .session_arc()
        .expect("session should still exist")
        .id()
        .to_string();
    assert_ne!(rotated_id, original_id, "login should rotate session id");

    let response = ctx.res.as_ref().unwrap();
    let session_cookie = response
        .headers
        .iter()
        .find(|(name, value)| name == "Set-Cookie" && !value.contains("Max-Age=0"))
        .map(|(_, value)| value.clone())
        .expect("expected session Set-Cookie header");
    assert!(session_cookie.contains(&rotated_id));
    assert!(!session_cookie.contains(&original_id));
}

#[tokio::test]
async fn session_destroy_on_session_object_emits_deletion_cookie() {
    let mut config = SessionConfig::new();
    config.secure = false;
    let middleware = SessionMiddleware::new(config);

    let mut ctx = create_test_context("/");
    let action = middleware.process_request(&mut ctx).await.unwrap();
    assert!(matches!(action, InboundAction::Capture));

    let session = ctx.session_arc().expect("session should exist");
    session.destroy();

    middleware.process_response(&mut ctx).await.unwrap();

    let response = ctx.res.as_ref().unwrap();
    let destroy_cookie = response
        .headers
        .iter()
        .find(|(name, value)| name == "Set-Cookie" && value.contains("Max-Age=0"));

    assert!(
        destroy_cookie.is_some(),
        "expected destroy Set-Cookie header"
    );
}
