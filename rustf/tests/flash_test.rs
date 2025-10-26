#[cfg(test)]
mod tests {
    use rustf::session::Session;
    use serde_json::Value;

    #[test]
    fn test_flash_messages_native() {
        // Test Session's native flash methods
        let session = Session::new("test-session");

        // Set flash messages
        session.flash_set("success", "Operation completed").unwrap();
        session.flash_set("error", "Something went wrong").unwrap();

        // Get individual flash messages (consumes them)
        let success: Option<String> = session.flash_get("success");
        assert_eq!(success, Some("Operation completed".to_string()));

        // Verify it's consumed
        let success_again: Option<String> = session.flash_get("success");
        assert_eq!(success_again, None);

        // Get all remaining flash
        let all_flash = session.flash_get_all();
        assert_eq!(
            all_flash.get("error").and_then(|v| v.as_str()),
            Some("Something went wrong")
        );

        // Verify all are consumed
        let all_flash_again = session.flash_get_all();
        assert!(all_flash_again.is_empty());
    }

    #[test]
    fn test_session_data_json_native() {
        let session = Session::new("test-session");

        // Set data
        session.set("user_id", 123).unwrap();
        session.set("username", "john_doe").unwrap();

        // Get as JSON value (zero-copy)
        let session_data = session.to_value();

        // Verify it's proper JSON
        assert!(session_data.is_object());
        if let Value::Object(map) = session_data {
            assert_eq!(map.get("user_id").and_then(|v| v.as_i64()), Some(123));
            assert_eq!(
                map.get("username").and_then(|v| v.as_str()),
                Some("john_doe")
            );
        }
    }

    #[test]
    fn test_flash_and_data_separation() {
        let session = Session::new("test-session");

        // Set both data and flash
        session.set("persistent_key", "stays").unwrap();
        session.flash_set("temporary_key", "goes").unwrap();

        // Get session data - should NOT include flash
        let data = session.to_value();
        if let Value::Object(map) = data {
            assert!(map.contains_key("persistent_key"));
            assert!(!map.contains_key("temporary_key"));
        }

        // Get flash separately
        let flash = session.flash_get_all();
        assert!(flash.contains_key("temporary_key"));
        assert!(!flash.contains_key("persistent_key"));
    }
}
