//! Database adapter trait for multi-database support
//!
//! This module provides a unified interface for different database backends,
//! allowing RustF to work with multiple databases simultaneously.

use crate::database::types::{from_query_builder_backend, SqlValue};
use crate::error::Result;
use crate::models::query_builder::{DatabaseBackend, QueryBuilder};
use async_trait::async_trait;
use serde_json::Value as JsonValue;

/// Result type for database query operations
#[derive(Debug)]
pub struct QueryResult {
    /// Number of rows affected by the query
    pub rows_affected: u64,
    /// Last inserted ID (if applicable)
    pub last_insert_id: Option<i64>,
}

/// Unified database adapter trait
///
/// This trait provides a common interface for all database backends,
/// enabling RustF to work with PostgreSQL, MySQL, and SQLite through
/// the same API.
#[async_trait]
pub trait DatabaseAdapter: Send + Sync {
    /// Get the name of this database connection
    fn name(&self) -> &str;

    /// Get the database backend type
    fn backend(&self) -> DatabaseBackend;

    /// Execute a query that modifies data (INSERT, UPDATE, DELETE)
    ///
    /// # Arguments
    /// * `sql` - The SQL query to execute
    /// * `params` - Parameters for the query
    ///
    /// # Returns
    /// * `Ok(QueryResult)` - Result with affected rows and last insert ID
    /// * `Err(Error)` - If the query fails
    async fn execute(&self, sql: &str, params: Vec<SqlValue>) -> Result<QueryResult>;

    /// Fetch all rows from a SELECT query
    ///
    /// # Arguments
    /// * `sql` - The SQL SELECT query
    /// * `params` - Parameters for the query
    ///
    /// # Returns
    /// * `Ok(Vec<JsonValue>)` - All matching rows as JSON values
    /// * `Err(Error)` - If the query fails
    async fn fetch_all(&self, sql: &str, params: Vec<SqlValue>) -> Result<Vec<JsonValue>>;

    /// Fetch a single row from a SELECT query
    ///
    /// # Arguments
    /// * `sql` - The SQL SELECT query
    /// * `params` - Parameters for the query
    ///
    /// # Returns
    /// * `Ok(Some(JsonValue))` - The first matching row
    /// * `Ok(None)` - If no rows match
    /// * `Err(Error)` - If the query fails
    async fn fetch_one(&self, sql: &str, params: Vec<SqlValue>) -> Result<Option<JsonValue>>;

    /// Test database connectivity
    ///
    /// # Returns
    /// * `Ok(true)` - Database is connected and responding
    /// * `Err(Error)` - Connection test failed
    async fn ping(&self) -> Result<bool>;

    /// Create a new query builder for this database
    ///
    /// # Returns
    /// A QueryBuilder configured for this database's SQL dialect
    fn query(&self) -> QueryBuilder {
        QueryBuilder::new(self.backend())
    }

    /// Get the database backend as the type system's DatabaseBackend
    fn type_backend(&self) -> crate::database::types::DatabaseBackend {
        from_query_builder_backend(self.backend())
    }

    /// Begin a transaction (optional, can return error for unsupported databases)
    ///
    /// # Returns
    /// * `Ok(())` - Transaction started
    /// * `Err(Error)` - If transactions aren't supported or fail to start
    async fn begin_transaction(&self) -> Result<()> {
        Err(crate::error::Error::database_transaction(
            "Transactions are not yet supported by the generic adapter API. Obtain the underlying sqlx::Pool via as_any() and manage transactions there."
        ))
    }

    /// Commit a transaction
    async fn commit(&self) -> Result<()> {
        Err(crate::error::Error::database_transaction(
            "Transactions are not yet supported by the generic adapter API. Obtain the underlying sqlx::Pool via as_any() and manage transactions there."
        ))
    }

    /// Rollback a transaction
    async fn rollback(&self) -> Result<()> {
        Err(crate::error::Error::database_transaction(
            "Transactions are not yet supported by the generic adapter API. Obtain the underlying sqlx::Pool via as_any() and manage transactions there."
        ))
    }

    /// Get the underlying connection pool as Any for downcasting
    ///
    /// This allows code that knows the specific database type to access
    /// the underlying sqlx pool directly when needed.
    fn as_any(&self) -> &dyn std::any::Any;

    /// Clone the adapter into a boxed trait object
    fn clone_box(&self) -> Box<dyn DatabaseAdapter>;
}

/// Extension trait for Arc<dyn DatabaseAdapter> to enable cloning
impl Clone for Box<dyn DatabaseAdapter> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
