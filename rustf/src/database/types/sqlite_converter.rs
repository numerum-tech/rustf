//! SQLite-specific type converter implementation

use super::converter::{DatabaseBackend, TypeConverter};
use super::value::SqlValue;
use crate::error::{Error, Result};
use async_trait::async_trait;
use chrono::DateTime;
use serde_json::Value as JsonValue;
use sqlx::sqlite::SqliteRow;
use sqlx::{Column, Row, TypeInfo, ValueRef};

/// SQLite type converter
#[derive(Clone)]
pub struct SqliteTypeConverter;

impl Default for SqliteTypeConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl SqliteTypeConverter {
    /// Create a new SQLite type converter
    pub fn new() -> Self {
        SqliteTypeConverter
    }

    /// Determine the SQLite type affinity
    fn get_type_affinity(type_name: &str) -> SqliteAffinity {
        let upper = type_name.to_uppercase();

        // SQLite type affinity rules (https://www.sqlite.org/datatype3.html)
        if upper.contains("INT") {
            SqliteAffinity::Integer
        } else if upper.contains("CHAR") || upper.contains("CLOB") || upper.contains("TEXT") {
            SqliteAffinity::Text
        } else if upper.contains("BLOB") || upper.is_empty() {
            SqliteAffinity::Blob
        } else if upper.contains("REAL") || upper.contains("FLOA") || upper.contains("DOUB") {
            SqliteAffinity::Real
        } else {
            SqliteAffinity::Numeric
        }
    }

    /// Extract value based on SQLite's type affinity
    fn extract_by_affinity(
        row: &SqliteRow,
        index: usize,
        affinity: SqliteAffinity,
        column_name: &str,
    ) -> Result<SqlValue> {
        match affinity {
            SqliteAffinity::Integer => {
                // SQLite stores booleans as integers (0 or 1)
                if let Ok(val) = row.try_get::<i64, _>(index) {
                    // Check if this might be a boolean (common pattern)
                    if column_name.to_lowercase().contains("bool")
                        || column_name.to_lowercase().contains("is_")
                        || column_name.to_lowercase().contains("has_")
                        || column_name.to_lowercase().contains("active")
                    {
                        return Ok(SqlValue::Bool(val != 0));
                    }
                    // Return as appropriate integer type based on value range
                    if val >= i32::MIN as i64 && val <= i32::MAX as i64 {
                        Ok(SqlValue::Int(val as i32))
                    } else {
                        Ok(SqlValue::BigInt(val))
                    }
                } else {
                    Ok(SqlValue::Null)
                }
            }
            SqliteAffinity::Text => {
                if let Ok(val) = row.try_get::<String, _>(index) {
                    // Check if this is a JSON column
                    if column_name.to_lowercase().contains("json") {
                        if let Ok(json_val) = serde_json::from_str(&val) {
                            return Ok(SqlValue::Json(json_val));
                        }
                    }
                    // Check if this is a datetime/date column
                    if column_name.to_lowercase().contains("date")
                        || column_name.to_lowercase().contains("time")
                        || column_name.to_lowercase().contains("created")
                        || column_name.to_lowercase().contains("updated")
                    {
                        // Try to parse as datetime
                        if val.contains('T') || val.contains(' ') {
                            return Ok(SqlValue::DateTime(val));
                        } else if val.contains('-') && val.len() == 10 {
                            return Ok(SqlValue::Date(val));
                        }
                    }
                    // Check if this is a UUID
                    if column_name.to_lowercase().contains("uuid")
                        || column_name.to_lowercase().contains("guid")
                    {
                        return Ok(SqlValue::Uuid(val));
                    }
                    Ok(SqlValue::String(val))
                } else {
                    Ok(SqlValue::Null)
                }
            }
            SqliteAffinity::Real => {
                if let Ok(val) = row.try_get::<f64, _>(index) {
                    Ok(SqlValue::Double(val))
                } else {
                    Ok(SqlValue::Null)
                }
            }
            SqliteAffinity::Blob => {
                if let Ok(val) = row.try_get::<Vec<u8>, _>(index) {
                    Ok(SqlValue::Bytes(val))
                } else {
                    Ok(SqlValue::Null)
                }
            }
            SqliteAffinity::Numeric => {
                // NUMERIC affinity can store as INTEGER, REAL, or TEXT
                // Try in order of preference
                if let Ok(val) = row.try_get::<i64, _>(index) {
                    if val >= i32::MIN as i64 && val <= i32::MAX as i64 {
                        Ok(SqlValue::Int(val as i32))
                    } else {
                        Ok(SqlValue::BigInt(val))
                    }
                } else if let Ok(val) = row.try_get::<f64, _>(index) {
                    Ok(SqlValue::Double(val))
                } else if let Ok(val) = row.try_get::<String, _>(index) {
                    // Could be a decimal stored as text
                    #[cfg(feature = "decimal")]
                    {
                        if let Ok(d) = val.parse::<rust_decimal::Decimal>() {
                            Ok(SqlValue::Decimal(d))
                        } else {
                            // Not a valid decimal, return as string
                            Ok(SqlValue::String(val))
                        }
                    }
                    #[cfg(not(feature = "decimal"))]
                    Ok(SqlValue::Decimal(val))
                } else {
                    Ok(SqlValue::Null)
                }
            }
        }
    }

    /// Bind a SqlValue to a SQLite query
    pub fn bind_param<'q>(
        query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
        value: SqlValue,
    ) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
        match value {
            SqlValue::Null => query.bind(None::<i32>), // SQLite accepts NULL for any type
            SqlValue::Bool(b) => query.bind(if b { 1i32 } else { 0i32 }), // SQLite stores bools as integers

            // Integer types - SQLite stores all as INTEGER
            SqlValue::TinyInt(i) => query.bind(i as i32),
            SqlValue::SmallInt(i) => query.bind(i as i32),
            SqlValue::Int(i) => query.bind(i),
            SqlValue::BigInt(i) => query.bind(i),

            // Unsigned integers - convert to signed
            SqlValue::UnsignedTinyInt(i) => query.bind(i as i32),
            SqlValue::UnsignedSmallInt(i) => query.bind(i as i32),
            SqlValue::UnsignedInt(i) => query.bind(i as i64),
            SqlValue::UnsignedBigInt(i) => {
                // SQLite INTEGER can hold up to 8 bytes (signed)
                // For very large unsigned values, store as text
                if i > i64::MAX as u64 {
                    query.bind(i.to_string())
                } else {
                    query.bind(i as i64)
                }
            }

            // Floating point - SQLite stores as REAL
            SqlValue::Float(f) => query.bind(f as f64),
            SqlValue::Double(f) => query.bind(f),
            #[cfg(feature = "decimal")]
            SqlValue::Decimal(d) => query.bind(d.to_string()), // SQLite doesn't have native decimal, store as text
            #[cfg(not(feature = "decimal"))]
            SqlValue::Decimal(s) => query.bind(s), // Store decimal as text to preserve precision

            // Text types
            SqlValue::String(s) | SqlValue::Text(s) => query.bind(s),

            // Binary
            SqlValue::Bytes(b) => query.bind(b),

            // Semantic types - all stored as text in SQLite
            SqlValue::Enum(s) => query.bind(s),
            SqlValue::Uuid(s) => query.bind(s),
            SqlValue::Json(j) => query.bind(j.to_string()),
            SqlValue::Date(s) => query.bind(s),
            SqlValue::Time(s) => query.bind(s),
            SqlValue::DateTime(s) => query.bind(s),
            SqlValue::Timestamp(ts) => {
                // Store timestamp as ISO8601 string
                if let Some(dt) = DateTime::from_timestamp(ts, 0) {
                    query.bind(dt.to_rfc3339())
                } else {
                    query.bind(ts.to_string())
                }
            }
            SqlValue::Default => {
                // DEFAULT cannot be bound as a parameter
                panic!(
                    "SqlValue::Default should be handled in SQL generation, not parameter binding"
                )
            }
            SqlValue::Array(values) => {
                // SQLite doesn't have native array support
                // Store as JSON string
                let json_array =
                    serde_json::Value::Array(values.into_iter().map(|v| v.to_json()).collect());
                query.bind(json_array.to_string())
            }

            // Network types - SQLite doesn't have native INET/CIDR types
            // Store as TEXT
            SqlValue::Inet(ip) => query.bind(ip.to_string()),
            SqlValue::Cidr(ip, prefix) => query.bind(format!("{}/{}", ip, prefix)),
        }
    }
}

/// SQLite type affinity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SqliteAffinity {
    Text,
    Numeric,
    Integer,
    Real,
    Blob,
}

#[async_trait]
impl TypeConverter for SqliteTypeConverter {
    fn backend(&self) -> DatabaseBackend {
        DatabaseBackend::SQLite
    }

    fn sql_value_to_param(&self, _value: &SqlValue, _param_index: usize) -> Result<String> {
        // SQLite uses ? for parameters
        Ok("?".to_string())
    }

    fn extract_column_value(
        &self,
        row: &dyn std::any::Any,
        column_index: usize,
        column_name: &str,
        column_type: &str,
    ) -> Result<SqlValue> {
        // Downcast to SqliteRow
        let sqlite_row = row
            .downcast_ref::<SqliteRow>()
            .ok_or_else(|| Error::template("Invalid row type for SQLite converter".to_string()))?;

        // Get column information
        let columns = sqlite_row.columns();
        let column = columns.get(column_index).ok_or_else(|| {
            Error::template(format!("Column index {} out of bounds", column_index))
        })?;

        // First, check if the value is NULL
        if sqlite_row
            .try_get_raw(column_index)
            .map_err(|e| {
                Error::template(format!(
                    "Failed to get raw value at column {}: {}",
                    column_index, e
                ))
            })?
            .is_null()
        {
            return Ok(SqlValue::Null);
        }

        let type_info = column.type_info();
        let type_name = type_info.name();

        // SQLite uses type affinity, not strict types
        // We need to determine the best extraction method based on the declared type
        let affinity = Self::get_type_affinity(column_type);

        // Try to extract based on affinity
        Self::extract_by_affinity(sqlite_row, column_index, affinity, column_name)
    }

    fn row_to_json(&self, row: &dyn std::any::Any) -> Result<JsonValue> {
        let sqlite_row = row
            .downcast_ref::<SqliteRow>()
            .ok_or_else(|| Error::template("Invalid row type for SQLite converter".to_string()))?;

        let mut obj = serde_json::Map::new();

        for (i, column) in sqlite_row.columns().iter().enumerate() {
            let name = column.name();
            // SQLite doesn't always provide accurate type information
            // We use the column name and value inspection to determine the best type
            let type_name = column.type_info().name();
            let value = self.extract_column_value(row, i, name, type_name)?;
            obj.insert(name.to_string(), value.to_json());
        }

        Ok(JsonValue::Object(obj))
    }

    fn is_null(&self, row: &dyn std::any::Any, column_index: usize) -> Result<bool> {
        let sqlite_row = row
            .downcast_ref::<SqliteRow>()
            .ok_or_else(|| Error::template("Invalid row type for SQLite converter".to_string()))?;

        Ok(sqlite_row
            .try_get_raw(column_index)
            .map(|raw| raw.is_null())
            .unwrap_or(true))
    }

    fn parameter_placeholder(&self, _index: usize) -> String {
        "?".to_string()
    }

    fn boolean_to_sql(&self, value: bool) -> String {
        // SQLite stores booleans as integers
        if value { "1" } else { "0" }.to_string()
    }
}
