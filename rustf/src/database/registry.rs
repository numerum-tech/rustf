//! Database registry for managing multiple database connections
//!
//! This module provides a registry pattern for managing multiple named
//! database connections, enabling RustF applications to work with multiple
//! databases simultaneously.

use crate::database::adapter::DatabaseAdapter;
use crate::error::{Error, Result};
use crate::models::query_builder::QueryBuilder;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Registry for managing multiple database connections
pub struct DatabaseRegistry {
    /// Map of database name to adapter
    adapters: Arc<RwLock<HashMap<String, Box<dyn DatabaseAdapter>>>>,
    /// Name of the default database (if any)
    default: Arc<RwLock<Option<String>>>,
}

impl DatabaseRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            adapters: Arc::new(RwLock::new(HashMap::new())),
            default: Arc::new(RwLock::new(None)),
        }
    }

    /// Register a database adapter
    ///
    /// # Arguments
    /// * `name` - Unique name for this database connection
    /// * `adapter` - The database adapter to register
    /// * `set_as_default` - Whether to set this as the default database
    pub async fn register(
        &self,
        name: impl Into<String>,
        adapter: Box<dyn DatabaseAdapter>,
        set_as_default: bool,
    ) -> Result<()> {
        let name = name.into();

        // Add to registry
        let mut adapters = self.adapters.write().await;
        adapters.insert(name.clone(), adapter);

        // Set as default if requested or if it's the first database
        if set_as_default || adapters.len() == 1 {
            let mut default = self.default.write().await;
            *default = Some(name);
        }

        Ok(())
    }

    /// Get a database adapter by name
    ///
    /// # Arguments
    /// * `name` - Name of the database to retrieve
    ///
    /// # Returns
    /// * `Some(adapter)` - If the database exists
    /// * `None` - If no database with that name is registered
    pub async fn get(&self, name: &str) -> Option<Box<dyn DatabaseAdapter>> {
        let adapters = self.adapters.read().await;
        adapters.get(name).map(|adapter| adapter.clone_box())
    }

    /// Get the default database adapter
    ///
    /// # Returns
    /// * `Ok(adapter)` - The default database adapter
    /// * `Err(Error)` - If no default database is set
    pub async fn get_default(&self) -> Result<Box<dyn DatabaseAdapter>> {
        let default = self.default.read().await;

        match &*default {
            Some(name) => self.get(name).await.ok_or_else(|| {
                Error::template(format!("Default database '{}' not found in registry", name))
            }),
            None => Err(Error::template(
                "No default database configured".to_string(),
            )),
        }
    }

    /// Set the default database
    ///
    /// # Arguments
    /// * `name` - Name of the database to set as default
    ///
    /// # Returns
    /// * `Ok(())` - If the database exists and was set as default
    /// * `Err(Error)` - If the database doesn't exist
    pub async fn set_default(&self, name: impl Into<String>) -> Result<()> {
        let name = name.into();

        // Check if database exists
        let adapters = self.adapters.read().await;
        if !adapters.contains_key(&name) {
            return Err(Error::template(format!(
                "Database '{}' not found in registry",
                name
            )));
        }
        drop(adapters); // Release read lock

        // Set as default
        let mut default = self.default.write().await;
        *default = Some(name);

        Ok(())
    }

    /// Get a query builder for a specific database
    ///
    /// # Arguments
    /// * `name` - Name of the database to query
    ///
    /// # Returns
    /// * `Ok(QueryBuilder)` - Query builder for the specified database
    /// * `Err(Error)` - If the database doesn't exist
    pub async fn query(&self, name: &str) -> Result<QueryBuilder> {
        self.get(name)
            .await
            .map(|adapter| adapter.query())
            .ok_or_else(|| Error::template(format!("Database '{}' not found", name)))
    }

    /// Get a query builder for the default database
    ///
    /// # Returns
    /// * `Ok(QueryBuilder)` - Query builder for the default database
    /// * `Err(Error)` - If no default database is set
    pub async fn query_default(&self) -> Result<QueryBuilder> {
        let adapter = self.get_default().await?;
        Ok(adapter.query())
    }

    /// List all registered database names
    pub async fn list_databases(&self) -> Vec<String> {
        let adapters = self.adapters.read().await;
        adapters.keys().cloned().collect()
    }

    /// Check if a database is registered
    pub async fn has_database(&self, name: &str) -> bool {
        let adapters = self.adapters.read().await;
        adapters.contains_key(name)
    }

    /// Remove a database from the registry
    ///
    /// # Arguments
    /// * `name` - Name of the database to remove
    ///
    /// # Returns
    /// * `Ok(())` - If the database was removed
    /// * `Err(Error)` - If the database doesn't exist or is the default
    pub async fn remove(&self, name: &str) -> Result<()> {
        // Check if it's the default
        let default = self.default.read().await;
        if default.as_ref().map(|d| d == name).unwrap_or(false) {
            return Err(Error::template(
                "Cannot remove the default database. Set a different default first.".to_string(),
            ));
        }
        drop(default);

        // Remove from registry
        let mut adapters = self.adapters.write().await;
        adapters
            .remove(name)
            .ok_or_else(|| Error::template(format!("Database '{}' not found", name)))?;

        Ok(())
    }

    /// Clear all databases from the registry
    pub async fn clear(&self) {
        let mut adapters = self.adapters.write().await;
        adapters.clear();

        let mut default = self.default.write().await;
        *default = None;
    }

    /// Get statistics about the registry
    pub async fn stats(&self) -> RegistryStats {
        let adapters = self.adapters.read().await;
        let default = self.default.read().await;

        RegistryStats {
            total_databases: adapters.len(),
            default_database: default.clone(),
            database_names: adapters.keys().cloned().collect(),
        }
    }
}

/// Statistics about the database registry
#[derive(Debug, Clone)]
pub struct RegistryStats {
    /// Total number of registered databases
    pub total_databases: usize,
    /// Name of the default database (if any)
    pub default_database: Option<String>,
    /// List of all database names
    pub database_names: Vec<String>,
}

impl Default for DatabaseRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = DatabaseRegistry::new();
        let stats = registry.stats().await;

        assert_eq!(stats.total_databases, 0);
        assert!(stats.default_database.is_none());
        assert!(stats.database_names.is_empty());
    }

    #[tokio::test]
    async fn test_list_databases() {
        let registry = DatabaseRegistry::new();

        let databases = registry.list_databases().await;
        assert!(databases.is_empty());
    }

    #[tokio::test]
    async fn test_has_database() {
        let registry = DatabaseRegistry::new();

        assert!(!registry.has_database("test").await);
    }

    #[tokio::test]
    async fn test_get_default_error() {
        let registry = DatabaseRegistry::new();

        let result = registry.get_default().await;
        assert!(result.is_err());
    }
}
