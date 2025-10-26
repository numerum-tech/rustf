//! Database connection handling for RustF query builder
//!
//! This module provides unified database connection management across
//! different database backends (PostgreSQL, MySQL, SQLite).

use crate::models::query_builder::core::QueryBuilder;
use crate::models::query_builder::dialects::{DatabaseBackend, QueryError};

/// Unified database connection wrapper
pub enum AnyDatabase {
    Postgres(sqlx::PgPool),
    MySQL(sqlx::MySqlPool),
    SQLite(sqlx::SqlitePool),
}

impl AnyDatabase {
    /// Connect to database based on URL scheme
    pub async fn connect(database_url: &str) -> Result<Self, QueryError> {
        if database_url.starts_with("postgresql://") || database_url.starts_with("postgres://") {
            let pool = sqlx::PgPool::connect(database_url)
                .await
                .map_err(|e| QueryError::Database(e.to_string()))?;
            Ok(AnyDatabase::Postgres(pool))
        } else if database_url.starts_with("mysql://") || database_url.starts_with("mariadb://") {
            let pool = sqlx::MySqlPool::connect(database_url)
                .await
                .map_err(|e| QueryError::Database(e.to_string()))?;
            Ok(AnyDatabase::MySQL(pool))
        } else if database_url.starts_with("sqlite://") {
            let pool = sqlx::SqlitePool::connect(database_url)
                .await
                .map_err(|e| QueryError::Database(e.to_string()))?;
            Ok(AnyDatabase::SQLite(pool))
        } else {
            Err(QueryError::Database(
                "Unsupported database URL scheme".to_string(),
            ))
        }
    }

    /// Get the backend type
    pub fn backend(&self) -> DatabaseBackend {
        match self {
            AnyDatabase::Postgres(_) => DatabaseBackend::Postgres,
            AnyDatabase::MySQL(_) => DatabaseBackend::MySQL,
            AnyDatabase::SQLite(_) => DatabaseBackend::SQLite,
        }
    }

    /// Create a query builder for this database
    pub fn query(&self) -> QueryBuilder {
        QueryBuilder::new(self.backend())
    }

    /// Get the PostgreSQL pool if this is a Postgres connection
    pub fn pg_pool(&self) -> Option<&sqlx::PgPool> {
        match self {
            AnyDatabase::Postgres(pool) => Some(pool),
            _ => None,
        }
    }

    /// Get the MySQL pool if this is a MySQL connection
    pub fn mysql_pool(&self) -> Option<&sqlx::MySqlPool> {
        match self {
            AnyDatabase::MySQL(pool) => Some(pool),
            _ => None,
        }
    }

    /// Get the SQLite pool if this is a SQLite connection
    pub fn sqlite_pool(&self) -> Option<&sqlx::SqlitePool> {
        match self {
            AnyDatabase::SQLite(pool) => Some(pool),
            _ => None,
        }
    }
}
