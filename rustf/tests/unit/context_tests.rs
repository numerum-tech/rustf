#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::session::SessionStore;
    use crate::views::ViewEngine;
    use crate::http::Request;
    use std::sync::Arc;
    use serde_json::json;

    async fn create_test_context() -> Context {
        let mut request = Request::default();
        request.method = "GET".to_string();
        request.uri = "/test".to_string();
        
        let session_store = SessionStore::new();
        let session = session_store.get_or_create("test_session").await.unwrap();
        let views = Arc::new(ViewEngine::from_directory("views"));
        
        Context::new(request, session, views)
    }

    #[tokio::test]
    async fn test_generic_flash_with_string() {
        let ctx = create_test_context().await;
        
        // Test setting a custom flash message with string
        let result = ctx.flash("warning_msg", "This is a warning");
        assert!(result.is_ok());
        
        // Verify it was set in the session
        let flash_value = ctx.session.flash_get("warning_msg");
        assert!(flash_value.is_some());
        if let Some(serde_json::Value::String(msg)) = flash_value {
            assert_eq!(msg, "This is a warning");
        } else {
            panic!("Expected string flash message");
        }
    }

    #[tokio::test]
    async fn test_generic_flash_with_number() {
        let ctx = create_test_context().await;
        
        // Test setting a flash message with number
        let result = ctx.flash("user_level", 42);
        assert!(result.is_ok());
        
        // Verify it was set in the session
        let flash_value = ctx.session.flash_get("user_level");
        assert!(flash_value.is_some());
        if let Some(serde_json::Value::Number(num)) = flash_value {
            assert_eq!(num.as_i64(), Some(42));
        } else {
            panic!("Expected number flash message");
        }
    }

    #[tokio::test]
    async fn test_generic_flash_with_json_object() {
        let ctx = create_test_context().await;
        
        // Test setting a flash message with JSON object
        let data = json!({
            "count": 10,
            "message": "Multiple items"
        });
        let result = ctx.flash("notification_data", data);
        assert!(result.is_ok());
        
        // Verify it was set in the session
        let flash_value = ctx.session.flash_get("notification_data");
        assert!(flash_value.is_some());
        if let Some(serde_json::Value::Object(map)) = flash_value {
            assert_eq!(map.get("count"), Some(&json!(10)));
            assert_eq!(map.get("message"), Some(&json!("Multiple items")));
        } else {
            panic!("Expected object flash message");
        }
    }

    #[tokio::test]
    async fn test_generic_flash_with_array() {
        let ctx = create_test_context().await;
        
        // Test setting a flash message with array
        let items = vec!["item1", "item2", "item3"];
        let result = ctx.flash("items", items);
        assert!(result.is_ok());
        
        // Verify it was set in the session
        let flash_value = ctx.session.flash_get("items");
        assert!(flash_value.is_some());
        if let Some(serde_json::Value::Array(arr)) = flash_value {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], json!("item1"));
            assert_eq!(arr[1], json!("item2"));
            assert_eq!(arr[2], json!("item3"));
        } else {
            panic!("Expected array flash message");
        }
    }

    #[tokio::test]
    async fn test_convenience_flash_methods_still_work() {
        let ctx = create_test_context().await;
        
        // Test that convenience methods still work
        ctx.flash_error("Error occurred");
        ctx.flash_info("Information");
        ctx.flash_success("Success!");
        
        // Verify they were set
        assert!(ctx.session.flash_get::<Value>("error_msg").is_some());
        assert!(ctx.session.flash_get::<Value>("info_msg").is_some());
        assert!(ctx.session.flash_get::<Value>("success_msg").is_some());
    }

    #[tokio::test]
    async fn test_flash_messages_available_in_view_data() {
        let ctx = create_test_context().await;
        
        // Set various flash messages
        ctx.flash("custom_key", "custom_value").unwrap();
        ctx.flash_error("An error");
        ctx.flash_success("Great!");
        
        // Note: We can't easily test the view() method here without mocking,
        // but we can verify the flash messages are in the session
        let all_flash = ctx.session.flash_get_all();
        
        assert_eq!(all_flash.len(), 3);
        assert_eq!(all_flash.get("custom_key"), Some(&json!("custom_value")));
        assert_eq!(all_flash.get("error_msg"), Some(&json!("An error")));
        assert_eq!(all_flash.get("success_msg"), Some(&json!("Great!")));
    }

    #[tokio::test]
    async fn test_flash_clear_all() {
        let ctx = create_test_context().await;
        
        // Set multiple flash messages
        ctx.flash("msg1", "value1").unwrap();
        ctx.flash("msg2", "value2").unwrap();
        ctx.flash_error("Error");
        ctx.flash_success("Success");
        
        // Clear all flash messages
        ctx.flash_clear();
        
        // Verify all flash messages are cleared
        let all_flash = ctx.session.flash_get_all();
        assert_eq!(all_flash.len(), 0);
    }

    #[tokio::test]
    async fn test_flash_clear_key() {
        let ctx = create_test_context().await;
        
        // Set multiple flash messages
        ctx.flash("keep_me", "I should stay").unwrap();
        ctx.flash("remove_me", "I should be removed").unwrap();
        ctx.flash_error("Error to remove");
        ctx.flash_success("Success to keep");
        
        // Clear specific flash messages
        let removed1 = ctx.flash_clear_key("remove_me");
        let removed2 = ctx.flash_clear_key("error_msg");
        let removed3 = ctx.flash_clear_key("non_existent");
        
        // Verify removed values
        assert_eq!(removed1, Some(json!("I should be removed")));
        assert_eq!(removed2, Some(json!("Error to remove")));
        assert_eq!(removed3, None);
        
        // Verify remaining flash messages
        let all_flash = ctx.session.flash_get_all();
        assert_eq!(all_flash.len(), 2);
        assert_eq!(all_flash.get("keep_me"), Some(&json!("I should stay")));
        assert_eq!(all_flash.get("success_msg"), Some(&json!("Success to keep")));
    }

    #[tokio::test]
    async fn test_flash_clear_then_set_new() {
        let ctx = create_test_context().await;
        
        // Set initial flash messages
        ctx.flash_error("Old error");
        ctx.flash_info("Old info");
        
        // Clear all and set new ones
        ctx.flash_clear();
        ctx.flash_success("New success");
        ctx.flash("custom", "New custom").unwrap();
        
        // Verify only new messages exist
        let all_flash = ctx.session.flash_get_all();
        assert_eq!(all_flash.len(), 2);
        assert_eq!(all_flash.get("success_msg"), Some(&json!("New success")));
        assert_eq!(all_flash.get("custom"), Some(&json!("New custom")));
        assert_eq!(all_flash.get("error_msg"), None);
        assert_eq!(all_flash.get("info_msg"), None);
    }
}