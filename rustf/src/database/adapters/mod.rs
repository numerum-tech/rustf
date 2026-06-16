//! Database adapter implementations for different database backends

#[cfg(feature = "db-mysql")]
pub mod mysql;
#[cfg(feature = "db-postgres")]
pub mod postgres;
#[cfg(feature = "db-sqlite")]
pub mod sqlite;

#[cfg(feature = "db-mysql")]
pub use mysql::MySqlAdapter;
#[cfg(feature = "db-postgres")]
pub use postgres::PostgresAdapter;
#[cfg(feature = "db-sqlite")]
pub use sqlite::SqliteAdapter;
