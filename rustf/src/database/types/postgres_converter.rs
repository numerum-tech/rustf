//! PostgreSQL-specific type converter implementation

use super::converter::{DatabaseBackend, TypeConverter};
use super::value::SqlValue;
use crate::error::{Error, Result};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::postgres::PgRow;
use sqlx::{Column, Row, TypeInfo, ValueRef};

/// PostgreSQL type converter
#[derive(Clone)]
pub struct PostgresTypeConverter;

impl Default for PostgresTypeConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl PostgresTypeConverter {
    /// Create a new PostgreSQL type converter
    pub fn new() -> Self {
        PostgresTypeConverter
    }

    /// Extract a boolean value from PostgreSQL
    fn extract_boolean(
        row: &PgRow,
        index: usize,
        column: &sqlx::postgres::PgColumn,
    ) -> Result<SqlValue> {
        // PostgreSQL can return booleans in different formats:
        // - Native boolean type
        // - Character 't'/'f' (especially from certain queries/views)
        // - String "true"/"false"

        // Try native boolean first
        if let Ok(val) = row.try_get::<bool, _>(index) {
            return Ok(SqlValue::Bool(val));
        }

        // Try as single character (PostgreSQL sometimes returns 't' or 'f')
        if let Ok(ch) = row.try_get::<&str, _>(index) {
            match ch {
                "t" | "true" | "TRUE" | "1" | "yes" | "YES" | "y" | "Y" => {
                    return Ok(SqlValue::Bool(true));
                }
                "f" | "false" | "FALSE" | "0" | "no" | "NO" | "n" | "N" => {
                    return Ok(SqlValue::Bool(false));
                }
                _ => {
                    log::warn!(
                        "Unexpected boolean value '{}' for column '{}'",
                        ch,
                        column.name()
                    );
                }
            }
        }

        // Try as String
        if let Ok(s) = row.try_get::<String, _>(index) {
            match s.as_str() {
                "t" | "true" | "TRUE" | "1" | "yes" | "YES" | "y" | "Y" => {
                    return Ok(SqlValue::Bool(true));
                }
                "f" | "false" | "FALSE" | "0" | "no" | "NO" | "n" | "N" => {
                    return Ok(SqlValue::Bool(false));
                }
                _ => {
                    log::warn!(
                        "Unexpected boolean value '{}' for column '{}'",
                        s,
                        column.name()
                    );
                }
            }
        }

        // Try as i8/i16/i32 (sometimes booleans are stored as integers)
        if let Ok(i) = row.try_get::<i32, _>(index) {
            return Ok(SqlValue::Bool(i != 0));
        }

        // If all else fails, log error and return null
        log::error!(
            "Failed to extract boolean value for column '{}'",
            column.name()
        );
        Ok(SqlValue::Null)
    }

    /// Extract a timestamp value from PostgreSQL
    /// Note: This is only called for non-NULL values, NULL check happens in extract_column_value
    fn extract_timestamp(
        row: &PgRow,
        index: usize,
        column: &sqlx::postgres::PgColumn,
    ) -> Result<SqlValue> {
        // Try to get as DateTime<Utc> directly (for TIMESTAMPTZ)
        if let Ok(dt) = row.try_get::<DateTime<Utc>, _>(index) {
            return Ok(SqlValue::DateTime(dt.to_rfc3339()));
        }

        // Try without timezone info (for TIMESTAMP)
        if let Ok(ndt) = row.try_get::<NaiveDateTime, _>(index) {
            let dt = DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc);
            return Ok(SqlValue::DateTime(dt.to_rfc3339()));
        }

        // Try as string (fallback for unusual formats)
        if let Ok(s) = row.try_get::<String, _>(index) {
            return Ok(SqlValue::DateTime(s));
        }

        // Log what we're trying to extract for debugging
        log::error!(
            "Failed to extract timestamp from column '{}' (type: {}). \
             Unable to parse as DateTime<Utc>, NaiveDateTime, or String.",
            column.name(),
            column.type_info().name()
        );

        Err(Error::template(format!(
            "Failed to extract timestamp from column '{}' (type: {})",
            column.name(),
            column.type_info().name()
        )))
    }

    /// Extract a date value from PostgreSQL
    /// Note: This is only called for non-NULL values, NULL check happens in extract_column_value
    fn extract_date(
        row: &PgRow,
        index: usize,
        column: &sqlx::postgres::PgColumn,
    ) -> Result<SqlValue> {
        // Try to get as NaiveDate directly
        if let Ok(date) = row.try_get::<NaiveDate, _>(index) {
            return Ok(SqlValue::Date(date.to_string()));
        }

        // Fallback to string
        if let Ok(s) = row.try_get::<String, _>(index) {
            return Ok(SqlValue::Date(s));
        }

        Err(Error::template(format!(
            "Failed to extract date from column '{}' (type: {})",
            column.name(),
            column.type_info().name()
        )))
    }

    /// Extract a time value from PostgreSQL
    /// Note: This is only called for non-NULL values, NULL check happens in extract_column_value
    fn extract_time(
        row: &PgRow,
        index: usize,
        column: &sqlx::postgres::PgColumn,
    ) -> Result<SqlValue> {
        // Try to get as NaiveTime directly
        if let Ok(time) = row.try_get::<NaiveTime, _>(index) {
            return Ok(SqlValue::Time(time.to_string()));
        }

        // Fallback to string
        if let Ok(s) = row.try_get::<String, _>(index) {
            return Ok(SqlValue::String(s));
        }

        Err(Error::template(format!(
            "Failed to extract time from column '{}' (type: {})",
            column.name(),
            column.type_info().name()
        )))
    }

    /// Extract a UUID value from PostgreSQL
    fn extract_uuid(
        row: &PgRow,
        index: usize,
        column: &sqlx::postgres::PgColumn,
    ) -> Result<SqlValue> {
        // Try to get as sqlx::types::Uuid
        if let Ok(uuid) = row.try_get::<sqlx::types::Uuid, _>(index) {
            return Ok(SqlValue::Uuid(uuid.to_string()));
        }

        // Fallback to string
        if let Ok(s) = row.try_get::<String, _>(index) {
            return Ok(SqlValue::Uuid(s));
        }

        Err(Error::template(format!(
            "Failed to extract UUID from column '{}'",
            column.name()
        )))
    }

    /// Extract a binary value from PostgreSQL
    fn extract_binary(
        row: &PgRow,
        index: usize,
        column: &sqlx::postgres::PgColumn,
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

    /// Bind a SqlValue to a PostgreSQL query
    pub fn bind_param<'q>(
        query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
        value: SqlValue,
    ) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
        match value {
            SqlValue::Null => {
                // PostgreSQL requires typed NULLs. We use Option<Vec<u8>> (bytea type)
                // as it has good implicit casting behavior in PostgreSQL
                query.bind(None::<Vec<u8>>)
            }
            SqlValue::Default => {
                // DEFAULT cannot be bound as a parameter, it should be in SQL directly
                // This is a programming error if we get here
                panic!(
                    "SqlValue::Default should be handled in SQL generation, not parameter binding"
                )
            }
            SqlValue::Bool(b) => query.bind(b),

            // Integer types
            SqlValue::TinyInt(i) => query.bind(i as i16), // PostgreSQL doesn't have TINYINT, use SMALLINT
            SqlValue::SmallInt(i) => query.bind(i),
            SqlValue::Int(i) => query.bind(i),
            SqlValue::BigInt(i) => query.bind(i),

            // Unsigned integers - PostgreSQL doesn't have unsigned, upcast to larger signed type
            SqlValue::UnsignedTinyInt(i) => query.bind(i as i16),
            SqlValue::UnsignedSmallInt(i) => query.bind(i as i32),
            SqlValue::UnsignedInt(i) => query.bind(i as i64),
            SqlValue::UnsignedBigInt(i) => {
                // For very large unsigned values, we might overflow
                // Store as NUMERIC/DECIMAL string for safety
                query.bind(i.to_string())
            }

            // Floating point
            SqlValue::Float(f) => query.bind(f),
            SqlValue::Double(f) => query.bind(f),
            #[cfg(feature = "decimal")]
            SqlValue::Decimal(d) => query.bind(d),
            #[cfg(not(feature = "decimal"))]
            SqlValue::Decimal(s) => {
                // When decimal feature is disabled, try to parse the string to f64
                // PostgreSQL will handle the conversion
                if let Ok(parsed) = s.parse::<f64>() {
                    query.bind(parsed)
                } else {
                    query.bind(s)
                }
            }

            // Text types
            SqlValue::String(s) | SqlValue::Text(s) => query.bind(s),

            // Binary
            SqlValue::Bytes(b) => query.bind(b),

            // Semantic types
            SqlValue::Enum(s) => {
                // PostgreSQL enums with type casting: extract only the value part
                // The type cast (::enum_type) is added in the SQL by the query builder
                if s.contains("::") {
                    if let Some((value, _type_name)) = s.split_once("::") {
                        query.bind(value.to_string())
                    } else {
                        query.bind(s)
                    }
                } else {
                    query.bind(s)
                }
            }
            SqlValue::Uuid(s) => {
                // Try to parse as UUID, fallback to string
                if let Ok(uuid) = sqlx::types::Uuid::parse_str(&s) {
                    query.bind(uuid)
                } else {
                    query.bind(s)
                }
            }
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
                // Try to parse as NaiveTime (with or without fractional seconds)
                if let Ok(time) = NaiveTime::parse_from_str(&s, "%H:%M:%S") {
                    query.bind(time)
                } else if let Ok(time) = NaiveTime::parse_from_str(&s, "%H:%M:%S%.f") {
                    query.bind(time)
                } else if let Ok(time) = NaiveTime::parse_from_str(&s, "%H:%M:%S%.3f") {
                    query.bind(time)
                } else if let Ok(time) = NaiveTime::parse_from_str(&s, "%H:%M:%S%.6f") {
                    query.bind(time)
                } else {
                    // Last resort: try to bind as string
                    query.bind(s)
                }
            }
            SqlValue::DateTime(s) => {
                // Try to parse as DateTime
                if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
                    query.bind(dt.with_timezone(&Utc))
                } else if let Ok(ndt) = NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S") {
                    query.bind(DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
                } else {
                    query.bind(s)
                }
            }
            SqlValue::Timestamp(ts) => {
                // Convert Unix timestamp to DateTime
                if let Some(dt) = DateTime::from_timestamp(ts, 0) {
                    query.bind(dt)
                } else {
                    query.bind(ts)
                }
            }
            SqlValue::Array(values) => {
                // PostgreSQL arrays need special handling
                // For now, convert to Vec of appropriate type if all elements are same type
                // This is a simplified implementation - full support would need type checking
                if values.is_empty() {
                    query.bind(Vec::<String>::new())
                } else {
                    // Try to bind as string array (most common case)
                    let string_array: Vec<String> = values
                        .iter()
                        .map(|v| match v {
                            SqlValue::String(s) | SqlValue::Text(s) => s.clone(),
                            SqlValue::Null => String::new(),
                            other => other.to_string(),
                        })
                        .collect();
                    query.bind(string_array)
                }
            }

            // Network types - PostgreSQL has native INET and CIDR types
            // Use ipnetwork::IpNetwork for proper SQLx binding
            SqlValue::Inet(ip) => {
                // Convert IpAddr to IpNetwork with /32 or /128 prefix for single hosts
                use ipnetwork::IpNetwork;
                let network = match ip {
                    std::net::IpAddr::V4(v4) => {
                        IpNetwork::V4(ipnetwork::Ipv4Network::new(v4, 32).expect("Valid IPv4"))
                    }
                    std::net::IpAddr::V6(v6) => {
                        IpNetwork::V6(ipnetwork::Ipv6Network::new(v6, 128).expect("Valid IPv6"))
                    }
                };
                query.bind(network)
            }
            SqlValue::Cidr(ip, prefix) => {
                // Convert to IpNetwork with the specified prefix
                use ipnetwork::IpNetwork;
                let network = match ip {
                    std::net::IpAddr::V4(v4) => {
                        IpNetwork::V4(ipnetwork::Ipv4Network::new(v4, prefix).expect("Valid CIDR"))
                    }
                    std::net::IpAddr::V6(v6) => {
                        IpNetwork::V6(ipnetwork::Ipv6Network::new(v6, prefix).expect("Valid CIDR"))
                    }
                };
                query.bind(network)
            }
        }
    }
}

#[async_trait]
impl TypeConverter for PostgresTypeConverter {
    fn backend(&self) -> DatabaseBackend {
        DatabaseBackend::Postgres
    }

    fn sql_value_to_param(&self, _value: &SqlValue, param_index: usize) -> Result<String> {
        // PostgreSQL uses $1, $2, etc. for parameters
        // But we return the value representation for now
        // The actual binding happens in bind_param
        Ok(format!("${}", param_index))
    }

    fn extract_column_value(
        &self,
        row: &dyn std::any::Any,
        column_index: usize,
        column_name: &str,
        _column_type: &str,
    ) -> Result<SqlValue> {
        // Downcast to PgRow
        let pg_row = row.downcast_ref::<PgRow>().ok_or_else(|| {
            Error::template("Invalid row type for PostgreSQL converter".to_string())
        })?;

        // Get column information
        let columns = pg_row.columns();
        let column = columns.get(column_index).ok_or_else(|| {
            Error::template(format!("Column index {} out of bounds", column_index))
        })?;

        let type_info = column.type_info();
        let type_name = type_info.name();

        // First, check if the value is NULL
        if pg_row
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

        // Now we know the value is NOT NULL, so we can extract it based on type
        match type_name {
            "BOOL" => Self::extract_boolean(pg_row, column_index, column),
            "INT2" | "INT4" => {
                let val: i32 = pg_row
                    .try_get(column_index)
                    .map_err(|e| Error::template(format!("Failed to extract int: {}", e)))?;
                Ok(SqlValue::Int(val))
            }
            "INT8" => {
                let val: i64 = pg_row
                    .try_get(column_index)
                    .map_err(|e| Error::template(format!("Failed to extract bigint: {}", e)))?;
                Ok(SqlValue::BigInt(val))
            }
            "FLOAT4" => {
                let val: f32 = pg_row
                    .try_get(column_index)
                    .map_err(|e| Error::template(format!("Failed to extract float: {}", e)))?;
                Ok(SqlValue::Float(val))
            }
            "FLOAT8" => {
                let val: f64 = pg_row
                    .try_get(column_index)
                    .map_err(|e| Error::template(format!("Failed to extract double: {}", e)))?;
                Ok(SqlValue::Double(val))
            }
            "TEXT" | "VARCHAR" | "CHAR" | "BPCHAR" | "NAME" => {
                let val: String = pg_row
                    .try_get(column_index)
                    .map_err(|e| Error::template(format!("Failed to extract string: {}", e)))?;
                Ok(SqlValue::String(val))
            }
            "JSON" | "JSONB" => {
                let val: JsonValue = pg_row
                    .try_get(column_index)
                    .map_err(|e| Error::template(format!("Failed to extract JSON: {}", e)))?;
                Ok(SqlValue::Json(val))
            }
            "TIMESTAMP" | "TIMESTAMPTZ" => Self::extract_timestamp(pg_row, column_index, column),
            "DATE" => Self::extract_date(pg_row, column_index, column),
            "TIME" | "TIMETZ" => Self::extract_time(pg_row, column_index, column),
            "UUID" => Self::extract_uuid(pg_row, column_index, column),
            "BYTEA" => Self::extract_binary(pg_row, column_index, column),
            "INET" | "CIDR" => {
                // PostgreSQL INET and CIDR types
                use ipnetwork::IpNetwork;
                let network: IpNetwork = pg_row
                    .try_get(column_index)
                    .map_err(|e| Error::template(format!("Failed to extract INET/CIDR: {}", e)))?;

                // Convert IpNetwork to SqlValue based on prefix
                match network {
                    IpNetwork::V4(v4) => {
                        if v4.prefix() == 32 {
                            // Single host, use INET
                            Ok(SqlValue::Inet(std::net::IpAddr::V4(v4.ip())))
                        } else {
                            // Network range, use CIDR
                            Ok(SqlValue::Cidr(std::net::IpAddr::V4(v4.ip()), v4.prefix()))
                        }
                    }
                    IpNetwork::V6(v6) => {
                        if v6.prefix() == 128 {
                            // Single host, use INET
                            Ok(SqlValue::Inet(std::net::IpAddr::V6(v6.ip())))
                        } else {
                            // Network range, use CIDR
                            Ok(SqlValue::Cidr(std::net::IpAddr::V6(v6.ip()), v6.prefix()))
                        }
                    }
                }
            }
            "NUMERIC" | "DECIMAL" => {
                // Try to get as Decimal first with the decimal feature
                #[cfg(feature = "decimal")]
                {
                    if let Ok(d) = pg_row.try_get::<rust_decimal::Decimal, _>(column_index) {
                        return Ok(SqlValue::Decimal(d));
                    }
                }

                // Fallback to string representation
                if let Ok(s) = pg_row.try_get::<String, _>(column_index) {
                    #[cfg(feature = "decimal")]
                    {
                        // Try to parse the string to Decimal
                        if let Ok(d) = s.parse::<rust_decimal::Decimal>() {
                            return Ok(SqlValue::Decimal(d));
                        }
                    }
                    #[cfg(not(feature = "decimal"))]
                    return Ok(SqlValue::Decimal(s));

                    #[cfg(feature = "decimal")]
                    return Err(Error::template(format!(
                        "Failed to parse decimal value: {}",
                        s
                    )));
                } else if let Ok(f) = pg_row.try_get::<f64, _>(column_index) {
                    #[cfg(feature = "decimal")]
                    {
                        // Convert f64 to Decimal
                        if let Some(d) = rust_decimal::Decimal::from_f64_retain(f) {
                            return Ok(SqlValue::Decimal(d));
                        }
                    }
                    #[cfg(not(feature = "decimal"))]
                    return Ok(SqlValue::Decimal(f.to_string()));

                    #[cfg(feature = "decimal")]
                    return Err(Error::template(format!(
                        "Failed to convert f64 {} to Decimal",
                        f
                    )));
                } else {
                    Err(Error::template(format!(
                        "Failed to extract numeric from column '{}'",
                        column_name
                    )))
                }
            }
            _ => {
                // For unknown types (including custom enums and arrays), we need special handling
                // PostgreSQL custom enum types will have names like "user_role", "currency", etc.

                // First, check if this is an array type
                let is_array = type_name.starts_with("_") || type_name.ends_with("[]");

                if is_array {
                    log::debug!(
                        "Attempting to extract PostgreSQL array type '{}' from column '{}'",
                        type_name,
                        column_name
                    );

                    // Try to extract as Vec<String> (most common case for arrays)
                    if let Ok(val) = pg_row.try_get::<Vec<String>, _>(column_index) {
                        let array_values: Vec<SqlValue> =
                            val.into_iter().map(SqlValue::String).collect();
                        return Ok(SqlValue::Array(array_values));
                    }

                    // Try as Vec<i32> for integer arrays
                    if let Ok(val) = pg_row.try_get::<Vec<i32>, _>(column_index) {
                        let array_values: Vec<SqlValue> =
                            val.into_iter().map(SqlValue::Int).collect();
                        return Ok(SqlValue::Array(array_values));
                    }

                    // Try as Vec<i64> for bigint arrays
                    if let Ok(val) = pg_row.try_get::<Vec<i64>, _>(column_index) {
                        let array_values: Vec<SqlValue> =
                            val.into_iter().map(SqlValue::BigInt).collect();
                        return Ok(SqlValue::Array(array_values));
                    }

                    // Try as Vec<f64> for float arrays
                    if let Ok(val) = pg_row.try_get::<Vec<f64>, _>(column_index) {
                        let array_values: Vec<SqlValue> =
                            val.into_iter().map(SqlValue::Double).collect();
                        return Ok(SqlValue::Array(array_values));
                    }

                    // Try as Vec<bool> for boolean arrays
                    if let Ok(val) = pg_row.try_get::<Vec<bool>, _>(column_index) {
                        let array_values: Vec<SqlValue> =
                            val.into_iter().map(SqlValue::Bool).collect();
                        return Ok(SqlValue::Array(array_values));
                    }

                    // Try as Vec<Uuid> for UUID arrays
                    if let Ok(val) = pg_row.try_get::<Vec<sqlx::types::Uuid>, _>(column_index) {
                        let array_values: Vec<SqlValue> = val
                            .into_iter()
                            .map(|uuid| SqlValue::Uuid(uuid.to_string()))
                            .collect();
                        return Ok(SqlValue::Array(array_values));
                    }

                    // Try as Vec<chrono::NaiveDate> for date arrays
                    if let Ok(val) = pg_row.try_get::<Vec<chrono::NaiveDate>, _>(column_index) {
                        let array_values: Vec<SqlValue> = val
                            .into_iter()
                            .map(|date| SqlValue::Date(date.to_string()))
                            .collect();
                        return Ok(SqlValue::Array(array_values));
                    }

                    // Try as Vec<chrono::NaiveTime> for time arrays
                    if let Ok(val) = pg_row.try_get::<Vec<chrono::NaiveTime>, _>(column_index) {
                        let array_values: Vec<SqlValue> = val
                            .into_iter()
                            .map(|time| SqlValue::Time(time.to_string()))
                            .collect();
                        return Ok(SqlValue::Array(array_values));
                    }

                    // Try as Vec<chrono::DateTime<chrono::Utc>> for timestamp arrays
                    if let Ok(val) =
                        pg_row.try_get::<Vec<chrono::DateTime<chrono::Utc>>, _>(column_index)
                    {
                        let array_values: Vec<SqlValue> = val
                            .into_iter()
                            .map(|dt| SqlValue::Timestamp(dt.timestamp()))
                            .collect();
                        return Ok(SqlValue::Array(array_values));
                    }

                    // Try as Vec<rust_decimal::Decimal> for decimal arrays
                    #[cfg(feature = "decimal")]
                    if let Ok(val) = pg_row.try_get::<Vec<rust_decimal::Decimal>, _>(column_index) {
                        let array_values: Vec<SqlValue> =
                            val.into_iter().map(SqlValue::Decimal).collect();
                        return Ok(SqlValue::Array(array_values));
                    }

                    // Try as Vec<Vec<u8>> for bytea arrays
                    if let Ok(val) = pg_row.try_get::<Vec<Vec<u8>>, _>(column_index) {
                        let array_values: Vec<SqlValue> =
                            val.into_iter().map(SqlValue::Bytes).collect();
                        return Ok(SqlValue::Array(array_values));
                    }

                    // For enum arrays or other custom types, try JSON representation
                    if let Ok(val) = pg_row.try_get::<serde_json::Value, _>(column_index) {
                        if let serde_json::Value::Array(arr) = val {
                            let array_values: Vec<SqlValue> = arr
                                .into_iter()
                                .map(|v| match v {
                                    serde_json::Value::String(s) => SqlValue::String(s),
                                    serde_json::Value::Number(n) => {
                                        if let Some(i) = n.as_i64() {
                                            SqlValue::BigInt(i)
                                        } else if let Some(f) = n.as_f64() {
                                            SqlValue::Double(f)
                                        } else {
                                            SqlValue::String(n.to_string())
                                        }
                                    }
                                    serde_json::Value::Bool(b) => SqlValue::Bool(b),
                                    serde_json::Value::Null => SqlValue::Null,
                                    _ => SqlValue::String(v.to_string()),
                                })
                                .collect();
                            return Ok(SqlValue::Array(array_values));
                        }
                    }

                    log::warn!(
                        "Could not extract PostgreSQL array type '{}' from column '{}'",
                        type_name,
                        column_name
                    );
                }

                // Check if this might be a custom enum type (non-array)
                let is_likely_enum = !is_array &&
                                    !type_name.contains("(") &&      // not a composite type
                                    type_name.chars().all(|c| c.is_alphanumeric() || c == '_'); // valid identifier

                if is_likely_enum {
                    // Try multiple extraction methods
                    // Method 1: Direct string extraction (sometimes works)
                    if let Ok(val) = pg_row.try_get::<String, _>(column_index) {
                        return Ok(SqlValue::Enum(val));
                    }

                    // Method 2: Try &str
                    if let Ok(val) = pg_row.try_get::<&str, _>(column_index) {
                        return Ok(SqlValue::Enum(val.to_string()));
                    }

                    // Method 3: Use raw decode with text assumption
                    use sqlx::Decode;
                    let value_ref = pg_row.try_get_raw(column_index).unwrap();

                    // Try to decode as text - this should work for enums
                    if let Ok(text_val) = <&str as Decode<'_, sqlx::Postgres>>::decode(value_ref) {
                        return Ok(SqlValue::Enum(text_val.to_string()));
                    }

                    log::warn!("Could not extract PostgreSQL enum type '{}' from column '{}' - treating as error",
                             type_name, column_name);
                }

                // For non-enum unknown types, try standard conversions
                // Try as string (most common fallback)
                if let Ok(val) = pg_row.try_get::<String, _>(column_index) {
                    return Ok(SqlValue::String(val));
                }

                // Try as &str
                if let Ok(val) = pg_row.try_get::<&str, _>(column_index) {
                    return Ok(SqlValue::String(val.to_string()));
                }

                // If all extraction attempts fail, return an error
                log::error!(
                    "Failed to extract value for column '{}' with type '{}'",
                    column_name,
                    type_name
                );
                Err(Error::template(format!(
                    "Unsupported PostgreSQL type '{}' for column '{}'",
                    type_name, column_name
                )))
            }
        }
    }

    fn row_to_json(&self, row: &dyn std::any::Any) -> Result<JsonValue> {
        let pg_row = row.downcast_ref::<PgRow>().ok_or_else(|| {
            Error::template("Invalid row type for PostgreSQL converter".to_string())
        })?;

        let mut obj = serde_json::Map::new();

        for (i, column) in pg_row.columns().iter().enumerate() {
            let name = column.name();
            let type_name = column.type_info().name();
            let value = self.extract_column_value(row, i, name, type_name)?;
            let json_value = value.to_json();

            // Debug log the conversion for troubleshooting
            obj.insert(name.to_string(), json_value);
        }

        Ok(JsonValue::Object(obj))
    }

    fn is_null(&self, row: &dyn std::any::Any, column_index: usize) -> Result<bool> {
        let pg_row = row.downcast_ref::<PgRow>().ok_or_else(|| {
            Error::template("Invalid row type for PostgreSQL converter".to_string())
        })?;

        Ok(pg_row
            .try_get_raw(column_index)
            .map(|raw| raw.is_null())
            .unwrap_or(true))
    }

    fn parameter_placeholder(&self, index: usize) -> String {
        format!("${}", index)
    }
}
