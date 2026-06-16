//! Database dialect implementations for RustF query builder
//!
//! This module contains database-specific SQL generation logic, separated
//! by database type for better maintainability and extensibility.

/// Database backend types supported by RustF
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DatabaseBackend {
    Postgres,
    MySQL,
    MariaDB,
    SQLite,
}

/// Unified error type for query building
#[derive(thiserror::Error, Debug)]
pub enum QueryError {
    #[error("Missing required clause: {clause}. Add .{clause}() to your query.")]
    MissingClause { clause: String },

    #[error("Feature not supported in {backend:?}: {feature}")]
    UnsupportedFeature {
        backend: DatabaseBackend,
        feature: String,
    },

    #[error("Invalid syntax for {backend:?}: {message}")]
    InvalidSyntax {
        backend: DatabaseBackend,
        message: String,
    },

    #[error("Database error: {0}")]
    Database(String),

    #[error("Not found")]
    NotFound,
}

/// Trait for database-specific SQL generation
pub trait SqlDialect: Send + Sync {
    /// Quote an identifier (table name, column name) for this database
    fn quote_identifier(&self, identifier: &str) -> String;

    /// Generate a parameter placeholder for the given position
    fn placeholder(&self, position: usize) -> String;

    /// Generate LIMIT/OFFSET syntax for this database
    fn limit_syntax(&self, limit: Option<i64>, offset: Option<i64>) -> String;

    /// Generate RETURNING clause syntax (if supported)
    fn returning_syntax(&self, columns: &[String]) -> Option<String>;

    /// Generate UPSERT (INSERT ... ON CONFLICT) syntax
    fn upsert_syntax(&self, table: &str, columns: &[String], conflict_columns: &[String])
        -> String;

    /// Get the current timestamp expression for this database
    fn current_timestamp(&self) -> &'static str;

    /// Get the auto-increment column syntax for this database
    fn auto_increment_syntax(&self) -> &'static str;

    /// Get the boolean type name for this database
    fn boolean_type(&self) -> &'static str;

    /// Support for downcasting to specific dialect implementations
    fn as_any(&self) -> &dyn std::any::Any;
}

#[cfg(feature = "db-mysql")]
pub mod mysql;
#[cfg(feature = "db-postgres")]
pub mod postgres;
#[cfg(feature = "db-sqlite")]
pub mod sqlite;

#[cfg(feature = "db-mysql")]
pub use mysql::MySQLDialect;
#[cfg(feature = "db-postgres")]
pub use postgres::PostgresDialect;
#[cfg(feature = "db-sqlite")]
pub use sqlite::SQLiteDialect;

/// Factory function to create the appropriate dialect for a database backend
///
/// # Panics
/// Panics if the requested backend's driver feature is not enabled.
#[cfg_attr(not(test), allow(clippy::panic))]
pub fn create_dialect(backend: DatabaseBackend) -> Box<dyn SqlDialect> {
    match backend {
        #[cfg(feature = "db-postgres")]
        DatabaseBackend::Postgres => Box::new(PostgresDialect::new()),
        #[cfg(feature = "db-mysql")]
        DatabaseBackend::MySQL | DatabaseBackend::MariaDB => Box::new(MySQLDialect::new()),
        #[cfg(feature = "db-sqlite")]
        DatabaseBackend::SQLite => Box::new(SQLiteDialect::new()),
        #[allow(unreachable_patterns)]
        other => panic!(
            "Dialect for backend {:?} is unavailable: the corresponding database driver feature is not enabled",
            other
        ),
    }
}
