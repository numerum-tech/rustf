//! Multi-database query builder for RustF framework
//!
//! This module provides a unified interface for building SQL queries across
//! different database backends (PostgreSQL, MySQL, SQLite) while handling
//! database-specific syntax differences automatically.

pub mod core;
pub mod dialects;
pub mod schema;

pub mod database;

// Re-export the main types for backward compatibility
#[cfg(feature = "db-mysql")]
pub use dialects::MySQLDialect;
#[cfg(feature = "db-postgres")]
pub use dialects::PostgresDialect;
#[cfg(feature = "db-sqlite")]
pub use dialects::SQLiteDialect;
pub use dialects::{DatabaseBackend, QueryError, SqlDialect};
pub use schema::{CreateTableBuilder, SchemaBuilder};

pub use database::AnyDatabase;

// Re-export commonly used types
pub use core::{
    JoinClause, JoinType, OrderByClause, OrderDirection, QueryBuilder, WhereCondition,
    WhereConnector,
};

// Re-export SqlValue from the unified type system
pub use crate::database::types::SqlValue;
