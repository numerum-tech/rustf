#[cfg(test)]
mod tests {
    use rustf::context::Context;
    use rustf::http::Request;
    use rustf::session::Session;
    use rustf::views::ViewEngine;
    use std::sync::Arc;

    #[test]
    fn test_context_flash_integration() {
        // Create a mock request
        let request = Request::new("GET", "/test", "1.1");

        // Create view engine
        let views = Arc::new(ViewEngine::new());

        // Create context
        let mut ctx = Context::new(request, views);

        // Create and set session
        let session = Arc::new(Session::new("test-session"));
        ctx.set_session(Some(session));

        // Test flash methods
        ctx.flash("message", "Hello World").unwrap();
        ctx.flash_success("Operation completed").unwrap();
        ctx.flash_error("Something failed").unwrap();

        // Get flash messages
        let msg = ctx.get_flash("message");
        assert_eq!(msg.as_ref().and_then(|v| v.as_str()), Some("Hello World"));

        // Get all flash
        let all_flash = ctx.get_all_flash();
        assert_eq!(
            all_flash.get("success").and_then(|v| v.as_str()),
            Some("Operation completed")
        );
        assert_eq!(
            all_flash.get("error").and_then(|v| v.as_str()),
            Some("Something failed")
        );

        // Verify they're consumed
        let all_flash_again = ctx.get_all_flash();
        assert!(all_flash_again.is_empty());
    }

    #[test]
    fn test_context_session_helpers() {
        let request = Request::new("GET", "/test", "1.1");
        let views = Arc::new(ViewEngine::new());
        let mut ctx = Context::new(request, views);

        // Create and set session
        let session = Arc::new(Session::new("test-session"));
        ctx.set_session(Some(session));

        // Test session helpers
        ctx.session_set("key1", "value1").unwrap();
        ctx.session_set("key2", 42).unwrap();

        let val1: Option<String> = ctx.session_get("key1");
        assert_eq!(val1, Some("value1".to_string()));

        let val2: Option<i32> = ctx.session_get("key2");
        assert_eq!(val2, Some(42));

        // Test remove
        let removed = ctx.session_remove("key1");
        assert!(removed.is_some());

        let val1_again: Option<String> = ctx.session_get("key1");
        assert_eq!(val1_again, None);
    }
}
