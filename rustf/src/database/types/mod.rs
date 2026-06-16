//! Unified type system for database operations
//!
//! This module provides a single point of truth for all database type conversions,
//! mappings, and operations across PostgreSQL, MySQL, and SQLite.

pub mod converter;
#[cfg(feature = "db-mysql")]
pub mod mysql_converter;
#[cfg(feature = "db-postgres")]
pub mod postgres_converter;
pub mod registry;
#[cfg(feature = "db-sqlite")]
pub mod sqlite_converter;
pub mod value;

// Re-export the main types
pub use converter::{ColumnMetadata, ConversionUtils, DatabaseBackend, TypeConverter};
#[cfg(feature = "db-mysql")]
pub use mysql_converter::MySqlTypeConverter;
#[cfg(feature = "db-postgres")]
pub use postgres_converter::PostgresTypeConverter;
pub use registry::{DatabaseTypeInfo, RustType, SqlType, TypeMapping, TypeRegistry};
#[cfg(feature = "db-sqlite")]
pub use sqlite_converter::SqliteTypeConverter;
pub use value::SqlValue;

use std::sync::{Arc, OnceLock};

/// Global type registry instance using safe OnceLock (available since Rust 1.70)
/// This provides thread-safe one-time initialization without unsafe code
static TYPE_REGISTRY: OnceLock<Arc<TypeRegistry>> = OnceLock::new();

/// Get the global type registry
///
/// # Safety
/// This function is completely safe - it uses OnceLock for thread-safe initialization
/// The registry is initialized exactly once on first access and cached for all subsequent calls
pub fn get_type_registry() -> Arc<TypeRegistry> {
    TYPE_REGISTRY
        .get_or_init(|| Arc::new(TypeRegistry::new()))
        .clone()
}

/// Create a type converter for the specified database backend
///
/// # Panics
/// Panics if the requested backend's driver feature is not enabled
/// (e.g. requesting a Postgres converter when `db-postgres` is off).
#[cfg_attr(not(test), allow(clippy::panic))]
pub fn create_type_converter(backend: DatabaseBackend) -> Box<dyn TypeConverter> {
    match backend {
        #[cfg(feature = "db-postgres")]
        DatabaseBackend::Postgres => Box::new(PostgresTypeConverter::new()),
        #[cfg(feature = "db-mysql")]
        DatabaseBackend::MySQL | DatabaseBackend::MariaDB => Box::new(MySqlTypeConverter::new()),
        #[cfg(feature = "db-sqlite")]
        DatabaseBackend::SQLite => Box::new(SqliteTypeConverter::new()),
        #[allow(unreachable_patterns)]
        other => panic!(
            "Type converter for backend {:?} is unavailable: the corresponding database driver feature is not enabled",
            other
        ),
    }
}

/// Convert a database backend enum to the query builder's DatabaseBackend
pub fn to_query_builder_backend(
    backend: DatabaseBackend,
) -> crate::models::query_builder::DatabaseBackend {
    match backend {
        DatabaseBackend::Postgres => crate::models::query_builder::DatabaseBackend::Postgres,
        DatabaseBackend::MySQL => crate::models::query_builder::DatabaseBackend::MySQL,
        DatabaseBackend::MariaDB => crate::models::query_builder::DatabaseBackend::MariaDB,
        DatabaseBackend::SQLite => crate::models::query_builder::DatabaseBackend::SQLite,
    }
}

/// Convert from query builder's DatabaseBackend to type system's DatabaseBackend
pub fn from_query_builder_backend(
    backend: crate::models::query_builder::DatabaseBackend,
) -> DatabaseBackend {
    match backend {
        crate::models::query_builder::DatabaseBackend::Postgres => DatabaseBackend::Postgres,
        crate::models::query_builder::DatabaseBackend::MySQL => DatabaseBackend::MySQL,
        crate::models::query_builder::DatabaseBackend::MariaDB => DatabaseBackend::MariaDB,
        crate::models::query_builder::DatabaseBackend::SQLite => DatabaseBackend::SQLite,
    }
}
