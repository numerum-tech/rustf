//! MySQL/MariaDB dialect implementation for RustF query builder
//!
//! This module contains MySQL and MariaDB-specific SQL generation logic,
//! including handling of MySQL's specific syntax and limitations.

use super::SqlDialect;

/// MySQL/MariaDB dialect
pub struct MySQLDialect;

impl MySQLDialect {
    /// Create a new MySQL dialect
    pub fn new() -> Self {
        Self
    }
}

impl Default for MySQLDialect {
    fn default() -> Self {
        Self::new()
    }
}

impl SqlDialect for MySQLDialect {
    fn quote_identifier(&self, identifier: &str) -> String {
        // Check if this is a qualified column name (table.column)
        if identifier.contains('.') {
            // For qualified names, don't quote - MySQL handles them correctly
            identifier.to_string()
        } else {
            // For simple identifiers, quote with backticks
            format!("`{}`", identifier.replace("`", "``"))
        }
    }

    fn placeholder(&self, _position: usize) -> String {
        "?".to_string()
    }

    fn limit_syntax(&self, limit: Option<i64>, offset: Option<i64>) -> String {
        match (limit, offset) {
            (Some(limit), Some(offset)) => format!(" LIMIT {} OFFSET {}", limit, offset),
            (Some(limit), None) => format!(" LIMIT {}", limit),
            (None, Some(_)) => panic!("MySQL requires LIMIT when using OFFSET"),
            (None, None) => String::new(),
        }
    }

    fn returning_syntax(&self, _columns: &[String]) -> Option<String> {
        None // MySQL doesn't support RETURNING
    }

    fn upsert_syntax(
        &self,
        table: &str,
        columns: &[String],
        _conflict_columns: &[String],
    ) -> String {
        format!(
            "INSERT INTO {} ({}) VALUES ({}) ON DUPLICATE KEY UPDATE {}",
            table,
            columns.join(", "),
            vec!["?"; columns.len()].join(", "),
            columns
                .iter()
                .map(|col| format!("{} = VALUES({})", col, col))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn current_timestamp(&self) -> &'static str {
        "CURRENT_TIMESTAMP()"
    }

    fn auto_increment_syntax(&self) -> &'static str {
        "AUTO_INCREMENT PRIMARY KEY"
    }

    fn boolean_type(&self) -> &'static str {
        "TINYINT(1)"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
