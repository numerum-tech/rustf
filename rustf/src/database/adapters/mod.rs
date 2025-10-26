//! Database adapter implementations for different database backends

pub mod mysql;
pub mod postgres;
pub mod sqlite;

pub use mysql::MySqlAdapter;
pub use postgres::PostgresAdapter;
pub use sqlite::SqliteAdapter;
