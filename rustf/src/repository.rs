//! Global repository access for RustF framework
//!
//! This module provides global APP/MAIN repository singletons that enable
//! shared data access throughout the application and in templates.
//!
//! # Usage
//! ```rust
//! use rustf::{APP, MAIN};
//!
//! // Set values in the global repository
//! APP::set("site_name", "My Application");
//! APP::set("version", "1.0.0");
//!
//! // Get values with dot notation
//! let site_name = APP::get_string("site_name");
//! let version = APP::get_string("version");
//!
//! // Check existence
//! if APP::has("site_name") {
//!     // Site name is configured
//! }
//! ```

use crate::error::{Error, Result};
use once_cell::sync::OnceCell;
use serde_json::Value;
use std::sync::{Arc, RwLock};

/// Global APP repository instance
static APP_REPOSITORY: OnceCell<Arc<RwLock<Value>>> = OnceCell::new();

/// Global MAIN repository instance (alias for APP)
static MAIN_REPOSITORY: OnceCell<Arc<RwLock<Value>>> = OnceCell::new();

/// Global APP repository access point
///
/// Provides Total.js-style global repository for application-wide data.
/// This allows any part of the application to access shared data without
/// needing a Context or dependency injection.
pub struct APP;

/// Global MAIN repository access point (alias for APP)
///
/// MAIN is an alias for APP, providing compatibility with Total.js conventions.
/// Both APP and MAIN share the same underlying repository.
pub struct MAIN;

impl APP {
    /// Initialize the global APP repository
    ///
    /// This should be called once during application startup, typically in the
    /// RustF application builder.
    ///
    /// # Arguments
    /// * `data` - The initial repository data (usually an empty object)
    ///
    /// # Examples
    /// ```rust,ignore
    /// APP::init(json!({}));
    /// ```
    pub fn init(data: Value) -> Result<()> {
        let repository = Arc::new(RwLock::new(data));

        // Set both APP and MAIN to the same repository
        APP_REPOSITORY.set(repository.clone()).map_err(|_| {
            Error::internal("APP repository has already been initialized".to_string())
        })?;

        MAIN_REPOSITORY.set(repository).map_err(|_| {
            Error::internal("MAIN repository has already been initialized".to_string())
        })?;

        log::debug!("Global APP/MAIN repository initialized");
        Ok(())
    }

    /// Get the entire repository Arc for direct access
    ///
    /// This is primarily used internally by the view engine.
    /// Most code should use the get/set methods instead.
    pub fn get_repository() -> Option<Arc<RwLock<Value>>> {
        APP_REPOSITORY.get().cloned()
    }

    /// Get a value from the repository by dot-notation path
    ///
    /// Returns the value at the specified path, or None if the path doesn't exist.
    ///
    /// # Arguments
    /// * `path` - Dot-separated path to the value
    ///
    /// # Examples
    /// ```rust,ignore
    /// let site_name: Option<String> = APP::get("site_name");
    /// let user_count: Option<i64> = APP::get("stats.users.count");
    /// ```
    pub fn get<T: serde::de::DeserializeOwned>(path: &str) -> Option<T> {
        let repo = APP_REPOSITORY.get()?;
        let value = repo.read().ok()?;
        let result = Self::get_nested(&value, path)?;
        serde_json::from_value(result.clone()).ok()
    }

    /// Get a string value from the repository
    ///
    /// Convenience method for getting string values.
    ///
    /// # Arguments
    /// * `path` - Dot-separated path to the value
    ///
    /// # Examples
    /// ```rust,ignore
    /// let site_name = APP::get_string("site_name");
    /// let api_url = APP::get_string("api.base_url");
    /// ```
    pub fn get_string(path: &str) -> Option<String> {
        Self::get(path)
    }

    /// Get an integer value from the repository
    ///
    /// Convenience method for getting integer values.
    ///
    /// # Arguments
    /// * `path` - Dot-separated path to the value
    ///
    /// # Examples
    /// ```rust,ignore
    /// let user_count = APP::get_int("stats.users.count");
    /// ```
    pub fn get_int(path: &str) -> Option<i64> {
        let repo = APP_REPOSITORY.get()?;
        let value = repo.read().ok()?;
        let result = Self::get_nested(&value, path)?;

        match result {
            Value::Number(n) => n.as_i64().or_else(|| n.as_u64().map(|u| u as i64)),
            _ => None,
        }
    }

    /// Get a boolean value from the repository
    ///
    /// Convenience method for getting boolean values.
    ///
    /// # Arguments
    /// * `path` - Dot-separated path to the value
    ///
    /// # Examples
    /// ```rust,ignore
    /// let maintenance_mode = APP::get_bool("maintenance_mode");
    /// ```
    pub fn get_bool(path: &str) -> Option<bool> {
        Self::get(path)
    }

    /// Get a value with a default
    ///
    /// Returns the value at the specified path, or the default if the path doesn't exist.
    ///
    /// # Arguments
    /// * `path` - Dot-separated path to the value
    /// * `default` - Default value to return if path doesn't exist
    ///
    /// # Examples
    /// ```rust,ignore
    /// let site_name = APP::get_or("site_name", "Default Site");
    /// ```
    pub fn get_or<T: serde::de::DeserializeOwned>(path: &str, default: T) -> T {
        Self::get(path).unwrap_or(default)
    }

    /// Set a value in the repository
    ///
    /// Sets a value at the specified path, creating nested objects as needed.
    ///
    /// # Arguments
    /// * `path` - Dot-separated path where to set the value
    /// * `value` - Value to set
    ///
    /// # Examples
    /// ```rust,ignore
    /// APP::set("site_name", "My Application");
    /// APP::set("stats.users.count", 100);
    /// APP::set("features.enabled", true);
    /// ```
    pub fn set<T: serde::Serialize>(path: &str, value: T) -> Result<()> {
        let repo = APP_REPOSITORY
            .get()
            .ok_or_else(|| Error::internal("APP repository not initialized".to_string()))?;

        let json_value = serde_json::to_value(value)?;
        let mut data = repo
            .write()
            .map_err(|e| Error::internal(format!("Failed to lock APP repository: {}", e)))?;

        Self::set_nested(&mut data, path, json_value);
        Ok(())
    }

    /// Remove a value from the repository
    ///
    /// Removes the value at the specified path.
    ///
    /// # Arguments
    /// * `path` - Dot-separated path to remove
    ///
    /// # Examples
    /// ```rust,ignore
    /// APP::remove("temp.data");
    /// ```
    pub fn remove(path: &str) -> Result<()> {
        let repo = APP_REPOSITORY
            .get()
            .ok_or_else(|| Error::internal("APP repository not initialized".to_string()))?;

        let mut data = repo
            .write()
            .map_err(|e| Error::internal(format!("Failed to lock APP repository: {}", e)))?;

        Self::remove_nested(&mut data, path);
        Ok(())
    }

    /// Check if a path exists in the repository
    ///
    /// Returns true if the specified path exists in the repository.
    ///
    /// # Arguments
    /// * `path` - Dot-separated path to check
    ///
    /// # Examples
    /// ```rust,ignore
    /// if APP::has("site_name") {
    ///     // Site name is configured
    /// }
    /// ```
    pub fn has(path: &str) -> bool {
        if let Some(repo) = APP_REPOSITORY.get() {
            if let Ok(data) = repo.read() {
                return Self::get_nested(&data, path).is_some();
            }
        }
        false
    }

    /// Clear the entire repository
    ///
    /// Removes all data from the repository, resetting it to an empty object.
    ///
    /// # Examples
    /// ```rust,ignore
    /// APP::clear();
    /// ```
    pub fn clear() -> Result<()> {
        let repo = APP_REPOSITORY
            .get()
            .ok_or_else(|| Error::internal("APP repository not initialized".to_string()))?;

        let mut data = repo
            .write()
            .map_err(|e| Error::internal(format!("Failed to lock APP repository: {}", e)))?;

        *data = Value::Object(serde_json::Map::new());
        Ok(())
    }

    /// Check if repository is initialized
    ///
    /// Returns true if APP::init() has been called successfully.
    ///
    /// # Examples
    /// ```rust,ignore
    /// if APP::is_initialized() {
    ///     let site_name = APP::get_string("site_name");
    /// }
    /// ```
    pub fn is_initialized() -> bool {
        APP_REPOSITORY.get().is_some()
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

    /// Internal helper to set nested values in JSON using dot notation
    fn set_nested(obj: &mut Value, path: &str, value: Value) {
        let parts: Vec<&str> = path.split('.').collect();

        if parts.is_empty() {
            return;
        }

        if parts.len() == 1 {
            if let Value::Object(map) = obj {
                map.insert(parts[0].to_string(), value);
            }
            return;
        }

        // Ensure obj is an object
        if !obj.is_object() {
            *obj = Value::Object(serde_json::Map::new());
        }

        if let Value::Object(map) = obj {
            let first = parts[0];
            let rest = &parts[1..].join(".");

            // Get or create the nested object
            let nested = map
                .entry(first.to_string())
                .or_insert_with(|| Value::Object(serde_json::Map::new()));

            Self::set_nested(nested, rest, value);
        }
    }

    /// Internal helper to remove nested values from JSON using dot notation
    fn remove_nested(obj: &mut Value, path: &str) {
        let parts: Vec<&str> = path.split('.').collect();

        if parts.is_empty() {
            return;
        }

        if parts.len() == 1 {
            if let Value::Object(map) = obj {
                map.remove(parts[0]);
            }
            return;
        }

        if let Value::Object(map) = obj {
            let first = parts[0];
            let rest = &parts[1..].join(".");

            if let Some(nested) = map.get_mut(first) {
                Self::remove_nested(nested, rest);
            }
        }
    }
}

// MAIN is an alias for APP, so all methods are the same
impl MAIN {
    /// Initialize the global MAIN repository (alias for APP::init)
    pub fn init(data: Value) -> Result<()> {
        APP::init(data)
    }

    /// Get the entire repository Arc for direct access (alias for APP::get_repository)
    pub fn get_repository() -> Option<Arc<RwLock<Value>>> {
        APP::get_repository()
    }

    /// Get a value from the repository (alias for APP::get)
    pub fn get<T: serde::de::DeserializeOwned>(path: &str) -> Option<T> {
        APP::get(path)
    }

    /// Get a string value (alias for APP::get_string)
    pub fn get_string(path: &str) -> Option<String> {
        APP::get_string(path)
    }

    /// Get an integer value (alias for APP::get_int)
    pub fn get_int(path: &str) -> Option<i64> {
        APP::get_int(path)
    }

    /// Get a boolean value (alias for APP::get_bool)
    pub fn get_bool(path: &str) -> Option<bool> {
        APP::get_bool(path)
    }

    /// Get a value with a default (alias for APP::get_or)
    pub fn get_or<T: serde::de::DeserializeOwned>(path: &str, default: T) -> T {
        APP::get_or(path, default)
    }

    /// Set a value in the repository (alias for APP::set)
    pub fn set<T: serde::Serialize>(path: &str, value: T) -> Result<()> {
        APP::set(path, value)
    }

    /// Remove a value from the repository (alias for APP::remove)
    pub fn remove(path: &str) -> Result<()> {
        APP::remove(path)
    }

    /// Check if a path exists (alias for APP::has)
    pub fn has(path: &str) -> bool {
        APP::has(path)
    }

    /// Clear the repository (alias for APP::clear)
    pub fn clear() -> Result<()> {
        APP::clear()
    }

    /// Check if repository is initialized (alias for APP::is_initialized)
    pub fn is_initialized() -> bool {
        APP::is_initialized()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_app_repository_operations() {
        // Initialize with empty object
        APP::init(json!({})).unwrap();

        // Test setting values
        APP::set("site_name", "Test Site").unwrap();
        APP::set("version", "1.0.0").unwrap();
        APP::set("stats.users.count", 100).unwrap();
        APP::set("features.enabled", true).unwrap();

        // Test getting values
        assert_eq!(APP::get_string("site_name"), Some("Test Site".to_string()));
        assert_eq!(APP::get_string("version"), Some("1.0.0".to_string()));
        assert_eq!(APP::get_int("stats.users.count"), Some(100));
        assert_eq!(APP::get_bool("features.enabled"), Some(true));

        // Test with defaults
        assert_eq!(APP::get_or("site_name", "Default".to_string()), "Test Site");
        assert_eq!(
            APP::get_or("non.existent", "default".to_string()),
            "default"
        );

        // Test existence checks
        assert!(APP::has("site_name"));
        assert!(APP::has("stats.users.count"));
        assert!(!APP::has("non.existent"));

        // Test removal
        APP::remove("features.enabled").unwrap();
        assert!(!APP::has("features.enabled"));

        // Test MAIN alias
        assert_eq!(MAIN::get_string("site_name"), Some("Test Site".to_string()));
        MAIN::set("alias_test", "works").unwrap();
        assert_eq!(APP::get_string("alias_test"), Some("works".to_string()));
    }
}
