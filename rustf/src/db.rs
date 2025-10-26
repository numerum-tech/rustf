//! Global database access for RustF framework
//!
//! This module provides global database access with support for both
//! single database (legacy) and multiple database connections.

use crate::database::config::DatabasesConfig;
use crate::database::types::SqlValue;
use crate::database::{adapters::*, DatabaseAdapter, DatabaseRegistry};
use crate::error::{Error, Result};
use crate::models::query_builder::{AnyDatabase, DatabaseBackend, QueryBuilder};
use once_cell::sync::OnceCell;
use std::sync::Arc;

/// Global database instance (for backward compatibility)
static DATABASE: OnceCell<Option<Arc<AnyDatabase>>> = OnceCell::new();

/// Global database registry (for multi-database support)
static REGISTRY: OnceCell<Arc<DatabaseRegistry>> = OnceCell::new();

/// Global database access point
pub struct DB;

impl DB {
    /// Initialize the global database connection from a database URL (legacy)
    ///
    /// This method maintains backward compatibility with existing code.
    /// For new applications, consider using `init_registry` for multi-database support.
    ///
    /// # Arguments
    /// * `database_url` - Optional database connection URL. If None, database operations will return errors.
    ///
    /// # Examples
    /// ```rust
    /// // Initialize with MySQL
    /// DB::init(Some("mysql://user:pass@localhost/mydb")).await?;
    ///
    /// // Initialize without database (some apps don't need it)
    /// DB::init(None).await?;
    /// ```
    pub async fn init(database_url: Option<&str>) -> Result<()> {
        // Initialize registry if not already done
        let _ = REGISTRY.get_or_init(|| Arc::new(DatabaseRegistry::new()));

        let db_option = if let Some(url) = database_url {
            log::info!(
                "Initializing database connection to: {}",
                Self::sanitize_url(url)
            );

            // Also add to registry as default
            if let Ok(registry) = Self::get_registry() {
                let adapter = Self::create_adapter("primary", url).await?;
                registry.register("primary", adapter, true).await?;
            }

            match AnyDatabase::connect(url).await {
                Ok(db) => {
                    log::info!("Database connection established successfully");
                    Some(Arc::new(db))
                }
                Err(e) => {
                    log::error!("Failed to connect to database: {}", e);
                    return Err(Error::template(format!(
                        "Database connection failed: {}",
                        e
                    )));
                }
            }
        } else {
            log::info!("No database URL provided - database operations will be unavailable");
            None
        };

        DATABASE
            .set(db_option)
            .map_err(|_| Error::template("Database has already been initialized".to_string()))?;

        Ok(())
    }

    /// Initialize database registry with multiple databases
    ///
    /// This is the new recommended way to initialize databases, supporting
    /// multiple named connections.
    ///
    /// # Arguments
    /// * `config` - Database configuration with one or more database connections
    ///
    /// # Examples
    /// ```rust
    /// let mut config = DatabasesConfig::new();
    /// config.add_database("primary", DatabaseConnectionConfig {
    ///     url: "postgresql://localhost/main".to_string(),
    ///     max_connections: 20,
    ///     ..Default::default()
    /// });
    /// config.add_database("analytics", DatabaseConnectionConfig {
    ///     url: "mysql://localhost/analytics".to_string(),
    ///     max_connections: 10,
    ///     ..Default::default()
    /// });
    ///
    /// DB::init_registry(config).await?;
    /// ```
    pub async fn init_registry(config: DatabasesConfig) -> Result<()> {
        let registry = REGISTRY.get_or_init(|| Arc::new(DatabaseRegistry::new()));

        // Get default database name if any
        let default_name = config.get_default().map(|(name, _)| name.clone());

        // Register all databases
        for (name, db_config) in &config.databases {
            log::info!(
                "Registering database '{}': {}",
                name,
                Self::sanitize_url(&db_config.url)
            );

            let adapter = Self::create_adapter(name, &db_config.url).await?;
            let is_default = default_name.as_ref().map(|d| d == name).unwrap_or(false);

            registry.register(name.clone(), adapter, is_default).await?;
        }

        // If we have a default database, also set it in the legacy DATABASE global
        if let Some(default_name) = default_name {
            if let Some(_adapter) = registry.get(&default_name).await {
                // Convert adapter back to AnyDatabase for legacy compatibility
                // This is a bit of a hack but maintains backward compatibility
                if let Some(default_config) = config.get(&default_name) {
                    if let Ok(db) = AnyDatabase::connect(&default_config.url).await {
                        let _ = DATABASE.set(Some(Arc::new(db)));
                    }
                }
            }
        }

        Ok(())
    }

    /// Create a database adapter based on the URL
    async fn create_adapter(name: &str, url: &str) -> Result<Box<dyn DatabaseAdapter>> {
        if url.starts_with("postgresql://") || url.starts_with("postgres://") {
            let adapter = PostgresAdapter::new(name, url).await?;
            Ok(Box::new(adapter))
        } else if url.starts_with("mysql://") || url.starts_with("mariadb://") {
            let adapter = MySqlAdapter::new(name, url).await?;
            Ok(Box::new(adapter))
        } else if url.starts_with("sqlite://") {
            let adapter = SqliteAdapter::new(name, url).await?;
            Ok(Box::new(adapter))
        } else {
            Err(Error::template(format!(
                "Unsupported database URL scheme: {}",
                url
            )))
        }
    }

    /// Get access to the database registry
    fn get_registry() -> Result<Arc<DatabaseRegistry>> {
        REGISTRY
            .get()
            .cloned()
            .ok_or_else(|| Error::template("Database registry not initialized".to_string()))
    }

    /// Use a specific named database
    ///
    /// # Arguments
    /// * `name` - Name of the database to use
    ///
    /// # Returns
    /// A database handle that can be used to execute queries
    ///
    /// # Examples
    /// ```rust
    /// let users = DB::use("analytics")
    ///     .await?
    ///     .query()
    ///     .from("users")
    ///     .where_eq("active", true)
    ///     .build()?;
    /// ```
    pub async fn r#use(name: &str) -> Result<Box<dyn DatabaseAdapter>> {
        let registry = Self::get_registry()?;
        registry
            .get(name)
            .await
            .ok_or_else(|| Error::template(format!("Database '{}' not found", name)))
    }

    /// Get a specific database adapter
    ///
    /// Alias for `use` that doesn't require the `r#` prefix
    pub async fn adapter(name: &str) -> Result<Box<dyn DatabaseAdapter>> {
        Self::r#use(name).await
    }

    /// Get a query builder for the configured database
    ///
    /// Returns a QueryBuilder that's pre-configured for the current database backend.
    /// This maintains backward compatibility with existing code.
    ///
    /// # Returns
    /// * `Ok(QueryBuilder)` - Ready-to-use query builder
    /// * `Err(Error)` - If database is not initialized or not configured
    ///
    /// # Examples
    /// ```rust
    /// let users = DB::query()
    ///     .from("users")
    ///     .where_eq("active", true)
    ///     .limit(10)
    ///     .build()?;
    /// ```
    pub fn query() -> Result<QueryBuilder> {
        // First try registry default
        if let Ok(registry) = Self::get_registry() {
            if let Ok(query) = futures::executor::block_on(registry.query_default()) {
                return Ok(query);
            }
        }

        // Fall back to legacy DATABASE
        match Self::connection() {
            Some(db) => Ok(db.query()),
            None => Err(Error::template(
                "Database not configured. Add database.url to your configuration or call DB::init()".to_string()
            ))
        }
    }

    /// Get direct access to the database connection
    ///
    /// Returns the underlying AnyDatabase connection for advanced operations.
    ///
    /// # Returns
    /// * `Some(Arc<AnyDatabase>)` - Database connection if initialized
    /// * `None` - If no database is configured
    pub fn connection() -> Option<Arc<AnyDatabase>> {
        DATABASE.get()?.clone()
    }

    /// Get the database connection pool for direct use
    ///
    /// This method is for compatibility with code expecting DB::get().
    /// Returns a Result containing the database connection.
    ///
    /// # Returns
    /// * `Ok(Arc<AnyDatabase>)` - Database connection if initialized
    /// * `Err(Error)` - If database is not configured
    pub async fn get() -> Result<Arc<AnyDatabase>> {
        Self::connection().ok_or_else(|| {
            Error::template(
                "Database not configured. Add database.url to your configuration".to_string(),
            )
        })
    }

    /// Check if database is initialized and available
    ///
    /// # Returns
    /// * `true` - Database is ready for use
    /// * `false` - Database is not configured or initialization failed
    pub fn is_initialized() -> bool {
        // Check both registry and legacy DATABASE
        if let Ok(registry) = Self::get_registry() {
            if futures::executor::block_on(registry.stats()).total_databases > 0 {
                return true;
            }
        }

        DATABASE.get().map(|db| db.is_some()).unwrap_or(false)
    }

    /// Get the database backend type if initialized
    ///
    /// # Returns
    /// * `Some(DatabaseBackend)` - The type of database (Postgres, MySQL, SQLite)
    /// * `None` - If database is not initialized
    pub fn backend() -> Option<DatabaseBackend> {
        Self::connection().map(|db| db.backend())
    }

    /// Get the PostgreSQL pool if database is PostgreSQL
    ///
    /// # Returns
    /// * `Ok(Arc<sqlx::PgPool>)` - PostgreSQL connection pool wrapped in Arc
    /// * `Err(Error)` - If database is not configured or is not PostgreSQL
    pub fn pg_pool() -> Result<Arc<sqlx::PgPool>> {
        let db = Self::connection()
            .ok_or_else(|| Error::template("Database not configured".to_string()))?;

        match db.as_ref() {
            AnyDatabase::Postgres(pool) => Ok(Arc::new(pool.clone())),
            _ => Err(Error::template("Database is not PostgreSQL".to_string())),
        }
    }

    /// Get the MySQL pool if database is MySQL
    ///
    /// # Returns
    /// * `Ok(Arc<sqlx::MySqlPool>)` - MySQL connection pool wrapped in Arc
    /// * `Err(Error)` - If database is not configured or is not MySQL
    pub fn mysql_pool() -> Result<Arc<sqlx::MySqlPool>> {
        let db = Self::connection()
            .ok_or_else(|| Error::template("Database not configured".to_string()))?;

        match db.as_ref() {
            AnyDatabase::MySQL(pool) => Ok(Arc::new(pool.clone())),
            _ => Err(Error::template("Database is not MySQL".to_string())),
        }
    }

    /// Get the SQLite pool if database is SQLite
    ///
    /// # Returns
    /// * `Ok(Arc<sqlx::SqlitePool>)` - SQLite connection pool wrapped in Arc
    /// * `Err(Error)` - If database is not configured or is not SQLite
    pub fn sqlite_pool() -> Result<Arc<sqlx::SqlitePool>> {
        let db = Self::connection()
            .ok_or_else(|| Error::template("Database not configured".to_string()))?;

        match db.as_ref() {
            AnyDatabase::SQLite(pool) => Ok(Arc::new(pool.clone())),
            _ => Err(Error::template("Database is not SQLite".to_string())),
        }
    }

    /// Execute a query with parameters using the appropriate database adapter
    ///
    /// This method automatically handles database-specific type conversions
    /// through the adapter layer, eliminating the need for database-specific
    /// binding code in models.
    ///
    /// # Arguments
    /// * `sql` - SQL query to execute
    /// * `params` - Parameters to bind to the query
    ///
    /// # Returns
    /// * `Ok(u64)` - Number of rows affected
    /// * `Err(Error)` - If execution fails
    pub async fn execute_with_params(sql: &str, params: Vec<SqlValue>) -> Result<u64> {
        // Try registry first
        if let Ok(registry) = Self::get_registry() {
            if let Ok(adapter) = registry.get_default().await {
                let result = adapter.execute(sql, params).await?;
                return Ok(result.rows_affected);
            }
        }

        // Fallback to legacy connection
        let db = Self::connection()
            .ok_or_else(|| Error::template("Database not configured".to_string()))?;

        // Use the appropriate adapter based on database type
        match db.as_ref() {
            AnyDatabase::Postgres(pool) => {
                use crate::database::adapters::PostgresAdapter;
                let adapter = PostgresAdapter::from_pool("default", pool.clone());
                let result = adapter.execute(sql, params).await?;
                Ok(result.rows_affected)
            }
            AnyDatabase::MySQL(pool) => {
                use crate::database::adapters::MySqlAdapter;
                let adapter = MySqlAdapter::from_pool("default", pool.clone());
                let result = adapter.execute(sql, params).await?;
                Ok(result.rows_affected)
            }
            AnyDatabase::SQLite(pool) => {
                use crate::database::adapters::SqliteAdapter;
                let adapter = SqliteAdapter::from_pool("default", pool.clone());
                let result = adapter.execute(sql, params).await?;
                Ok(result.rows_affected)
            }
        }
    }

    /// Fetch all rows from a query with parameters
    ///
    /// Returns results as JSON values for flexibility.
    ///
    /// # Arguments
    /// * `sql` - SQL query to execute
    /// * `params` - Parameters to bind to the query
    ///
    /// # Returns
    /// * `Ok(Vec<serde_json::Value>)` - Query results as JSON
    /// * `Err(Error)` - If execution fails
    pub async fn fetch_all_with_params(
        sql: &str,
        params: Vec<SqlValue>,
    ) -> Result<Vec<serde_json::Value>> {
        // Try registry first
        if let Ok(registry) = Self::get_registry() {
            if let Ok(adapter) = registry.get_default().await {
                return adapter.fetch_all(sql, params).await;
            }
        }

        // Fallback to legacy connection
        let db = Self::connection()
            .ok_or_else(|| Error::template("Database not configured".to_string()))?;

        // Use the appropriate adapter based on database type
        match db.as_ref() {
            AnyDatabase::Postgres(pool) => {
                use crate::database::adapters::PostgresAdapter;
                let adapter = PostgresAdapter::from_pool("default", pool.clone());
                adapter.fetch_all(sql, params).await
            }
            AnyDatabase::MySQL(pool) => {
                use crate::database::adapters::MySqlAdapter;
                let adapter = MySqlAdapter::from_pool("default", pool.clone());
                adapter.fetch_all(sql, params).await
            }
            AnyDatabase::SQLite(pool) => {
                use crate::database::adapters::SqliteAdapter;
                let adapter = SqliteAdapter::from_pool("default", pool.clone());
                adapter.fetch_all(sql, params).await
            }
        }
    }

    /// Fetch one row from a query with parameters
    ///
    /// Returns result as JSON value for flexibility.
    ///
    /// # Arguments
    /// * `sql` - SQL query to execute
    /// * `params` - Parameters to bind to the query
    ///
    /// # Returns
    /// * `Ok(Option<serde_json::Value>)` - Query result as JSON if found
    /// * `Err(Error)` - If execution fails
    pub async fn fetch_one_with_params(
        sql: &str,
        params: Vec<SqlValue>,
    ) -> Result<Option<serde_json::Value>> {
        // Try registry first
        if let Ok(registry) = Self::get_registry() {
            if let Ok(adapter) = registry.get_default().await {
                return adapter.fetch_one(sql, params).await;
            }
        }

        // Fallback to legacy connection
        let db = Self::connection()
            .ok_or_else(|| Error::template("Database not configured".to_string()))?;

        // Use the appropriate adapter based on database type
        match db.as_ref() {
            AnyDatabase::Postgres(pool) => {
                use crate::database::adapters::PostgresAdapter;
                let adapter = PostgresAdapter::from_pool("default", pool.clone());
                adapter.fetch_one(sql, params).await
            }
            AnyDatabase::MySQL(pool) => {
                use crate::database::adapters::MySqlAdapter;
                let adapter = MySqlAdapter::from_pool("default", pool.clone());
                adapter.fetch_one(sql, params).await
            }
            AnyDatabase::SQLite(pool) => {
                use crate::database::adapters::SqliteAdapter;
                let adapter = SqliteAdapter::from_pool("default", pool.clone());
                adapter.fetch_one(sql, params).await
            }
        }
    }

    /// Execute an INSERT query and return the inserted row
    ///
    /// This method handles database-specific RETURNING syntax and type conversions.
    /// For databases that don't support RETURNING, it uses alternative methods
    /// to fetch the inserted row.
    ///
    /// # Arguments
    /// * `sql` - INSERT SQL query (without RETURNING clause)
    /// * `params` - Parameters to bind to the query
    /// * `table` - Table name for fallback fetch
    /// * `id_field` - Name of the ID field for fallback fetch
    ///
    /// # Returns
    /// * `Ok(Some(JsonValue))` - The inserted row as JSON
    /// * `Err(Error)` - If insertion fails
    pub async fn execute_insert_returning(
        sql: &str,
        params: Vec<SqlValue>,
        table: &str,
        id_field: &str,
    ) -> Result<Option<serde_json::Value>> {
        // Try registry first
        if let Ok(registry) = Self::get_registry() {
            if let Ok(adapter) = registry.get_default().await {
                // Add RETURNING clause based on database type
                if adapter.backend() == DatabaseBackend::Postgres {
                    // PostgreSQL supports RETURNING, but has issues with enum types
                    // Try RETURNING first, fall back to separate INSERT + SELECT if it fails
                    let sql_with_returning = format!("{} RETURNING *", sql);
                    match adapter.fetch_one(&sql_with_returning, params.clone()).await {
                        Ok(result) => return Ok(result),
                        Err(e) if e.to_string().contains("but expression is of type") => {
                            // Enum type error - fall back to INSERT then SELECT
                            log::debug!(
                                "PostgreSQL enum type issue detected, using fallback approach"
                            );

                            // Execute the INSERT without RETURNING
                            adapter.execute(sql, params).await?;

                            // Fetch the inserted row using a simple SELECT
                            // This assumes we have an id field - might need adjustment
                            let fetch_sql = format!(
                                "SELECT * FROM {} ORDER BY {} DESC LIMIT 1",
                                table, id_field
                            );
                            return adapter.fetch_one(&fetch_sql, vec![]).await;
                        }
                        Err(e) => return Err(e),
                    }
                } else if adapter.backend() == DatabaseBackend::SQLite {
                    // SQLite also supports RETURNING (since 3.35)
                    let sql_with_returning = format!("{} RETURNING *", sql);
                    return adapter.fetch_one(&sql_with_returning, params).await;
                } else {
                    // MySQL and SQLite need alternative approach
                    let result = adapter.execute(sql, params).await?;

                    // Fetch the inserted row
                    let fetch_sql = match adapter.backend() {
                        DatabaseBackend::MySQL | DatabaseBackend::MariaDB => {
                            // Use LAST_INSERT_ID() for MySQL/MariaDB
                            if let Some(last_id) = result.last_insert_id {
                                format!("SELECT * FROM {} WHERE {} = {}", table, id_field, last_id)
                            } else {
                                return Ok(None);
                            }
                        }
                        DatabaseBackend::SQLite => {
                            // SQLite uses last_insert_rowid()
                            format!(
                                "SELECT * FROM {} WHERE {} = last_insert_rowid()",
                                table, id_field
                            )
                        }
                        _ => return Ok(None),
                    };

                    return adapter.fetch_one(&fetch_sql, vec![]).await;
                }
            }
        }

        // Fallback to legacy connection
        let db = Self::connection()
            .ok_or_else(|| Error::template("Database not configured".to_string()))?;

        // Use the appropriate adapter based on database type
        match db.as_ref() {
            AnyDatabase::Postgres(pool) => {
                use crate::database::adapters::PostgresAdapter;
                let adapter = PostgresAdapter::from_pool("default", pool.clone());
                // Add RETURNING clause for PostgreSQL
                let sql_with_returning = format!("{} RETURNING *", sql);
                adapter.fetch_one(&sql_with_returning, params).await
            }
            AnyDatabase::MySQL(pool) => {
                use crate::database::adapters::MySqlAdapter;
                let adapter = MySqlAdapter::from_pool("default", pool.clone());
                let result = adapter.execute(sql, params).await?;

                if let Some(last_id) = result.last_insert_id {
                    let fetch_sql =
                        format!("SELECT * FROM {} WHERE {} = {}", table, id_field, last_id);
                    adapter.fetch_one(&fetch_sql, vec![]).await
                } else {
                    Ok(None)
                }
            }
            AnyDatabase::SQLite(pool) => {
                use crate::database::adapters::SqliteAdapter;
                let adapter = SqliteAdapter::from_pool("default", pool.clone());
                // Add RETURNING clause for SQLite
                let sql_with_returning = format!("{} RETURNING *", sql);
                adapter.fetch_one(&sql_with_returning, params).await
            }
        }
    }

    /// Execute a raw SQL query and return results
    ///
    /// This is a low-level method for cases where the query builder
    /// doesn't provide the needed functionality.
    ///
    /// # Arguments
    /// * `sql` - Raw SQL query string
    ///
    /// # Safety
    /// This method executes raw SQL. Ensure the SQL is safe and parameterized
    /// to prevent SQL injection attacks.
    pub async fn execute_raw(sql: &str) -> Result<sqlx::mysql::MySqlQueryResult> {
        let db = Self::connection()
            .ok_or_else(|| Error::template("Database not configured".to_string()))?;

        match db.as_ref() {
            AnyDatabase::MySQL(pool) => sqlx::query(sql)
                .execute(pool)
                .await
                .map_err(|e| Error::template(format!("SQL execution failed: {}", e))),
            AnyDatabase::Postgres(_) => Err(Error::template(
                "Raw query execution not yet implemented for PostgreSQL".to_string(),
            )),
            AnyDatabase::SQLite(_) => Err(Error::template(
                "Raw query execution not yet implemented for SQLite".to_string(),
            )),
        }
    }

    /// Test database connectivity
    ///
    /// Sends a simple ping/test query to verify the database connection is alive.
    /// This is useful for health checks and monitoring.
    ///
    /// # Returns
    /// * `Ok(true)` - Database is connected and responding
    /// * `Ok(false)` - Database is not configured (no connection URL)
    /// * `Err(Error)` - Database is configured but connection test failed
    ///
    /// # Examples
    /// ```rust
    /// // Health check during startup or monitoring
    /// if DB::ping().await? {
    ///     println!("Database is healthy");
    /// }
    /// ```
    pub async fn ping() -> Result<bool> {
        // Try registry first
        if let Ok(registry) = Self::get_registry() {
            if let Ok(adapter) = registry.get_default().await {
                return adapter.ping().await;
            }
        }

        // Fall back to legacy DATABASE
        // If no database is configured, return false (not an error)
        let db = match Self::connection() {
            Some(connection) => connection,
            None => return Ok(false),
        };

        // Test connectivity based on database type
        match db.as_ref() {
            AnyDatabase::Postgres(pool) => {
                // PostgreSQL: SELECT 1
                sqlx::query("SELECT 1")
                    .fetch_one(pool)
                    .await
                    .map(|_| true)
                    .map_err(|e| Error::template(format!("PostgreSQL ping failed: {}", e)))
            }
            AnyDatabase::MySQL(pool) => {
                // MySQL: SELECT 1
                sqlx::query("SELECT 1")
                    .fetch_one(pool)
                    .await
                    .map(|_| true)
                    .map_err(|e| Error::template(format!("MySQL ping failed: {}", e)))
            }
            AnyDatabase::SQLite(pool) => {
                // SQLite: SELECT 1
                sqlx::query("SELECT 1")
                    .fetch_one(pool)
                    .await
                    .map(|_| true)
                    .map_err(|e| Error::template(format!("SQLite ping failed: {}", e)))
            }
        }
    }

    /// Gracefully close all database connections
    ///
    /// This method should be called during application shutdown to ensure
    /// all database connections are properly closed and resources are released.
    ///
    /// # Examples
    /// ```rust
    /// // During application shutdown
    /// DB::shutdown().await?;
    /// ```
    pub async fn shutdown() -> Result<()> {
        // Shutdown registry databases
        if let Ok(registry) = Self::get_registry() {
            for name in registry.list_databases().await {
                log::info!("Closing database connection: {}", name);
            }
            registry.clear().await;
        }

        // Shutdown legacy DATABASE
        if let Some(db_option) = DATABASE.get() {
            if let Some(connection) = db_option.as_ref() {
                // First check if connection is still alive
                if let Ok(true) = Self::ping().await {
                    log::info!("Database connection is alive, proceeding with shutdown");
                }

                log::info!("Closing database connections...");

                match connection.as_ref() {
                    AnyDatabase::Postgres(pool) => {
                        pool.close().await;
                        log::info!("PostgreSQL connections closed");
                    }
                    AnyDatabase::MySQL(pool) => {
                        pool.close().await;
                        log::info!("MySQL connections closed");
                    }
                    AnyDatabase::SQLite(pool) => {
                        pool.close().await;
                        log::info!("SQLite connections closed");
                    }
                }
            }
        }
        Ok(())
    }

    /// Sanitize database URL for logging (hide password)
    fn sanitize_url(url: &str) -> String {
        if let Ok(parsed) = url::Url::parse(url) {
            let mut sanitized = parsed.clone();
            if sanitized.password().is_some() {
                let _ = sanitized.set_password(Some("***"));
            }
            sanitized.to_string()
        } else {
            "[invalid URL]".to_string()
        }
    }
}

/// Helper function to check database status for debugging
pub fn database_status() -> String {
    // Check registry first
    if let Ok(registry) = DB::get_registry() {
        let stats = futures::executor::block_on(registry.stats());
        if stats.total_databases > 0 {
            return format!(
                "Database: {} registered (default: {:?})",
                stats.total_databases, stats.default_database
            );
        }
    }

    // Fall back to legacy DATABASE
    if let Some(db) = DB::connection() {
        format!("Database: Connected ({:?})", db.backend())
    } else if DATABASE.get().is_some() {
        "Database: Configured but not connected".to_string()
    } else {
        "Database: Not configured".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_db_query_without_connection() {
        // Test behavior when database is not properly initialized
        // Note: We can't reset global state due to OnceCell limitations
        // This test checks error handling when no connection exists

        // If DATABASE is already initialized with None, query should return error
        if let Some(None) = DATABASE.get() {
            assert!(DB::query().is_err());
            assert!(DB::connection().is_none());
            assert!(!DB::is_initialized());
        }
        // If DATABASE has a connection, we can't test this scenario
        // This is a limitation of global state testing
    }

    #[tokio::test]
    async fn test_db_initialization_without_url() {
        // Test initialization with None URL
        // Note: This test may fail if DATABASE is already initialized

        match DB::init(None).await {
            Ok(_) => {
                // Successfully initialized with None
                assert!(!DB::is_initialized());
                assert!(DB::connection().is_none());
            }
            Err(_) => {
                // DATABASE was already initialized - this is expected in test environments
                // We can't reset OnceCell in tests, so this is acceptable
            }
        }

        // Query should return error when no URL provided
        assert!(DATABASE.get().is_some());
    }

    #[test]
    fn test_url_sanitization() {
        let url = "mysql://user:password@localhost/db";
        let sanitized = DB::sanitize_url(url);
        assert!(sanitized.contains("user:***"));
        assert!(!sanitized.contains("password"));
    }

    #[test]
    fn test_database_status() {
        let status = database_status();
        assert!(status.starts_with("Database:"));
    }
}
