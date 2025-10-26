#[cfg(test)]
mod tests {
    use super::super::*;
    use serde_json::json;

    #[test]
    fn test_json_session_basic_operations() {
        let session = Session::new("test-session");
        
        // Test setting and getting values
        session.set("user_id", "123").unwrap();
        session.set("count", 42).unwrap();
        session.set("active", true).unwrap();
        
        assert_eq!(session.get::<String>("user_id"), Some("123".to_string()));
        assert_eq!(session.get::<i32>("count"), Some(42));
        assert_eq!(session.get::<bool>("active"), Some(true));
    }
    
    #[test]
    fn test_json_session_to_value_zero_cost() {
        let session = Session::new("test-session");
        
        // Set some data
        session.set("name", "John").unwrap();
        session.set("age", 30).unwrap();
        
        // Get as Value - this should be zero-cost (just clone the Arc'd Value)
        let value = session.to_value();
        
        // Verify the data is correct
        assert!(value.is_object());
        if let Value::Object(map) = value {
            assert_eq!(map.get("name"), Some(&json!("John")));
            assert_eq!(map.get("age"), Some(&json!(30)));
        }
    }
    
    #[test]
    fn test_json_session_flash_messages() {
        let session = Session::new("test-session");
        
        // Set flash messages
        session.flash_set("error", "Something went wrong").unwrap();
        session.flash_set("success", "Operation completed").unwrap();
        
        // Get and consume flash messages
        assert_eq!(session.flash_get::<String>("error"), Some("Something went wrong".to_string()));
        assert_eq!(session.flash_get::<String>("error"), None); // Already consumed
        
        // Get all flash messages
        session.flash_set("info", "Info message").unwrap();
        let all = session.flash_get_all();
        assert_eq!(all.get("success"), Some(&json!("Operation completed")));
        assert_eq!(all.get("info"), Some(&json!("Info message")));
        
        // Verify flash is cleared
        assert_eq!(session.flash_count(), 0);
    }
    
    #[test]
    fn test_json_session_to_from_data() {
        let session1 = Session::new("test-session");
        
        // Set data
        session1.set("user", json!({"id": 123, "name": "Alice"})).unwrap();
        session1.flash_set("message", "Hello").unwrap();
        
        // Convert to SessionData
        let data = session1.to_data();
        
        // Create new session from SessionData
        let session2 = Session::from_data("test-session", data);
        
        // Verify data is preserved
        let user: serde_json::Value = session2.get("user").unwrap();
        assert_eq!(user["id"], 123);
        assert_eq!(user["name"], "Alice");
        
        // Verify flash is preserved
        assert_eq!(session2.flash_get::<String>("message"), Some("Hello".to_string()));
    }
    
    #[test] 
    fn test_json_session_clear_operations() {
        let session = Session::new("test-session");
        
        // Add data
        session.set("key1", "value1").unwrap();
        session.set("key2", "value2").unwrap();
        session.flash_set("flash1", "message1").unwrap();
        
        assert_eq!(session.data_count(), 2);
        assert_eq!(session.flash_count(), 1);
        assert!(!session.is_empty());
        
        // Clear all
        session.clear();
        
        assert_eq!(session.data_count(), 0);
        assert_eq!(session.flash_count(), 0);
        assert!(session.is_empty());
    }
    
    #[test]
    fn test_json_session_remove_operations() {
        let session = Session::new("test-session");
        
        // Add and remove data
        session.set("keep", "this").unwrap();
        session.set("remove", "that").unwrap();
        
        let removed = session.remove("remove");
        assert_eq!(removed, Some(json!("that")));
        assert_eq!(session.get::<String>("keep"), Some("this".to_string()));
        assert_eq!(session.get::<String>("remove"), None);
    }
}