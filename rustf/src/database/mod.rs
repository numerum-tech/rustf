//! Multi-database support module for RustF
//!
//! This module provides the infrastructure for working with multiple databases
//! simultaneously, including adapters for different database backends and a
//! registry for managing multiple connections.

pub mod adapter;
pub mod adapters;
pub mod config;
pub mod registry;
pub mod types;

// Re-export main types for convenience
pub use adapter::{DatabaseAdapter, QueryResult};
pub use adapters::{MySqlAdapter, PostgresAdapter, SqliteAdapter};
pub use config::{DatabaseConnectionConfig, DatabasesConfig};
pub use registry::{DatabaseRegistry, RegistryStats};
pub use types::{DatabaseBackend, SqlValue, TypeConverter, TypeRegistry};
