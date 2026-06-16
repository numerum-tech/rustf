//! Database connection handling for RustF query builder
//!
//! This module provides unified database connection management across
//! different database backends (PostgreSQL, MySQL, SQLite).

use crate::models::query_builder::core::QueryBuilder;
use crate::models::query_builder::dialects::{DatabaseBackend, QueryError};

/// Unified database connection wrapper
pub enum AnyDatabase {
    #[cfg(feature = "db-postgres")]
    Postgres(sqlx::PgPool),
    #[cfg(feature = "db-mysql")]
    MySQL(sqlx::MySqlPool),
    #[cfg(feature = "db-sqlite")]
    SQLite(sqlx::SqlitePool),
}

impl AnyDatabase {
    /// Connect to database based on URL scheme
    pub async fn connect(database_url: &str) -> Result<Self, QueryError> {
        #[cfg(feature = "db-postgres")]
        if database_url.starts_with("postgresql://") || database_url.starts_with("postgres://") {
            let pool = sqlx::PgPool::connect(database_url)
                .await
                .map_err(|e| QueryError::Database(e.to_string()))?;
            return Ok(AnyDatabase::Postgres(pool));
        }
        #[cfg(feature = "db-mysql")]
        if database_url.starts_with("mysql://") || database_url.starts_with("mariadb://") {
            let pool = sqlx::MySqlPool::connect(database_url)
                .await
                .map_err(|e| QueryError::Database(e.to_string()))?;
            return Ok(AnyDatabase::MySQL(pool));
        }
        #[cfg(feature = "db-sqlite")]
        if database_url.starts_with("sqlite://") {
            let pool = sqlx::SqlitePool::connect(database_url)
                .await
                .map_err(|e| QueryError::Database(e.to_string()))?;
            return Ok(AnyDatabase::SQLite(pool));
        }
        Err(QueryError::Database(format!(
            "Unsupported database URL scheme (or its driver feature is not enabled): {}",
            database_url
        )))
    }

    /// Get the backend type
    pub fn backend(&self) -> DatabaseBackend {
        match self {
            #[cfg(feature = "db-postgres")]
            AnyDatabase::Postgres(_) => DatabaseBackend::Postgres,
            #[cfg(feature = "db-mysql")]
            AnyDatabase::MySQL(_) => DatabaseBackend::MySQL,
            #[cfg(feature = "db-sqlite")]
            AnyDatabase::SQLite(_) => DatabaseBackend::SQLite,
        }
    }

    /// Create a query builder for this database
    pub fn query(&self) -> QueryBuilder {
        QueryBuilder::new(self.backend())
    }

    /// Get the PostgreSQL pool if this is a Postgres connection
    #[cfg(feature = "db-postgres")]
    pub fn pg_pool(&self) -> Option<&sqlx::PgPool> {
        match self {
            AnyDatabase::Postgres(pool) => Some(pool),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }

    /// Get the MySQL pool if this is a MySQL connection
    #[cfg(feature = "db-mysql")]
    pub fn mysql_pool(&self) -> Option<&sqlx::MySqlPool> {
        match self {
            AnyDatabase::MySQL(pool) => Some(pool),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }

    /// Get the SQLite pool if this is a SQLite connection
    #[cfg(feature = "db-sqlite")]
    pub fn sqlite_pool(&self) -> Option<&sqlx::SqlitePool> {
        match self {
            AnyDatabase::SQLite(pool) => Some(pool),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }
}
