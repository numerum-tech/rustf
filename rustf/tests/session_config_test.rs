#[cfg(test)]
mod tests {
    use rustf::config::{AppConfig, SessionConfig};
    use std::fs;

    #[test]
    fn test_session_config_defaults() {
        let config = SessionConfig::default();

        assert_eq!(config.enabled, true);
        assert_eq!(config.cookie_name, "rustf_session");
        assert_eq!(config.idle_timeout, 900); // 15 minutes
        assert_eq!(config.absolute_timeout, 28800); // 8 hours
        assert_eq!(config.same_site, "Lax");
    }

    #[test]
    fn test_load_development_config() {
        let config_content = r#"
environment = "development"

[session]
enabled = true
cookie_name = "dev_session"
idle_timeout = 3600
absolute_timeout = 86400
same_site = "Lax"

[session.storage]
type = "memory"
"#;

        // Write test config
        let test_path = "test_dev_config.toml";
        fs::write(test_path, config_content).unwrap();

        // Load config
        let config = AppConfig::from_file(test_path).unwrap();

        // Check session settings
        assert_eq!(config.session.enabled, true);
        assert_eq!(config.session.cookie_name, "dev_session");
        assert_eq!(config.session.idle_timeout, 3600);
        assert_eq!(config.session.absolute_timeout, 86400);
        assert_eq!(config.session.same_site, "Lax");

        // Cleanup
        fs::remove_file(test_path).unwrap();
    }

    #[test]
    fn test_load_production_config() {
        let config_content = r#"
environment = "production"

[session]
enabled = true
cookie_name = "prod_sid"
idle_timeout = 900
absolute_timeout = 28800
same_site = "Strict"

[session.storage]
type = "memory"
"#;

        // Write test config
        let test_path = "test_prod_config.toml";
        fs::write(test_path, config_content).unwrap();

        // Load config
        let config = AppConfig::from_file(test_path).unwrap();

        // Check session settings
        assert_eq!(config.session.enabled, true);
        assert_eq!(config.session.cookie_name, "prod_sid");
        assert_eq!(config.session.idle_timeout, 900);
        assert_eq!(config.session.absolute_timeout, 28800);
        assert_eq!(config.session.same_site, "Strict");

        // Cleanup
        fs::remove_file(test_path).unwrap();
    }

    #[test]
    fn test_disabled_sessions() {
        let config_content = r#"
environment = "production"

[session]
enabled = false
"#;

        // Write test config
        let test_path = "test_no_session_config.toml";
        fs::write(test_path, config_content).unwrap();

        // Load config
        let config = AppConfig::from_file(test_path).unwrap();

        // Check session is disabled
        assert_eq!(config.session.enabled, false);

        // Cleanup
        fs::remove_file(test_path).unwrap();
    }

    #[test]
    fn test_config_adapter() {
        use rustf::session::manager::SessionConfig as InternalConfig;
        use rustf::session::SameSite;

        let file_config = SessionConfig {
            enabled: true,
            cookie_name: "test_session".to_string(),
            idle_timeout: 1800,
            absolute_timeout: 7200,
            same_site: "Strict".to_string(),
            fingerprint_mode: "soft".to_string(),
            storage: Default::default(),
            exempt_routes: vec![],
        };

        let internal_config: InternalConfig = file_config.into();

        assert_eq!(internal_config.cookie_name, "test_session");
        assert_eq!(internal_config.idle_timeout.as_secs(), 1800);
        assert_eq!(internal_config.absolute_timeout.as_secs(), 7200);
        assert!(matches!(internal_config.same_site, SameSite::Strict));

        // Security defaults
        assert_eq!(internal_config.http_only, true);
        assert_eq!(internal_config.rotation_on_privilege, true);
        assert_eq!(internal_config.secure_id_length, 32);
    }
}
