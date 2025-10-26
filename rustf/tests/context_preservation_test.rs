use async_trait::async_trait;
use rustf::middleware::{InboundAction, InboundMiddleware, MiddlewareRegistry};
use rustf::prelude::*;
use std::sync::Arc;
use tokio;

/// Test middleware that sets context values
struct TestMiddleware;

#[async_trait]
impl InboundMiddleware for TestMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> rustf::Result<InboundAction> {
        // Set values in context
        ctx.layout(""); // The original issue - setting layout to empty
        ctx.repository_set("middleware_key", "middleware_value");
        ctx.set("test_number", 42i32);

        Ok(InboundAction::Continue)
    }
}

/// Test handler that verifies context values
async fn test_handler(ctx: &mut Context) -> Result<()> {
    let mut results = vec![];

    // Check repository value set by middleware
    if let Some(value) = ctx.repository_get("middleware_key") {
        results.push(format!("✓ Repository value preserved: {:?}", value));
    } else {
        results.push("✗ Repository value NOT preserved!".to_string());
    }

    // Check typed value set by middleware
    if let Some(value) = ctx.get::<i32>("test_number") {
        results.push(format!("✓ Typed value preserved: {}", value));
    } else {
        results.push("✗ Typed value NOT preserved!".to_string());
    }

    ctx.json(json!({
        "test": "context_preservation",
        "results": results
    }))?;
    Ok(())
}

#[tokio::test]
async fn test_middleware_context_preservation() {
    // Create a minimal app for testing
    let mut registry = MiddlewareRegistry::new();
    registry.register_inbound("test_middleware", TestMiddleware);

    // Create context with minimal setup
    let request = Request::new("GET", "/test", "HTTP/1.1");

    // Create view engine (required for context)
    let views = Arc::new(rustf::views::ViewEngine::from_directory("views"));

    let mut ctx = Context::new(request, views);

    // Process through middleware
    let middleware = TestMiddleware;
    let result = middleware.process_request(&mut ctx).await;
    assert!(matches!(result, Ok(InboundAction::Continue)));

    // Now call handler with the same context
    test_handler(&mut ctx).await.unwrap();

    // Check response body contains success markers
    let response = ctx.res.as_ref().expect("Response should be set");
    let body_str = String::from_utf8_lossy(&response.body);

    println!("Response body: {}", body_str);

    // Verify that values were preserved
    assert!(body_str.contains("Repository value preserved"));
    assert!(body_str.contains("Typed value preserved"));
    assert!(!body_str.contains("NOT preserved"));
}
