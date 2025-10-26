//! SQLite database adapter implementation

use crate::database::adapter::{DatabaseAdapter, QueryResult};
use crate::database::types::{SqlValue, SqliteTypeConverter, TypeConverter};
use crate::error::{Error, Result};
use crate::models::query_builder::DatabaseBackend;
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use sqlx::SqlitePool;
use std::sync::Arc;

/// SQLite database adapter
#[derive(Clone)]
pub struct SqliteAdapter {
    name: String,
    pool: Arc<SqlitePool>,
    converter: SqliteTypeConverter,
}

impl SqliteAdapter {
    /// Create a new SQLite adapter
    pub async fn new(name: impl Into<String>, connection_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(connection_url)
            .await
            .map_err(|e| Error::template(format!("Failed to connect to SQLite: {}", e)))?;

        Ok(Self {
            name: name.into(),
            pool: Arc::new(pool),
            converter: SqliteTypeConverter::new(),
        })
    }

    /// Create adapter from existing pool
    pub fn from_pool(name: impl Into<String>, pool: SqlitePool) -> Self {
        Self {
            name: name.into(),
            pool: Arc::new(pool),
            converter: SqliteTypeConverter::new(),
        }
    }

    /// Get reference to the underlying pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Convert a SQLite row to JSON using the type converter
    fn row_to_json(&self, row: &sqlx::sqlite::SqliteRow) -> Result<JsonValue> {
        self.converter.row_to_json(row)
    }
}

#[async_trait]
impl DatabaseAdapter for SqliteAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn backend(&self) -> DatabaseBackend {
        DatabaseBackend::SQLite
    }

    async fn execute(&self, sql: &str, params: Vec<SqlValue>) -> Result<QueryResult> {
        // Log SQL in development mode
        #[cfg(debug_assertions)]
        {
            log::debug!("SQLite EXECUTE: {}", sql);
            log::debug!("  Parameters: {:?}", params);
        }

        let mut query = sqlx::query(sql);

        // Bind parameters using the converter
        for param in params {
            query = SqliteTypeConverter::bind_param(query, param);
        }

        let result = query
            .execute(&*self.pool)
            .await
            .map_err(|e| Error::template(format!("SQLite execute failed: {}", e)))?;

        Ok(QueryResult {
            rows_affected: result.rows_affected(),
            last_insert_id: Some(result.last_insert_rowid()),
        })
    }

    async fn fetch_all(&self, sql: &str, params: Vec<SqlValue>) -> Result<Vec<JsonValue>> {
        // Log SQL in development mode
        #[cfg(debug_assertions)]
        {
            log::debug!("SQLite FETCH_ALL: {}", sql);
            log::debug!("  Parameters: {:?}", params);
        }

        let mut query = sqlx::query(sql);

        // Bind parameters using the converter
        for param in params {
            query = SqliteTypeConverter::bind_param(query, param);
        }

        let rows = query
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| Error::template(format!("SQLite fetch_all failed: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(self.row_to_json(&row)?);
        }

        Ok(results)
    }

    async fn fetch_one(&self, sql: &str, params: Vec<SqlValue>) -> Result<Option<JsonValue>> {
        // Log SQL in development mode
        #[cfg(debug_assertions)]
        {
            log::debug!("SQLite FETCH_ONE: {}", sql);
            log::debug!("  Parameters: {:?}", params);
        }

        let mut query = sqlx::query(sql);

        // Bind parameters using the converter
        for param in params {
            query = SqliteTypeConverter::bind_param(query, param);
        }

        let row = query
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| Error::template(format!("SQLite fetch_one failed: {}", e)))?;

        match row {
            Some(row) => Ok(Some(self.row_to_json(&row)?)),
            None => Ok(None),
        }
    }

    async fn ping(&self) -> Result<bool> {
        sqlx::query("SELECT 1")
            .fetch_one(&*self.pool)
            .await
            .map(|_| true)
            .map_err(|e| Error::template(format!("SQLite ping failed: {}", e)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_box(&self) -> Box<dyn DatabaseAdapter> {
        Box::new(self.clone())
    }
}
