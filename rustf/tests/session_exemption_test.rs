use rustf::middleware::builtin::SessionMiddleware;
use rustf::prelude::*;
use rustf::session::manager::SessionConfig;
use rustf::views::ViewEngine;
use std::sync::Arc;
use std::time::Duration;

fn create_test_context(path: &str) -> Context {
    let view_engine = Arc::new(ViewEngine::new());
    let request = Request::new("GET", path, "HTTP/1.1");
    Context::new(request, view_engine)
}

#[tokio::test]
async fn test_session_exemption_paths() {
    // Create session config with exemptions
    let mut config = SessionConfig::new();
    config.exempt_routes = vec![
        "/api/*".to_string(),
        "/webhooks/*".to_string(),
        "/health".to_string(),
    ];
    config.idle_timeout = Duration::from_secs(900);
    config.absolute_timeout = Duration::from_secs(3600);

    // Create middleware with the config
    let middleware = SessionMiddleware::new(config);

    // Test 1: Regular path should get session
    let mut ctx = create_test_context("/");
    let action = middleware.process_request(&mut ctx).await.unwrap();
    assert!(matches!(action, InboundAction::Capture));
    assert!(
        ctx.session_arc().is_some(),
        "Regular path should have session"
    );

    // Test 2: API path should be exempt
    let mut ctx = create_test_context("/api/users");
    let action = middleware.process_request(&mut ctx).await.unwrap();
    assert!(matches!(action, InboundAction::Continue));
    assert!(
        ctx.session_arc().is_none(),
        "API path should not have session"
    );

    // Test 3: Webhook path should be exempt
    let mut ctx = create_test_context("/webhooks/github");
    let action = middleware.process_request(&mut ctx).await.unwrap();
    assert!(matches!(action, InboundAction::Continue));
    assert!(
        ctx.session_arc().is_none(),
        "Webhook path should not have session"
    );

    // Test 4: Health check should be exempt
    let mut ctx = create_test_context("/health");
    let action = middleware.process_request(&mut ctx).await.unwrap();
    assert!(matches!(action, InboundAction::Continue));
    assert!(
        ctx.session_arc().is_none(),
        "Health path should not have session"
    );

    // Test 5: API root should be exempt
    let mut ctx = create_test_context("/api");
    let action = middleware.process_request(&mut ctx).await.unwrap();
    assert!(matches!(action, InboundAction::Continue));
    assert!(
        ctx.session_arc().is_none(),
        "API root should not have session"
    );

    // Test 6: Similar but non-matching path should get session
    let mut ctx = create_test_context("/apikeys");
    let action = middleware.process_request(&mut ctx).await.unwrap();
    assert!(matches!(action, InboundAction::Capture));
    assert!(
        ctx.session_arc().is_some(),
        "/apikeys should have session (doesn't match /api/*)"
    );
}

#[tokio::test]
async fn test_session_globally_disabled() {
    // Create session config with sessions disabled
    let mut config = SessionConfig::new();
    config.enabled = false;

    // Create middleware with the config
    let middleware = SessionMiddleware::new(config);

    // Test: All paths should skip session when globally disabled
    let mut ctx = create_test_context("/");
    let action = middleware.process_request(&mut ctx).await.unwrap();
    assert!(matches!(action, InboundAction::Continue));
    assert!(
        ctx.session_arc().is_none(),
        "Sessions disabled: no session should be created"
    );

    let mut ctx = create_test_context("/users");
    let action = middleware.process_request(&mut ctx).await.unwrap();
    assert!(matches!(action, InboundAction::Continue));
    assert!(
        ctx.session_arc().is_none(),
        "Sessions disabled: no session should be created"
    );
}

#[tokio::test]
async fn test_session_exemption_pattern_matching() {
    // Create session config with various patterns
    let mut config = SessionConfig::new();
    config.exempt_routes = vec!["/api/*".to_string(), "/static/*".to_string()];

    let middleware = SessionMiddleware::new(config);

    // Test edge cases for pattern matching

    // Should match
    assert_should_be_exempt(&middleware, "/api/v1/users").await;
    assert_should_be_exempt(&middleware, "/api/v2/posts/123").await;
    assert_should_be_exempt(&middleware, "/static/css/style.css").await;
    assert_should_be_exempt(&middleware, "/static/js/app.js").await;

    // Should NOT match
    assert_should_have_session(&middleware, "/apiv1").await; // No slash after api
    assert_should_have_session(&middleware, "/api_v1").await; // Underscore instead of slash
    assert_should_have_session(&middleware, "/static-files").await; // Hyphen instead of slash
    assert_should_have_session(&middleware, "/").await; // Root path
    assert_should_have_session(&middleware, "/users").await; // Regular path
}

async fn assert_should_be_exempt(middleware: &SessionMiddleware, path: &str) {
    let mut ctx = create_test_context(path);
    let action = middleware.process_request(&mut ctx).await.unwrap();
    assert!(
        matches!(action, InboundAction::Continue),
        "Path {} should be exempt from sessions",
        path
    );
    assert!(
        ctx.session_arc().is_none(),
        "Path {} should not have session",
        path
    );
}

async fn assert_should_have_session(middleware: &SessionMiddleware, path: &str) {
    let mut ctx = create_test_context(path);
    let action = middleware.process_request(&mut ctx).await.unwrap();
    assert!(
        matches!(action, InboundAction::Capture),
        "Path {} should not be exempt from sessions",
        path
    );
    assert!(
        ctx.session_arc().is_some(),
        "Path {} should have session",
        path
    );
}
