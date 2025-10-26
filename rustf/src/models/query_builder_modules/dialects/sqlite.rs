//! SQLite dialect implementation for RustF query builder
//!
//! This module contains SQLite-specific SQL generation logic,
//! handling SQLite's specific syntax and capabilities.

use super::SqlDialect;

/// SQLite dialect
pub struct SQLiteDialect;

impl SQLiteDialect {
    /// Create a new SQLite dialect
    pub fn new() -> Self {
        Self
    }
}

impl Default for SQLiteDialect {
    fn default() -> Self {
        Self::new()
    }
}

impl SqlDialect for SQLiteDialect {
    fn quote_identifier(&self, identifier: &str) -> String {
        // Check if this is a qualified column name (table.column)
        if identifier.contains('.') {
            // For qualified names, don't quote - SQLite handles them correctly
            identifier.to_string()
        } else {
            // For simple identifiers, quote with double quotes
            format!("\"{}\"", identifier.replace("\"", "\"\""))
        }
    }

    fn placeholder(&self, _position: usize) -> String {
        "?".to_string()
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
            "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT({}) DO UPDATE SET {}",
            table,
            columns.join(", "),
            vec!["?"; columns.len()].join(", "),
            conflict_columns.join(", "),
            columns
                .iter()
                .map(|col| format!("{} = excluded.{}", col, col))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn current_timestamp(&self) -> &'static str {
        "CURRENT_TIMESTAMP"
    }

    fn auto_increment_syntax(&self) -> &'static str {
        "INTEGER PRIMARY KEY AUTOINCREMENT"
    }

    fn boolean_type(&self) -> &'static str {
        "INTEGER"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
