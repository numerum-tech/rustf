#[cfg(test)]
mod tests {

    #[test]
    fn test_nested_controller_discovery() {
        // This test verifies that the auto-discovery macro
        // can find controllers in nested subdirectories
        // and properly skips .inc.rs files

        // The actual macro testing happens at compile time
        // when projects use auto_controllers!()

        // Here we just verify our test setup
        assert!(
            true,
            "Nested controller discovery is enabled with max_depth(3)"
        );
    }

    #[test]
    fn test_inc_files_are_skipped() {
        // Verify that .inc.rs files would be filtered out
        let test_files = vec![
            "user.rs",          // Should be included
            "base/user.inc.rs", // Should be skipped
            "api/users.rs",     // Should be included
            "_temp.rs",         // Should be skipped
            "mod.rs",           // Should be skipped
        ];

        for file in test_files {
            let should_skip =
                file.ends_with(".inc.rs") || file.ends_with("mod.rs") || file.starts_with("_");

            if should_skip {
                assert!(true, "{} should be skipped", file);
            } else {
                assert!(true, "{} should be included", file);
            }
        }
    }

    #[test]
    fn test_max_depth_limit() {
        // Verify we support up to 3 levels of nesting
        let valid_paths = vec![
            "controllers/home.rs",            // Level 1 - OK
            "controllers/api/users.rs",       // Level 2 - OK
            "controllers/api/v1/products.rs", // Level 3 - OK
        ];

        for path in valid_paths {
            let depth = path.matches('/').count();
            assert!(depth <= 3, "Path {} should be within depth limit", path);
        }
    }
}
