//! PostgreSQL database adapter implementation

use crate::database::adapter::{DatabaseAdapter, QueryResult};
use crate::database::types::{PostgresTypeConverter, SqlValue, TypeConverter};
use crate::error::{Error, Result};
use crate::models::query_builder::DatabaseBackend;
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use std::sync::Arc;

/// PostgreSQL database adapter
#[derive(Clone)]
pub struct PostgresAdapter {
    name: String,
    pool: Arc<PgPool>,
    converter: PostgresTypeConverter,
}

impl PostgresAdapter {
    /// Create a new PostgreSQL adapter
    pub async fn new(name: impl Into<String>, connection_url: &str) -> Result<Self> {
        let pool = PgPool::connect(connection_url)
            .await
            .map_err(|e| Error::template(format!("Failed to connect to PostgreSQL: {}", e)))?;

        Ok(Self {
            name: name.into(),
            pool: Arc::new(pool),
            converter: PostgresTypeConverter::new(),
        })
    }

    /// Create adapter from existing pool
    pub fn from_pool(name: impl Into<String>, pool: PgPool) -> Self {
        Self {
            name: name.into(),
            pool: Arc::new(pool),
            converter: PostgresTypeConverter::new(),
        }
    }

    /// Get reference to the underlying pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Convert a PostgreSQL row to JSON using the type converter
    fn row_to_json(&self, row: &sqlx::postgres::PgRow) -> Result<JsonValue> {
        self.converter.row_to_json(row)
    }
}

#[async_trait]
impl DatabaseAdapter for PostgresAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn backend(&self) -> DatabaseBackend {
        DatabaseBackend::Postgres
    }

    async fn execute(&self, sql: &str, params: Vec<SqlValue>) -> Result<QueryResult> {
        let mut query = sqlx::query(sql);

        // Bind parameters using the converter
        for param in params {
            query = PostgresTypeConverter::bind_param(query, param);
        }

        let result = query
            .execute(&*self.pool)
            .await
            .map_err(|e| Error::template(format!("PostgreSQL execute failed: {}", e)))?;

        Ok(QueryResult {
            rows_affected: result.rows_affected(),
            last_insert_id: None, // PostgreSQL doesn't have last_insert_id like MySQL
        })
    }

    async fn fetch_all(&self, sql: &str, params: Vec<SqlValue>) -> Result<Vec<JsonValue>> {
        let mut query = sqlx::query(sql);

        // Bind parameters using the converter
        for param in params {
            query = PostgresTypeConverter::bind_param(query, param);
        }

        let rows = query
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| Error::template(format!("PostgreSQL fetch_all failed: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(self.row_to_json(&row)?);
        }

        Ok(results)
    }

    async fn fetch_one(&self, sql: &str, params: Vec<SqlValue>) -> Result<Option<JsonValue>> {
        let mut query = sqlx::query(sql);

        // Bind parameters using the converter
        for param in params {
            query = PostgresTypeConverter::bind_param(query, param);
        }

        let row = query
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| Error::template(format!("PostgreSQL fetch_one failed: {}", e)))?;

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
            .map_err(|e| Error::template(format!("PostgreSQL ping failed: {}", e)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_box(&self) -> Box<dyn DatabaseAdapter> {
        Box::new(self.clone())
    }
}
