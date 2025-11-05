//! MySQL-specific type converter implementation

use super::converter::{DatabaseBackend, TypeConverter};
use super::value::SqlValue;
use crate::error::{Error, Result};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::mysql::MySqlRow;
use sqlx::{Column, Row, TypeInfo, ValueRef};

/// MySQL type converter
#[derive(Clone)]
pub struct MySqlTypeConverter;

impl Default for MySqlTypeConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl MySqlTypeConverter {
    /// Create a new MySQL type converter
    pub fn new() -> Self {
        MySqlTypeConverter
    }

    /// Extract a TINYINT value from MySQL (handles TINYINT(1) which may be returned as bool)
    fn extract_tinyint(row: &MySqlRow, index: usize) -> Result<SqlValue> {
        // First check if the value is NULL using raw value
        let raw_value = match row.try_get_raw(index) {
            Ok(v) => v,
            Err(e) => {
                return Err(Error::template(format!(
                    "Failed to get raw value for TINYINT at index {}: {}",
                    index, e
                )));
            }
        };

        if raw_value.is_null() {
            log::trace!("TINYINT at index {} is NULL", index);
            return Ok(SqlValue::Null);
        }

        // MySQL TINYINT(1) is often returned as bool by newer drivers
        // Try to decode as bool first, silently
        if let Ok(val) = row.try_get::<bool, _>(index) {
            log::trace!("TINYINT at index {} extracted as bool: {}", index, val);
            return Ok(SqlValue::TinyInt(if val { 1 } else { 0 }));
        }

        // Try as regular i8 TINYINT
        if let Ok(val) = row.try_get::<i8, _>(index) {
            log::trace!("TINYINT at index {} extracted as i8: {}", index, val);
            return Ok(SqlValue::TinyInt(val));
        }

        // If all else fails, log error and return a default
        log::warn!(
            "Could not extract TINYINT at index {} as bool or i8, defaulting to 0",
            index
        );
        Ok(SqlValue::TinyInt(0))
    }

    /// Extract a boolean value from MySQL (for actual BOOLEAN/BOOL types)
    fn extract_boolean(row: &MySqlRow, index: usize) -> Result<SqlValue> {
        // First check if the value is NULL using raw value
        if let Ok(raw_value) = row.try_get_raw(index) {
            if raw_value.is_null() {
                return Ok(SqlValue::Null);
            }
        }

        // Try native boolean first (most common for BOOLEAN type)
        if let Ok(val) = row.try_get::<bool, _>(index) {
            return Ok(SqlValue::Bool(val));
        }

        // Only try integer conversions if boolean extraction failed
        // This avoids the "invalid type: boolean `true`, expected i8" error
        // Try as i8 (TINYINT(1) might be used for boolean in some schemas)
        if let Ok(val) = row.try_get::<i8, _>(index) {
            return Ok(SqlValue::Bool(val != 0));
        }

        // Try as i32 for compatibility with some MySQL configurations
        if let Ok(val) = row.try_get::<i32, _>(index) {
            return Ok(SqlValue::Bool(val != 0));
        }

        // Default to false if we can't extract the value
        log::warn!(
            "Could not extract BOOLEAN at index {}, defaulting to false",
            index
        );
        Ok(SqlValue::Bool(false))
    }

    /// Extract a timestamp/datetime value from MySQL
    fn extract_datetime(
        row: &MySqlRow,
        index: usize,
        column: &sqlx::mysql::MySqlColumn,
    ) -> Result<SqlValue> {
        // First check if the value is NULL
        if row.try_get_raw(index).ok().is_some_and(|v| v.is_null()) {
            return Ok(SqlValue::Null);
        }

        // Debug log the actual type to help diagnose issues
        let type_info = column.type_info();
        let type_name = type_info.name();
        log::trace!(
            "Extracting datetime from column '{}' with MySQL type '{}'",
            column.name(),
            type_name
        );

        // Try as DateTime<Utc> directly (for TIMESTAMP fields)
        if let Ok(dt) = row.try_get::<DateTime<Utc>, _>(index) {
            return Ok(SqlValue::DateTime(dt.to_rfc3339()));
        }

        // Try with Option<DateTime<Utc>> for nullable TIMESTAMP
        if let Ok(Some(dt)) = row.try_get::<Option<DateTime<Utc>>, _>(index) {
            return Ok(SqlValue::DateTime(dt.to_rfc3339()));
        }

        // Try as i64 Unix timestamp (some MySQL configs return TIMESTAMP as seconds since epoch)
        if let Ok(timestamp) = row.try_get::<i64, _>(index) {
            if let Some(dt) = DateTime::from_timestamp(timestamp, 0) {
                return Ok(SqlValue::DateTime(dt.to_rfc3339()));
            }
        }

        // Try as string (MySQL often returns TIMESTAMP/DATETIME as string)
        if let Ok(s) = row.try_get::<String, _>(index) {
            // MySQL format: "2025-09-03 19:35:50"
            // Try to parse and convert to ISO 8601 format
            if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S") {
                let utc_dt = DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc);
                return Ok(SqlValue::DateTime(utc_dt.to_rfc3339()));
            }
            // Also try with microseconds: "2025-09-03 19:35:50.123456"
            if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S%.f") {
                let utc_dt = DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc);
                return Ok(SqlValue::DateTime(utc_dt.to_rfc3339()));
            }
            // If parsing fails, return the string as-is
            return Ok(SqlValue::DateTime(s));
        }

        // Try to get as NaiveDateTime (for DATETIME fields)
        if let Ok(dt) = row.try_get::<NaiveDateTime, _>(index) {
            // Convert to UTC DateTime for consistent ISO 8601 format
            let utc_dt = DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc);
            return Ok(SqlValue::DateTime(utc_dt.to_rfc3339()));
        }

        // Try with Option<NaiveDateTime> to handle nullable DATETIME
        if let Ok(Some(dt)) = row.try_get::<Option<NaiveDateTime>, _>(index) {
            let utc_dt = DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc);
            return Ok(SqlValue::DateTime(utc_dt.to_rfc3339()));
        }

        // If all extraction attempts fail, provide a more informative error
        Err(Error::template(format!(
            "Failed to extract datetime from column '{}' (MySQL type: '{}'). Value could not be converted to DateTime<Utc>, NaiveDateTime, i64 timestamp, or String",
            column.name(),
            type_name
        )))
    }

    /// Extract a date value from MySQL
    fn extract_date(
        row: &MySqlRow,
        index: usize,
        column: &sqlx::mysql::MySqlColumn,
    ) -> Result<SqlValue> {
        // Try to get as NaiveDate
        if let Ok(date) = row.try_get::<NaiveDate, _>(index) {
            return Ok(SqlValue::Date(date.to_string()));
        }

        // Fallback to string
        if let Ok(s) = row.try_get::<String, _>(index) {
            return Ok(SqlValue::Date(s));
        }

        Err(Error::template(format!(
            "Failed to extract date from column '{}'",
            column.name()
        )))
    }

    /// Extract a time value from MySQL
    fn extract_time(
        row: &MySqlRow,
        index: usize,
        column: &sqlx::mysql::MySqlColumn,
    ) -> Result<SqlValue> {
        // Try to get as NaiveTime
        if let Ok(time) = row.try_get::<NaiveTime, _>(index) {
            return Ok(SqlValue::String(time.to_string()));
        }

        // Fallback to string
        if let Ok(s) = row.try_get::<String, _>(index) {
            return Ok(SqlValue::String(s));
        }

        Err(Error::template(format!(
            "Failed to extract time from column '{}'",
            column.name()
        )))
    }

    /// Extract a binary value from MySQL
    fn extract_binary(
        row: &MySqlRow,
        index: usize,
        column: &sqlx::mysql::MySqlColumn,
    ) -> Result<SqlValue> {
        // Try to get as Vec<u8>
        if let Ok(bytes) = row.try_get::<Vec<u8>, _>(index) {
            return Ok(SqlValue::Bytes(bytes));
        }

        Err(Error::template(format!(
            "Failed to extract binary data from column '{}'",
            column.name()
        )))
    }

    /// Extract a decimal value from MySQL
    fn extract_decimal(
        row: &MySqlRow,
        index: usize,
        column: &sqlx::mysql::MySqlColumn,
    ) -> Result<SqlValue> {
        // Try native decimal first with the decimal feature
        #[cfg(feature = "decimal")]
        {
            if let Ok(d) = row.try_get::<rust_decimal::Decimal, _>(index) {
                return Ok(SqlValue::Decimal(d));
            }
        }

        // MySQL DECIMAL is usually returned as string to preserve precision
        if let Ok(s) = row.try_get::<String, _>(index) {
            #[cfg(feature = "decimal")]
            {
                if let Ok(d) = s.parse::<rust_decimal::Decimal>() {
                    return Ok(SqlValue::Decimal(d));
                }
            }
            #[cfg(not(feature = "decimal"))]
            return Ok(SqlValue::Decimal(s));
        }

        // Sometimes it might come as f64
        if let Ok(f) = row.try_get::<f64, _>(index) {
            #[cfg(feature = "decimal")]
            {
                if let Some(d) = rust_decimal::Decimal::from_f64_retain(f) {
                    return Ok(SqlValue::Decimal(d));
                }
            }
            #[cfg(not(feature = "decimal"))]
            return Ok(SqlValue::Decimal(f.to_string()));
        }

        Err(Error::template(format!(
            "Failed to extract decimal from column '{}'",
            column.name()
        )))
    }

    /// Bind a SqlValue to a MySQL query
    pub fn bind_param<'q>(
        query: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
        value: SqlValue,
    ) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
        match value {
            SqlValue::Null => query.bind(None::<Vec<u8>>), // MySQL accepts NULL for any type
            SqlValue::Bool(b) => query.bind(b),

            // Integer types
            SqlValue::TinyInt(i) => query.bind(i),
            SqlValue::SmallInt(i) => query.bind(i),
            SqlValue::Int(i) => query.bind(i),
            SqlValue::BigInt(i) => query.bind(i),

            // Unsigned integers - MySQL supports unsigned types natively
            SqlValue::UnsignedTinyInt(i) => query.bind(i),
            SqlValue::UnsignedSmallInt(i) => query.bind(i),
            SqlValue::UnsignedInt(i) => query.bind(i),
            SqlValue::UnsignedBigInt(i) => query.bind(i),

            // Floating point
            SqlValue::Float(f) => query.bind(f),
            SqlValue::Double(f) => query.bind(f),
            #[cfg(feature = "decimal")]
            SqlValue::Decimal(d) => query.bind(d),
            #[cfg(not(feature = "decimal"))]
            SqlValue::Decimal(s) => query.bind(s),

            // Text types
            SqlValue::String(s) | SqlValue::Text(s) => query.bind(s),

            // Binary
            SqlValue::Bytes(b) => query.bind(b),

            // Semantic types
            SqlValue::Enum(s) => query.bind(s),
            SqlValue::Uuid(s) => query.bind(s), // MySQL stores UUID as CHAR(36)
            SqlValue::Json(j) => query.bind(j),
            SqlValue::Date(s) => {
                // Try to parse as NaiveDate
                if let Ok(date) = NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
                    query.bind(date)
                } else {
                    query.bind(s)
                }
            }
            SqlValue::Time(s) => {
                // Try to parse as NaiveTime
                if let Ok(time) = NaiveTime::parse_from_str(&s, "%H:%M:%S") {
                    query.bind(time)
                } else if let Ok(time) = NaiveTime::parse_from_str(&s, "%H:%M:%S%.f") {
                    query.bind(time)
                } else {
                    query.bind(s)
                }
            }
            SqlValue::DateTime(s) => {
                // Try to parse as NaiveDateTime (MySQL doesn't have timezone info)
                if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
                    query.bind(dt.naive_utc())
                } else if let Ok(ndt) = NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S") {
                    query.bind(ndt)
                } else {
                    query.bind(s)
                }
            }
            SqlValue::Timestamp(ts) => {
                // Convert Unix timestamp to NaiveDateTime
                if let Some(dt) = DateTime::from_timestamp(ts, 0) {
                    query.bind(dt.naive_utc())
                } else {
                    query.bind(ts)
                }
            }
            SqlValue::Default => {
                // DEFAULT cannot be bound as a parameter
                panic!(
                    "SqlValue::Default should be handled in SQL generation, not parameter binding"
                )
            }
            SqlValue::Array(_) => {
                // MySQL doesn't have native array support like PostgreSQL
                // Would need to serialize to JSON or handle differently
                panic!("Array types are not supported in MySQL. Consider using JSON instead.")
            }

            // Network types - MySQL doesn't have native INET/CIDR types
            // Store as VARCHAR/CHAR
            SqlValue::Inet(ip) => query.bind(ip.to_string()),
            SqlValue::Cidr(ip, prefix) => query.bind(format!("{}/{}", ip, prefix)),
        }
    }
}

#[async_trait]
impl TypeConverter for MySqlTypeConverter {
    fn backend(&self) -> DatabaseBackend {
        DatabaseBackend::MySQL
    }

    fn sql_value_to_param(&self, _value: &SqlValue, _param_index: usize) -> Result<String> {
        // MySQL uses ? for parameters
        Ok("?".to_string())
    }

    fn extract_column_value(
        &self,
        row: &dyn std::any::Any,
        column_index: usize,
        column_name: &str,
        _column_type: &str,
    ) -> Result<SqlValue> {
        // Downcast to MySqlRow
        let mysql_row = row
            .downcast_ref::<MySqlRow>()
            .ok_or_else(|| Error::template("Invalid row type for MySQL converter".to_string()))?;

        // Get column information
        let columns = mysql_row.columns();
        let column = columns.get(column_index).ok_or_else(|| {
            Error::template(format!("Column index {} out of bounds", column_index))
        })?;

        // First, check if the value is NULL
        if mysql_row
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

        // Now we know the value is NOT NULL, so we can extract it based on type
        match type_name {
            "BOOLEAN" | "BOOL" => {
                // MySQL BOOLEAN is actually TINYINT(1), so extract as TINYINT
                // This avoids type mismatch errors with models expecting i8
                Self::extract_tinyint(mysql_row, column_index)
            }
            "TINYINT" => {
                // MySQL TINYINT - always return as TinyInt even if driver returns bool
                Self::extract_tinyint(mysql_row, column_index)
            }
            "SMALLINT" => {
                let val: i16 = mysql_row
                    .try_get(column_index)
                    .map_err(|e| Error::template(format!("Failed to extract smallint: {}", e)))?;
                Ok(SqlValue::SmallInt(val))
            }
            "MEDIUMINT" | "INT" | "INTEGER" => {
                let val: i32 = mysql_row
                    .try_get(column_index)
                    .map_err(|e| Error::template(format!("Failed to extract int: {}", e)))?;
                Ok(SqlValue::Int(val))
            }
            "BIGINT" => {
                let val: i64 = mysql_row
                    .try_get(column_index)
                    .map_err(|e| Error::template(format!("Failed to extract bigint: {}", e)))?;
                Ok(SqlValue::BigInt(val))
            }
            "FLOAT" => {
                let val: f32 = mysql_row
                    .try_get(column_index)
                    .map_err(|e| Error::template(format!("Failed to extract float: {}", e)))?;
                Ok(SqlValue::Float(val))
            }
            "DOUBLE" | "REAL" => {
                let val: f64 = mysql_row
                    .try_get(column_index)
                    .map_err(|e| Error::template(format!("Failed to extract double: {}", e)))?;
                Ok(SqlValue::Double(val))
            }
            "VARCHAR" | "CHAR" | "TEXT" | "TINYTEXT" | "MEDIUMTEXT" | "LONGTEXT" => {
                let val: String = mysql_row
                    .try_get(column_index)
                    .map_err(|e| Error::template(format!("Failed to extract string: {}", e)))?;
                Ok(SqlValue::String(val))
            }
            "JSON" => {
                let val: JsonValue = mysql_row
                    .try_get(column_index)
                    .map_err(|e| Error::template(format!("Failed to extract JSON: {}", e)))?;
                Ok(SqlValue::Json(val))
            }
            "DATE" => Self::extract_date(mysql_row, column_index, column),
            "TIME" => Self::extract_time(mysql_row, column_index, column),
            "DATETIME" | "TIMESTAMP" => Self::extract_datetime(mysql_row, column_index, column),
            "BINARY" | "VARBINARY" | "BLOB" | "TINYBLOB" | "MEDIUMBLOB" | "LONGBLOB" => {
                Self::extract_binary(mysql_row, column_index, column)
            }
            "DECIMAL" | "NUMERIC" => Self::extract_decimal(mysql_row, column_index, column),
            "YEAR" => {
                // MySQL YEAR type
                if let Ok(i) = mysql_row.try_get::<i16, _>(column_index) {
                    Ok(SqlValue::SmallInt(i))
                } else if let Ok(s) = mysql_row.try_get::<String, _>(column_index) {
                    Ok(SqlValue::String(s))
                } else {
                    Ok(SqlValue::Null)
                }
            }
            "BIT" => {
                // MySQL BIT type
                if let Ok(val) = mysql_row.try_get::<u64, _>(column_index) {
                    Ok(SqlValue::UnsignedBigInt(val))
                } else if let Ok(bytes) = mysql_row.try_get::<Vec<u8>, _>(column_index) {
                    // Convert bytes to u64
                    let mut val = 0u64;
                    for (i, &byte) in bytes.iter().enumerate().take(8) {
                        val |= (byte as u64) << (i * 8);
                    }
                    Ok(SqlValue::UnsignedBigInt(val))
                } else {
                    Ok(SqlValue::Null)
                }
            }
            "ENUM" | "SET" => {
                // MySQL ENUM and SET types
                if let Ok(s) = mysql_row.try_get::<String, _>(column_index) {
                    Ok(SqlValue::Enum(s))
                } else {
                    Ok(SqlValue::Null)
                }
            }
            _ => {
                // Check if it's an unsigned type
                if type_name.contains("UNSIGNED") {
                    if type_name.contains("TINYINT") {
                        let val: u8 = mysql_row.try_get(column_index).map_err(|e| {
                            Error::template(format!("Failed to extract unsigned tinyint: {}", e))
                        })?;
                        return Ok(SqlValue::UnsignedTinyInt(val));
                    } else if type_name.contains("SMALLINT") {
                        let val: u16 = mysql_row.try_get(column_index).map_err(|e| {
                            Error::template(format!("Failed to extract unsigned smallint: {}", e))
                        })?;
                        return Ok(SqlValue::UnsignedSmallInt(val));
                    } else if type_name.contains("INT") && !type_name.contains("BIGINT") {
                        let val: u32 = mysql_row.try_get(column_index).map_err(|e| {
                            Error::template(format!("Failed to extract unsigned int: {}", e))
                        })?;
                        return Ok(SqlValue::UnsignedInt(val));
                    } else if type_name.contains("BIGINT") {
                        let val: u64 = mysql_row.try_get(column_index).map_err(|e| {
                            Error::template(format!("Failed to extract unsigned bigint: {}", e))
                        })?;
                        return Ok(SqlValue::UnsignedBigInt(val));
                    }
                }

                // Try to extract as string for unknown types
                if let Ok(s) = mysql_row.try_get::<String, _>(column_index) {
                    Ok(SqlValue::String(s))
                } else {
                    log::warn!(
                        "Unknown MySQL type '{}' for column '{}'",
                        type_name,
                        column_name
                    );
                    Ok(SqlValue::Null)
                }
            }
        }
    }

    fn row_to_json(&self, row: &dyn std::any::Any) -> Result<JsonValue> {
        let mysql_row = row
            .downcast_ref::<MySqlRow>()
            .ok_or_else(|| Error::template("Invalid row type for MySQL converter".to_string()))?;

        let mut obj = serde_json::Map::new();

        for (i, column) in mysql_row.columns().iter().enumerate() {
            let name = column.name();
            let type_name = column.type_info().name();
            let value = self.extract_column_value(row, i, name, type_name)?;
            obj.insert(name.to_string(), value.to_json());
        }

        Ok(JsonValue::Object(obj))
    }

    fn is_null(&self, row: &dyn std::any::Any, column_index: usize) -> Result<bool> {
        let mysql_row = row
            .downcast_ref::<MySqlRow>()
            .ok_or_else(|| Error::template("Invalid row type for MySQL converter".to_string()))?;

        Ok(mysql_row
            .try_get_raw(column_index)
            .map(|raw| raw.is_null())
            .unwrap_or(true))
    }

    fn parameter_placeholder(&self, _index: usize) -> String {
        "?".to_string()
    }
}
