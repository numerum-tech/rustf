//! Database configuration structures and parsing
//!
//! This module handles parsing database configurations from config files
//! and environment variables, supporting multiple named database connections.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a single database connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConnectionConfig {
    /// Database connection URL (required)
    pub url: String,

    /// Maximum number of connections in the pool
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Minimum number of connections to maintain
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,

    /// Connection timeout in seconds
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: u64,

    /// Idle timeout in seconds (how long a connection can be idle before being closed)
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout: u64,

    /// Maximum lifetime of a connection in seconds
    #[serde(default = "default_max_lifetime")]
    pub max_lifetime: u64,

    /// Whether this database should be set as the default
    #[serde(default)]
    pub is_default: bool,
}

/// Configuration for multiple databases
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabasesConfig {
    /// Map of database name to configuration
    #[serde(flatten)]
    pub databases: HashMap<String, DatabaseConnectionConfig>,
}

impl DatabasesConfig {
    /// Create a new empty databases configuration
    pub fn new() -> Self {
        Self {
            databases: HashMap::new(),
        }
    }

    /// Add a database configuration
    pub fn add_database(&mut self, name: impl Into<String>, config: DatabaseConnectionConfig) {
        self.databases.insert(name.into(), config);
    }

    /// Get a database configuration by name
    pub fn get(&self, name: &str) -> Option<&DatabaseConnectionConfig> {
        self.databases.get(name)
    }

    /// Get the default database configuration
    ///
    /// Returns the database marked as default, or the first one if none is marked
    pub fn get_default(&self) -> Option<(&String, &DatabaseConnectionConfig)> {
        // First, look for explicitly marked default
        for (name, config) in &self.databases {
            if config.is_default {
                return Some((name, config));
            }
        }

        // Otherwise, return the first database (if any)
        self.databases.iter().next()
    }

    /// Check if any databases are configured
    pub fn is_empty(&self) -> bool {
        self.databases.is_empty()
    }

    /// Get the number of configured databases
    pub fn len(&self) -> usize {
        self.databases.len()
    }

    /// List all database names
    pub fn list_names(&self) -> Vec<String> {
        self.databases.keys().cloned().collect()
    }

    /// Merge with another configuration (other takes precedence)
    pub fn merge(&mut self, other: DatabasesConfig) {
        for (name, config) in other.databases {
            self.databases.insert(name, config);
        }
    }

    /// Create from legacy single database configuration
    ///
    /// This enables backward compatibility with existing configurations
    /// that only have a single database.url field
    pub fn from_legacy(url: Option<String>, max_connections: Option<u32>) -> Self {
        let mut config = Self::new();

        if let Some(url) = url {
            let db_config = DatabaseConnectionConfig {
                url,
                max_connections: max_connections.unwrap_or_else(default_max_connections),
                min_connections: default_min_connections(),
                connect_timeout: default_connect_timeout(),
                idle_timeout: default_idle_timeout(),
                max_lifetime: default_max_lifetime(),
                is_default: true,
            };

            config.add_database("primary", db_config);
        }

        config
    }
}

// Default values for configuration
fn default_max_connections() -> u32 {
    10
}
fn default_min_connections() -> u32 {
    1
}
fn default_connect_timeout() -> u64 {
    30
}
fn default_idle_timeout() -> u64 {
    600
} // 10 minutes
fn default_max_lifetime() -> u64 {
    1800
} // 30 minutes

/// Builder for DatabaseConnectionConfig
pub struct DatabaseConnectionConfigBuilder {
    url: Option<String>,
    max_connections: u32,
    min_connections: u32,
    connect_timeout: u64,
    idle_timeout: u64,
    max_lifetime: u64,
    is_default: bool,
}

impl DatabaseConnectionConfigBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            url: None,
            max_connections: default_max_connections(),
            min_connections: default_min_connections(),
            connect_timeout: default_connect_timeout(),
            idle_timeout: default_idle_timeout(),
            max_lifetime: default_max_lifetime(),
            is_default: false,
        }
    }

    /// Set the database URL (required)
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set maximum connections
    pub fn max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }

    /// Set minimum connections
    pub fn min_connections(mut self, min: u32) -> Self {
        self.min_connections = min;
        self
    }

    /// Set connection timeout in seconds
    pub fn connect_timeout(mut self, timeout: u64) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set idle timeout in seconds
    pub fn idle_timeout(mut self, timeout: u64) -> Self {
        self.idle_timeout = timeout;
        self
    }

    /// Set maximum lifetime in seconds
    pub fn max_lifetime(mut self, lifetime: u64) -> Self {
        self.max_lifetime = lifetime;
        self
    }

    /// Mark as default database
    pub fn is_default(mut self, is_default: bool) -> Self {
        self.is_default = is_default;
        self
    }

    /// Build the configuration
    pub fn build(self) -> Result<DatabaseConnectionConfig, String> {
        let url = self.url.ok_or("Database URL is required")?;

        Ok(DatabaseConnectionConfig {
            url,
            max_connections: self.max_connections,
            min_connections: self.min_connections,
            connect_timeout: self.connect_timeout,
            idle_timeout: self.idle_timeout,
            max_lifetime: self.max_lifetime,
            is_default: self.is_default,
        })
    }
}

impl Default for DatabaseConnectionConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_config() {
        let config = DatabasesConfig::new();
        assert!(config.is_empty());
        assert_eq!(config.len(), 0);
    }

    #[test]
    fn test_add_database() {
        let mut config = DatabasesConfig::new();

        let db_config = DatabaseConnectionConfig {
            url: "postgresql://localhost/test".to_string(),
            max_connections: 20,
            min_connections: 5,
            connect_timeout: 30,
            idle_timeout: 600,
            max_lifetime: 1800,
            is_default: true,
        };

        config.add_database("primary", db_config.clone());

        assert_eq!(config.len(), 1);
        assert!(!config.is_empty());
        assert_eq!(config.get("primary").unwrap().url, db_config.url);
    }

    #[test]
    fn test_get_default() {
        let mut config = DatabasesConfig::new();

        // Add non-default database
        config.add_database(
            "secondary",
            DatabaseConnectionConfig {
                url: "mysql://localhost/secondary".to_string(),
                max_connections: 10,
                min_connections: 1,
                connect_timeout: 30,
                idle_timeout: 600,
                max_lifetime: 1800,
                is_default: false,
            },
        );

        // Add default database
        config.add_database(
            "primary",
            DatabaseConnectionConfig {
                url: "postgresql://localhost/primary".to_string(),
                max_connections: 20,
                min_connections: 5,
                connect_timeout: 30,
                idle_timeout: 600,
                max_lifetime: 1800,
                is_default: true,
            },
        );

        let (name, _) = config.get_default().unwrap();
        assert_eq!(name, "primary");
    }

    #[test]
    fn test_from_legacy() {
        let config = DatabasesConfig::from_legacy(
            Some("postgresql://localhost/legacy".to_string()),
            Some(25),
        );

        assert_eq!(config.len(), 1);
        let (name, db_config) = config.get_default().unwrap();
        assert_eq!(name, "primary");
        assert_eq!(db_config.url, "postgresql://localhost/legacy");
        assert_eq!(db_config.max_connections, 25);
        assert!(db_config.is_default);
    }

    #[test]
    fn test_builder() {
        let config = DatabaseConnectionConfigBuilder::new()
            .url("sqlite://./test.db")
            .max_connections(5)
            .min_connections(1)
            .is_default(true)
            .build()
            .unwrap();

        assert_eq!(config.url, "sqlite://./test.db");
        assert_eq!(config.max_connections, 5);
        assert_eq!(config.min_connections, 1);
        assert!(config.is_default);
    }
}
