//! Type conversion trait and base utilities
//!
//! This module defines the core TypeConverter trait that all database-specific
//! converters must implement, providing a unified interface for type conversions.

use super::value::SqlValue;
use crate::error::{Error, Result};
use async_trait::async_trait;
use serde_json::Value as JsonValue;

/// Database backend identifier for type conversion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseBackend {
    Postgres,
    MySQL,
    MariaDB,
    SQLite,
}

/// Trait for database-specific type conversion
///
/// Each database adapter implements this trait to handle its specific
/// type conversion requirements while maintaining a unified interface.
#[async_trait]
pub trait TypeConverter: Send + Sync {
    /// Get the database backend this converter is for
    fn backend(&self) -> DatabaseBackend;

    /// Convert a SqlValue to a database-specific parameter for binding
    ///
    /// This is used when preparing parameters for SQL queries.
    fn sql_value_to_param(&self, value: &SqlValue, param_index: usize) -> Result<String>;

    /// Extract a column value from a database row and convert to SqlValue
    ///
    /// This is the main method for converting database-specific types to our
    /// unified SqlValue representation.
    fn extract_column_value(
        &self,
        row: &dyn std::any::Any,
        column_index: usize,
        column_name: &str,
        column_type: &str,
    ) -> Result<SqlValue>;

    /// Convert a row to JSON representation
    ///
    /// This method should iterate through all columns and build a JSON object.
    fn row_to_json(&self, row: &dyn std::any::Any) -> Result<JsonValue>;

    /// Check if a value is NULL in the database-specific way
    fn is_null(&self, row: &dyn std::any::Any, column_index: usize) -> Result<bool>;

    /// Get the SQL representation for a NULL value with proper type casting if needed
    fn null_representation(&self, _target_type: Option<&str>) -> String {
        "NULL".to_string()
    }

    /// Convert boolean value to database-specific representation
    fn boolean_to_sql(&self, value: bool) -> String {
        if value { "TRUE" } else { "FALSE" }.to_string()
    }

    /// Get the parameter placeholder for this database (e.g., $1 for Postgres, ? for MySQL)
    fn parameter_placeholder(&self, index: usize) -> String;
}

/// Helper struct to store column metadata during conversion
#[derive(Debug, Clone)]
pub struct ColumnMetadata {
    pub name: String,
    pub type_name: String,
    pub nullable: bool,
    pub position: usize,
}

/// Common conversion utilities used by all database converters
pub struct ConversionUtils;

impl ConversionUtils {
    /// Parse a boolean from various string representations
    pub fn parse_bool_string(s: &str) -> Option<bool> {
        match s.to_lowercase().as_str() {
            "true" | "t" | "yes" | "y" | "1" => Some(true),
            "false" | "f" | "no" | "n" | "0" => Some(false),
            _ => None,
        }
    }

    /// Convert a timestamp string to Unix timestamp
    pub fn parse_timestamp(s: &str) -> Result<i64> {
        use chrono::{DateTime, NaiveDateTime, Utc};

        // Try parsing as RFC3339/ISO8601
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Ok(dt.timestamp());
        }

        // Try parsing as naive datetime and assume UTC
        if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
            return Ok(DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc).timestamp());
        }

        // Try parsing as date only
        if let Ok(ndt) =
            NaiveDateTime::parse_from_str(&format!("{} 00:00:00", s), "%Y-%m-%d %H:%M:%S")
        {
            return Ok(DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc).timestamp());
        }

        Err(Error::template(format!("Failed to parse timestamp: {}", s)))
    }

    /// Format a Unix timestamp as ISO8601 string
    pub fn format_timestamp(timestamp: i64) -> String {
        use chrono::{DateTime, Utc};
        DateTime::from_timestamp(timestamp, 0)
            .map(|dt: DateTime<Utc>| dt.to_rfc3339())
            .unwrap_or_else(|| format!("{}", timestamp))
    }

    /// Escape a string for SQL
    pub fn escape_sql_string(s: &str) -> String {
        s.replace('\'', "''")
    }

    /// Convert bytes to hex string
    pub fn bytes_to_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02X}", b)).collect()
    }

    /// Convert hex string to bytes
    pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>> {
        let hex = hex.trim_start_matches("0x").trim_start_matches("0X");

        if hex.len() % 2 != 0 {
            return Err(Error::template(
                "Invalid hex string: odd length".to_string(),
            ));
        }

        (0..hex.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&hex[i..i + 2], 16)
                    .map_err(|e| Error::template(format!("Invalid hex string: {}", e)))
            })
            .collect()
    }
}
