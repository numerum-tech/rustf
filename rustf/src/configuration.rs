//! Global configuration access for RustF framework
//!
//! This module provides a global configuration singleton that enables uniform
//! configuration access throughout the application using dot notation paths.
//! All configuration sections - both predefined and custom - are accessed the same way.
//!
//! # Usage
//! ```rust
//! use rustf::CONF;
//!
//! // Access predefined configuration sections
//! let port = CONF::get_int("server.port").unwrap_or(8000);
//! let db_url = CONF::get_string("database.url");
//! let cache_enabled = CONF::get_bool("views.cache_enabled");
//!
//! // Access custom configuration sections (same syntax!)
//! let feature_flag = CONF::get_string("my-section.feature_flag");
//! let api_endpoint = CONF::get_string("my-section.api_endpoint");
//! let custom_val = CONF::get_string("custom.api_key");
//!
//! // With defaults
//! let timeout = CONF::get_or("server.timeout", 30);
//!
//! // Check existence
//! if CONF::has("database.url") {
//!     // Database is configured
//! }
//! ```

use crate::config::AppConfig;
use crate::error::{Error, Result};
use once_cell::sync::OnceCell;
use serde_json::Value;
use std::sync::Arc;

/// Global configuration instance
static CONFIG: OnceCell<ConfigStore> = OnceCell::new();

/// Internal storage for configuration
struct ConfigStore {
    /// Original typed configuration
    config: Arc<AppConfig>,
    /// JSON representation for path-based access
    json: Value,
}

/// Global configuration access point
///
/// Provides Total.js-style uniform configuration access using dot notation paths.
/// This allows any part of the application to access configuration values without
/// needing a Context or dependency injection.
pub struct CONF;

impl CONF {
    /// Initialize the global configuration
    ///
    /// This should be called once during application startup, typically in the
    /// RustF application builder.
    ///
    /// # Arguments
    /// * `config` - The application configuration to make globally available
    ///
    /// # Examples
    /// ```rust,ignore
    /// let config = AppConfig::load()?;
    /// CONF::init(config)?;
    /// ```
    pub fn init(config: AppConfig) -> Result<()> {
        // Convert config to JSON for path-based access
        let json = serde_json::to_value(&config)
            .map_err(|e| Error::internal(format!("Failed to serialize config: {}", e)))?;

        let store = ConfigStore {
            config: Arc::new(config),
            json,
        };

        CONFIG.set(store).map_err(|_| {
            Error::internal("Configuration has already been initialized".to_string())
        })?;

        log::debug!("Global configuration initialized");
        Ok(())
    }

    /// Get a configuration value by dot-notation path
    ///
    /// Returns the value at the specified path, or None if the path doesn't exist.
    /// The type T must implement Deserialize.
    ///
    /// # Arguments
    /// * `path` - Dot-separated path to the configuration value
    ///
    /// # Examples
    /// ```rust,ignore
    /// let port: Option<u16> = CONF::get("server.port");
    /// let db_url: Option<String> = CONF::get("database.url");
    /// ```
    pub fn get<T: serde::de::DeserializeOwned>(path: &str) -> Option<T> {
        let store = CONFIG.get()?;
        let value = Self::get_nested(&store.json, path)?;
        serde_json::from_value(value.clone()).ok()
    }

    /// Get a string configuration value
    ///
    /// Convenience method for getting string values.
    ///
    /// # Arguments
    /// * `path` - Dot-separated path to the configuration value
    ///
    /// # Examples
    /// ```rust,ignore
    /// let db_url = CONF::get_string("database.url");
    /// let api_key = CONF::get_string("custom.api_key");
    /// ```
    pub fn get_string(path: &str) -> Option<String> {
        Self::get(path)
    }

    /// Get an integer configuration value
    ///
    /// Convenience method for getting integer values.
    /// Handles both i64 and u64 internally.
    ///
    /// # Arguments
    /// * `path` - Dot-separated path to the configuration value
    ///
    /// # Examples
    /// ```rust,ignore
    /// let port = CONF::get_int("server.port").unwrap_or(8000);
    /// let max_connections = CONF::get_int("server.max_connections");
    /// ```
    pub fn get_int(path: &str) -> Option<i64> {
        let store = CONFIG.get()?;
        let value = Self::get_nested(&store.json, path)?;

        match value {
            Value::Number(n) => n.as_i64().or_else(|| n.as_u64().map(|u| u as i64)),
            _ => None,
        }
    }

    /// Get a boolean configuration value
    ///
    /// Convenience method for getting boolean values.
    ///
    /// # Arguments
    /// * `path` - Dot-separated path to the configuration value
    ///
    /// # Examples
    /// ```rust,ignore
    /// let ssl_enabled = CONF::get_bool("server.ssl_enabled").unwrap_or(false);
    /// let cache_enabled = CONF::get_bool("views.cache_enabled");
    /// ```
    pub fn get_bool(path: &str) -> Option<bool> {
        Self::get(path)
    }

    /// Get a float configuration value
    ///
    /// Convenience method for getting float values.
    ///
    /// # Arguments
    /// * `path` - Dot-separated path to the configuration value
    ///
    /// # Examples
    /// ```rust,ignore
    /// let rate = CONF::get_float("custom.exchange_rate");
    /// ```
    pub fn get_float(path: &str) -> Option<f64> {
        let store = CONFIG.get()?;
        let value = Self::get_nested(&store.json, path)?;

        match value {
            Value::Number(n) => n.as_f64(),
            _ => None,
        }
    }

    /// Get a configuration value with a default
    ///
    /// Returns the value at the specified path, or the default if the path doesn't exist.
    ///
    /// # Arguments
    /// * `path` - Dot-separated path to the configuration value
    /// * `default` - Default value to return if path doesn't exist
    ///
    /// # Examples
    /// ```rust,ignore
    /// let port = CONF::get_or("server.port", 8000);
    /// let timeout = CONF::get_or("server.timeout", 30);
    /// ```
    pub fn get_or<T: serde::de::DeserializeOwned>(path: &str, default: T) -> T {
        Self::get(path).unwrap_or(default)
    }

    /// Check if a configuration path exists
    ///
    /// Returns true if the specified path exists in the configuration.
    ///
    /// # Arguments
    /// * `path` - Dot-separated path to check
    ///
    /// # Examples
    /// ```rust,ignore
    /// if CONF::has("database.url") {
    ///     // Database is configured
    /// }
    /// ```
    pub fn has(path: &str) -> bool {
        CONFIG
            .get()
            .and_then(|store| Self::get_nested(&store.json, path))
            .is_some()
    }

    /// Get the entire configuration
    ///
    /// Returns the full AppConfig. This is rarely needed as path-based access
    /// is preferred, but it's available for cases where the typed config is required.
    ///
    /// # Examples
    /// ```rust,ignore
    /// let config = CONF::all();
    /// let env = config.environment.as_str();
    /// ```
    pub fn all() -> Option<Arc<AppConfig>> {
        CONFIG.get().map(|store| store.config.clone())
    }

    /// Get the current environment
    ///
    /// Convenience method to get the current environment.
    ///
    /// # Examples
    /// ```rust,ignore
    /// let env = CONF::env(); // "development", "production", etc.
    /// ```
    pub fn env() -> Option<String> {
        Self::get_string("environment")
    }

    /// Check if running in production
    ///
    /// Convenience method to check if the application is in production mode.
    ///
    /// # Examples
    /// ```rust,ignore
    /// if CONF::is_production() {
    ///     // Enable production optimizations
    /// }
    /// ```
    pub fn is_production() -> bool {
        Self::env().map(|e| e == "production").unwrap_or(false)
    }

    /// Check if running in development
    ///
    /// Convenience method to check if the application is in development mode.
    ///
    /// # Examples
    /// ```rust,ignore
    /// if CONF::is_development() {
    ///     // Enable development features
    /// }
    /// ```
    pub fn is_development() -> bool {
        Self::env().map(|e| e == "development").unwrap_or(true)
    }

    /// Check if configuration is initialized
    ///
    /// Returns true if CONF::init() has been called successfully.
    ///
    /// # Examples
    /// ```rust,ignore
    /// if CONF::is_initialized() {
    ///     let port = CONF::get_int("server.port");
    /// }
    /// ```
    pub fn is_initialized() -> bool {
        CONFIG.get().is_some()
    }

    /// Internal helper to get nested values from JSON using dot notation
    fn get_nested<'a>(obj: &'a Value, path: &str) -> Option<&'a Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = obj;

        for part in parts {
            match current {
                Value::Object(map) => {
                    current = map.get(part)?;
                }
                _ => return None,
            }
        }

        Some(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    #[test]
    fn test_conf_initialization_and_access() {
        // Create a test config
        let mut config = AppConfig::default();
        config.server.port = 3000;
        config.server.host = "localhost".to_string();
        config.views.directory = "/test/views".to_string();
        config
            .custom
            .insert("api_key".to_string(), "secret123".to_string());

        // Initialize CONF
        CONF::init(config).unwrap();

        // Test various access methods
        assert_eq!(CONF::get_int("server.port"), Some(3000));
        assert_eq!(
            CONF::get_string("server.host"),
            Some("localhost".to_string())
        );
        assert_eq!(
            CONF::get_string("views.directory"),
            Some("/test/views".to_string())
        );
        assert_eq!(
            CONF::get_string("custom.api_key"),
            Some("secret123".to_string())
        );

        // Test with defaults
        assert_eq!(CONF::get_or("server.port", 8000), 3000);
        assert_eq!(
            CONF::get_or("non.existent", "default".to_string()),
            "default".to_string()
        );

        // Test existence checks
        assert!(CONF::has("server.port"));
        assert!(CONF::has("custom.api_key"));
        assert!(!CONF::has("non.existent.path"));

        // Test environment helpers
        assert!(CONF::is_development());
        assert!(!CONF::is_production());
    }
}
