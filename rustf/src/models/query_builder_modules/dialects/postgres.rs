//! PostgreSQL dialect implementation for RustF query builder
//!
//! This module contains PostgreSQL-specific SQL generation logic,
//! including support for PostgreSQL enums, arrays, JSONB, and other advanced features.

use super::SqlDialect;

/// PostgreSQL dialect with advanced feature support
pub struct PostgresDialect {
    /// Schema information for enum type resolution
    pub(crate) schema_context: Option<PostgresSchemaContext>,
}

/// PostgreSQL schema context for advanced type handling
pub struct PostgresSchemaContext {
    /// Map of table.column -> enum_type_name for automatic enum casting
    pub enum_fields: std::collections::HashMap<String, String>,
}

impl PostgresDialect {
    /// Create a new PostgreSQL dialect without schema context
    pub fn new() -> Self {
        Self {
            schema_context: None,
        }
    }

    /// Create a PostgreSQL dialect with schema context for advanced features
    pub fn with_schema_context(schema_context: PostgresSchemaContext) -> Self {
        Self {
            schema_context: Some(schema_context),
        }
    }

    /// Check if a field should be cast to an enum type
    pub fn get_enum_type(&self, table: &str, column: &str) -> Option<&String> {
        self.schema_context
            .as_ref()?
            .enum_fields
            .get(&format!("{}.{}", table, column))
    }

    /// Generate parameter placeholder with optional enum casting
    pub fn placeholder_with_enum_cast(&self, position: usize, enum_type: Option<&str>) -> String {
        match enum_type {
            Some(enum_name) => format!("${}::{}", position, enum_name),
            None => format!("${}", position),
        }
    }
}

impl Default for PostgresDialect {
    fn default() -> Self {
        Self::new()
    }
}

impl SqlDialect for PostgresDialect {
    fn quote_identifier(&self, identifier: &str) -> String {
        // Check if this is a qualified column name (table.column)
        if identifier.contains('.') {
            // For qualified names, don't quote - PostgreSQL handles them correctly
            // This allows "settlement_banks.is_active" to work properly
            identifier.to_string()
        } else {
            // For simple identifiers, quote as normal
            format!("\"{}\"", identifier.replace("\"", "\"\""))
        }
    }

    fn placeholder(&self, position: usize) -> String {
        format!("${}", position)
    }

    fn limit_syntax(&self, limit: Option<i64>, offset: Option<i64>) -> String {
        let mut sql = String::new();
        if let Some(limit) = limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }
        sql
    }

    fn returning_syntax(&self, columns: &[String]) -> Option<String> {
        Some(format!(" RETURNING {}", columns.join(", ")))
    }

    fn upsert_syntax(
        &self,
        table: &str,
        columns: &[String],
        conflict_columns: &[String],
    ) -> String {
        format!(
            "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ({}) DO UPDATE SET {}",
            table,
            columns.join(", "),
            (1..=columns.len())
                .map(|i| format!("${}", i))
                .collect::<Vec<_>>()
                .join(", "),
            conflict_columns.join(", "),
            columns
                .iter()
                .enumerate()
                .map(|(i, col)| format!("{} = ${}", col, i + 1))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn current_timestamp(&self) -> &'static str {
        "CURRENT_TIMESTAMP"
    }

    fn auto_increment_syntax(&self) -> &'static str {
        "SERIAL PRIMARY KEY"
    }

    fn boolean_type(&self) -> &'static str {
        "BOOLEAN"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
