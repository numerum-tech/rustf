//! Unified SQL value type for all database operations
//!
//! This module provides the single source of truth for SQL values
//! across all database adapters and the query builder.

use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fmt;
use std::net::IpAddr;

/// Generic SQL value type for parameter binding and result extraction
///
/// This enum represents all possible SQL data types across PostgreSQL,
/// MySQL, and SQLite, providing a unified interface for type handling.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SqlValue {
    // Null
    Null,

    // Default - for INSERT statements to use column's DEFAULT value
    Default,

    // Boolean
    Bool(bool),

    // Integer variants (for precise type mapping)
    TinyInt(i8),   // -128 to 127
    SmallInt(i16), // -32,768 to 32,767
    Int(i32),      // -2,147,483,648 to 2,147,483,647
    BigInt(i64),   // -9,223,372,036,854,775,808 to 9,223,372,036,854,775,807

    // Unsigned integers (important for MySQL)
    UnsignedTinyInt(u8),   // 0 to 255
    UnsignedSmallInt(u16), // 0 to 65,535
    UnsignedInt(u32),      // 0 to 4,294,967,295
    UnsignedBigInt(u64),   // 0 to 18,446,744,073,709,551,615

    // Floating point
    Float(f32),  // Single precision
    Double(f64), // Double precision
    #[cfg(feature = "decimal")]
    Decimal(rust_decimal::Decimal), // Arbitrary precision decimal
    #[cfg(not(feature = "decimal"))]
    Decimal(String), // Fallback to string when decimal feature is disabled

    // Text types
    String(String), // VARCHAR/CHAR
    Text(String),   // TEXT/CLOB (unlimited length)

    // Binary
    Bytes(Vec<u8>),

    // Array types (PostgreSQL specific but important)
    Array(Vec<SqlValue>), // Array of values

    // Semantic types
    Enum(String),     // Enum value (database-agnostic)
    Uuid(String),     // UUID as string
    Json(JsonValue),  // JSON data
    Date(String),     // ISO date: "2024-01-15"
    Time(String),     // ISO time: "14:30:00" or "14:30:00.123"
    DateTime(String), // ISO datetime: "2024-01-15T10:30:00"
    Timestamp(i64),   // Unix timestamp (seconds since epoch)

    // Network types (PostgreSQL)
    Inet(IpAddr),     // IP address (v4 or v6)
    Cidr(IpAddr, u8), // IP address with prefix length (CIDR notation)
}

impl SqlValue {
    /// Check if this value is NULL
    pub fn is_null(&self) -> bool {
        matches!(self, SqlValue::Null)
    }

    /// Check if this value is DEFAULT
    pub fn is_default(&self) -> bool {
        matches!(self, SqlValue::Default)
    }

    /// Convert to a boolean if possible
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            SqlValue::Bool(b) => Some(*b),
            SqlValue::TinyInt(i) => Some(*i != 0),
            SqlValue::SmallInt(i) => Some(*i != 0),
            SqlValue::Int(i) => Some(*i != 0),
            SqlValue::BigInt(i) => Some(*i != 0),
            SqlValue::UnsignedTinyInt(i) => Some(*i != 0),
            SqlValue::UnsignedSmallInt(i) => Some(*i != 0),
            SqlValue::UnsignedInt(i) => Some(*i != 0),
            SqlValue::UnsignedBigInt(i) => Some(*i != 0),
            SqlValue::String(s) | SqlValue::Text(s) => match s.to_lowercase().as_str() {
                "true" | "t" | "yes" | "y" | "1" => Some(true),
                "false" | "f" | "no" | "n" | "0" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    /// Convert to an i32 if possible
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            SqlValue::TinyInt(i) => Some(*i as i32),
            SqlValue::SmallInt(i) => Some(*i as i32),
            SqlValue::Int(i) => Some(*i),
            SqlValue::UnsignedTinyInt(i) => Some(*i as i32),
            SqlValue::UnsignedSmallInt(i) => Some(*i as i32),
            SqlValue::UnsignedInt(i) if *i <= i32::MAX as u32 => Some(*i as i32),
            SqlValue::String(s) | SqlValue::Text(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// Convert to an i64 if possible
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            SqlValue::TinyInt(i) => Some(*i as i64),
            SqlValue::SmallInt(i) => Some(*i as i64),
            SqlValue::Int(i) => Some(*i as i64),
            SqlValue::BigInt(i) => Some(*i),
            SqlValue::UnsignedTinyInt(i) => Some(*i as i64),
            SqlValue::UnsignedSmallInt(i) => Some(*i as i64),
            SqlValue::UnsignedInt(i) => Some(*i as i64),
            SqlValue::UnsignedBigInt(i) if *i <= i64::MAX as u64 => Some(*i as i64),
            SqlValue::String(s) | SqlValue::Text(s) => s.parse().ok(),
            SqlValue::Timestamp(ts) => Some(*ts),
            _ => None,
        }
    }

    /// Convert to a String
    pub fn as_string(&self) -> Option<String> {
        match self {
            SqlValue::String(s) | SqlValue::Text(s) => Some(s.clone()),
            SqlValue::Enum(s) | SqlValue::Uuid(s) => Some(s.clone()),
            SqlValue::Date(s) | SqlValue::Time(s) | SqlValue::DateTime(s) => Some(s.clone()),
            SqlValue::Json(j) => Some(j.to_string()),
            SqlValue::Bool(b) => Some(b.to_string()),
            SqlValue::TinyInt(i) => Some(i.to_string()),
            SqlValue::SmallInt(i) => Some(i.to_string()),
            SqlValue::Int(i) => Some(i.to_string()),
            SqlValue::BigInt(i) => Some(i.to_string()),
            SqlValue::UnsignedTinyInt(i) => Some(i.to_string()),
            SqlValue::UnsignedSmallInt(i) => Some(i.to_string()),
            SqlValue::UnsignedInt(i) => Some(i.to_string()),
            SqlValue::UnsignedBigInt(i) => Some(i.to_string()),
            SqlValue::Float(f) => Some(f.to_string()),
            SqlValue::Double(f) => Some(f.to_string()),
            #[cfg(feature = "decimal")]
            SqlValue::Decimal(d) => Some(d.to_string()),
            #[cfg(not(feature = "decimal"))]
            SqlValue::Decimal(s) => Some(s.clone()),
            SqlValue::Timestamp(ts) => Some(ts.to_string()),
            SqlValue::Inet(ip) => Some(ip.to_string()),
            SqlValue::Cidr(ip, prefix) => Some(format!("{}/{}", ip, prefix)),
            SqlValue::Null => None,
            SqlValue::Default => None,
            SqlValue::Bytes(_) => None,
            SqlValue::Array(_) => None,
        }
    }

    /// Convert to JSON value
    pub fn to_json(&self) -> JsonValue {
        match self {
            SqlValue::Null => JsonValue::Null,
            SqlValue::Default => JsonValue::String("DEFAULT".to_string()),
            SqlValue::Bool(b) => JsonValue::Bool(*b),
            SqlValue::TinyInt(i) => JsonValue::Number((*i).into()),
            SqlValue::SmallInt(i) => JsonValue::Number((*i).into()),
            SqlValue::Int(i) => JsonValue::Number((*i).into()),
            SqlValue::BigInt(i) => JsonValue::Number((*i).into()),
            SqlValue::UnsignedTinyInt(i) => JsonValue::Number((*i).into()),
            SqlValue::UnsignedSmallInt(i) => JsonValue::Number((*i).into()),
            SqlValue::UnsignedInt(i) => JsonValue::Number((*i).into()),
            SqlValue::UnsignedBigInt(i) => JsonValue::Number((*i).into()),
            SqlValue::Float(f) => serde_json::Number::from_f64(*f as f64)
                .map(JsonValue::Number)
                .unwrap_or(JsonValue::Null),
            SqlValue::Double(f) => serde_json::Number::from_f64(*f)
                .map(JsonValue::Number)
                .unwrap_or(JsonValue::Null),
            #[cfg(feature = "decimal")]
            SqlValue::Decimal(d) => JsonValue::String(d.to_string()),
            #[cfg(not(feature = "decimal"))]
            SqlValue::Decimal(s) => JsonValue::String(s.clone()),
            SqlValue::String(s) | SqlValue::Text(s) => JsonValue::String(s.clone()),
            SqlValue::Enum(s) | SqlValue::Uuid(s) => JsonValue::String(s.clone()),
            SqlValue::Date(s) | SqlValue::Time(s) | SqlValue::DateTime(s) => {
                JsonValue::String(s.clone())
            }
            SqlValue::Json(j) => j.clone(),
            SqlValue::Timestamp(ts) => JsonValue::Number((*ts).into()),
            SqlValue::Inet(ip) => JsonValue::String(ip.to_string()),
            SqlValue::Cidr(ip, prefix) => JsonValue::String(format!("{}/{}", ip, prefix)),
            SqlValue::Bytes(bytes) => JsonValue::String(base64_encode(bytes)),
            SqlValue::Array(values) => {
                JsonValue::Array(values.iter().map(|v| v.to_json()).collect())
            }
        }
    }

    /// Convert to SQL string representation (for SQL generation, not binding)
    pub fn to_sql_string(&self) -> String {
        match self {
            SqlValue::Null => "NULL".to_string(),
            SqlValue::Default => "DEFAULT".to_string(),
            SqlValue::Bool(b) => b.to_string(),

            // Integer types
            SqlValue::TinyInt(i) => i.to_string(),
            SqlValue::SmallInt(i) => i.to_string(),
            SqlValue::Int(i) => i.to_string(),
            SqlValue::BigInt(i) => i.to_string(),

            // Unsigned integers
            SqlValue::UnsignedTinyInt(i) => i.to_string(),
            SqlValue::UnsignedSmallInt(i) => i.to_string(),
            SqlValue::UnsignedInt(i) => i.to_string(),
            SqlValue::UnsignedBigInt(i) => i.to_string(),

            // Floating point
            SqlValue::Float(f) => f.to_string(),
            SqlValue::Double(f) => f.to_string(),
            #[cfg(feature = "decimal")]
            SqlValue::Decimal(d) => d.to_string(),
            #[cfg(not(feature = "decimal"))]
            SqlValue::Decimal(s) => s.clone(),

            // Text types - escape single quotes
            SqlValue::String(s) | SqlValue::Text(s) => format!("'{}'", s.replace('\'', "''")),

            // Binary - return as hex string
            SqlValue::Bytes(bytes) => format!("X'{}'", hex_encode(bytes)),

            // Semantic types
            SqlValue::Enum(val) => format!("'{}'", val.replace('\'', "''")),
            SqlValue::Uuid(uuid) => format!("'{}'", uuid),
            SqlValue::Json(json) => format!("'{}'", json.to_string().replace('\'', "''")),
            SqlValue::Date(date) => format!("'{}'", date),
            SqlValue::Time(time) => format!("'{}'", time),
            SqlValue::DateTime(dt) => format!("'{}'", dt),
            SqlValue::Timestamp(ts) => ts.to_string(),

            // Network types
            SqlValue::Inet(ip) => format!("'{}'::inet", ip),
            SqlValue::Cidr(ip, prefix) => format!("'{}/{}'::cidr", ip, prefix),

            // Array (PostgreSQL style)
            SqlValue::Array(values) => {
                let elements: Vec<String> = values.iter().map(|v| v.to_sql_string()).collect();
                format!("ARRAY[{}]", elements.join(", "))
            }
        }
    }
}

impl fmt::Display for SqlValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SqlValue::Null => write!(f, "NULL"),
            SqlValue::Default => write!(f, "DEFAULT"),
            SqlValue::Bool(b) => write!(f, "{}", b),
            SqlValue::TinyInt(i) => write!(f, "{}", i),
            SqlValue::SmallInt(i) => write!(f, "{}", i),
            SqlValue::Int(i) => write!(f, "{}", i),
            SqlValue::BigInt(i) => write!(f, "{}", i),
            SqlValue::UnsignedTinyInt(i) => write!(f, "{}", i),
            SqlValue::UnsignedSmallInt(i) => write!(f, "{}", i),
            SqlValue::UnsignedInt(i) => write!(f, "{}", i),
            SqlValue::UnsignedBigInt(i) => write!(f, "{}", i),
            SqlValue::Float(fl) => write!(f, "{}", fl),
            SqlValue::Double(d) => write!(f, "{}", d),
            #[cfg(feature = "decimal")]
            SqlValue::Decimal(d) => write!(f, "{}", d),
            #[cfg(not(feature = "decimal"))]
            SqlValue::Decimal(s) => write!(f, "{}", s),
            SqlValue::String(s) | SqlValue::Text(s) => write!(f, "{}", s),
            SqlValue::Enum(s) | SqlValue::Uuid(s) => write!(f, "{}", s),
            SqlValue::Date(s) | SqlValue::Time(s) | SqlValue::DateTime(s) => write!(f, "{}", s),
            SqlValue::Json(j) => write!(f, "{}", j),
            SqlValue::Timestamp(ts) => write!(f, "{}", ts),
            SqlValue::Inet(ip) => write!(f, "{}", ip),
            SqlValue::Cidr(ip, prefix) => write!(f, "{}/{}", ip, prefix),
            SqlValue::Bytes(b) => write!(f, "<binary:{} bytes>", b.len()),
            SqlValue::Array(values) => {
                write!(f, "[")?;
                for (i, val) in values.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", val)?;
                }
                write!(f, "]")
            }
        }
    }
}

// Helper functions
fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02X}", b)).collect()
}

// From trait implementations for common types
impl From<bool> for SqlValue {
    fn from(v: bool) -> Self {
        SqlValue::Bool(v)
    }
}

impl From<i8> for SqlValue {
    fn from(v: i8) -> Self {
        SqlValue::TinyInt(v)
    }
}

impl From<i16> for SqlValue {
    fn from(v: i16) -> Self {
        SqlValue::SmallInt(v)
    }
}

impl From<i32> for SqlValue {
    fn from(v: i32) -> Self {
        SqlValue::Int(v)
    }
}

impl From<i64> for SqlValue {
    fn from(v: i64) -> Self {
        SqlValue::BigInt(v)
    }
}

impl From<u8> for SqlValue {
    fn from(v: u8) -> Self {
        SqlValue::UnsignedTinyInt(v)
    }
}

impl From<u16> for SqlValue {
    fn from(v: u16) -> Self {
        SqlValue::UnsignedSmallInt(v)
    }
}

impl From<u32> for SqlValue {
    fn from(v: u32) -> Self {
        SqlValue::UnsignedInt(v)
    }
}

impl From<u64> for SqlValue {
    fn from(v: u64) -> Self {
        SqlValue::UnsignedBigInt(v)
    }
}

impl From<f32> for SqlValue {
    fn from(v: f32) -> Self {
        SqlValue::Float(v)
    }
}

impl From<f64> for SqlValue {
    fn from(v: f64) -> Self {
        SqlValue::Double(v)
    }
}

impl From<String> for SqlValue {
    fn from(s: String) -> Self {
        // Auto-detect PostgreSQL enum values by the "::" pattern
        // This is crucial for PostgreSQL enum type casting
        if s.contains("::") {
            SqlValue::Enum(s)
        } else {
            SqlValue::String(s)
        }
    }
}

impl From<&str> for SqlValue {
    fn from(s: &str) -> Self {
        // Auto-detect PostgreSQL enum values by the "::" pattern
        // This is crucial for PostgreSQL enum type casting
        if s.contains("::") {
            SqlValue::Enum(s.to_string())
        } else {
            SqlValue::String(s.to_string())
        }
    }
}

impl From<Vec<u8>> for SqlValue {
    fn from(v: Vec<u8>) -> Self {
        SqlValue::Bytes(v)
    }
}

impl From<JsonValue> for SqlValue {
    fn from(v: JsonValue) -> Self {
        SqlValue::Json(v)
    }
}

impl<T> From<Option<T>> for SqlValue
where
    T: Into<SqlValue>,
{
    fn from(v: Option<T>) -> Self {
        match v {
            Some(val) => val.into(),
            None => SqlValue::Null,
        }
    }
}

// From reference implementations for convenience in tests and APIs
impl From<&String> for SqlValue {
    fn from(s: &String) -> Self {
        // Auto-detect PostgreSQL enum values by the "::" pattern
        if s.contains("::") {
            SqlValue::Enum(s.clone())
        } else {
            SqlValue::String(s.clone())
        }
    }
}

impl From<&i32> for SqlValue {
    fn from(i: &i32) -> Self {
        SqlValue::Int(*i)
    }
}

impl From<&i64> for SqlValue {
    fn from(i: &i64) -> Self {
        SqlValue::BigInt(*i)
    }
}

impl From<&bool> for SqlValue {
    fn from(b: &bool) -> Self {
        SqlValue::Bool(*b)
    }
}

impl From<&f32> for SqlValue {
    fn from(f: &f32) -> Self {
        SqlValue::Float(*f)
    }
}

impl From<&f64> for SqlValue {
    fn from(f: &f64) -> Self {
        SqlValue::Double(*f)
    }
}

// Date/Time type conversions
impl From<chrono::DateTime<chrono::Utc>> for SqlValue {
    fn from(dt: chrono::DateTime<chrono::Utc>) -> Self {
        SqlValue::DateTime(dt.to_rfc3339())
    }
}

impl From<chrono::NaiveDate> for SqlValue {
    fn from(date: chrono::NaiveDate) -> Self {
        SqlValue::Date(date.to_string())
    }
}

impl From<chrono::NaiveTime> for SqlValue {
    fn from(time: chrono::NaiveTime) -> Self {
        SqlValue::Time(time.to_string())
    }
}

impl From<chrono::NaiveDateTime> for SqlValue {
    fn from(dt: chrono::NaiveDateTime) -> Self {
        // Convert NaiveDateTime to UTC DateTime for consistent ISO 8601 format
        let utc_dt = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc);
        SqlValue::DateTime(utc_dt.to_rfc3339())
    }
}

// UUID type conversion (for uuid crate)
#[cfg(feature = "uuid")]
impl From<uuid::Uuid> for SqlValue {
    fn from(uuid: uuid::Uuid) -> Self {
        SqlValue::Uuid(uuid.to_string())
    }
}

// Decimal type conversion (for rust_decimal crate)
#[cfg(feature = "decimal")]
impl From<rust_decimal::Decimal> for SqlValue {
    fn from(d: rust_decimal::Decimal) -> Self {
        // Store the actual Decimal value for proper type handling
        SqlValue::Decimal(d)
    }
}

// IP Address type conversions
impl From<IpAddr> for SqlValue {
    fn from(ip: IpAddr) -> Self {
        SqlValue::Inet(ip)
    }
}

impl From<std::net::Ipv4Addr> for SqlValue {
    fn from(ip: std::net::Ipv4Addr) -> Self {
        SqlValue::Inet(IpAddr::V4(ip))
    }
}

impl From<std::net::Ipv6Addr> for SqlValue {
    fn from(ip: std::net::Ipv6Addr) -> Self {
        SqlValue::Inet(IpAddr::V6(ip))
    }
}

// CIDR type conversion
impl From<(IpAddr, u8)> for SqlValue {
    fn from((ip, prefix): (IpAddr, u8)) -> Self {
        SqlValue::Cidr(ip, prefix)
    }
}

// IpNetwork conversion (for PostgreSQL INET type)
impl From<IpNetwork> for SqlValue {
    fn from(network: IpNetwork) -> Self {
        // IpNetwork can represent both single IPs and CIDR blocks
        // For INET type, we use the IP address without the prefix for single hosts
        // and CIDR for network ranges
        match network {
            IpNetwork::V4(v4) => {
                if v4.prefix() == 32 {
                    // Single host, use INET
                    SqlValue::Inet(IpAddr::V4(v4.ip()))
                } else {
                    // Network range, use CIDR
                    SqlValue::Cidr(IpAddr::V4(v4.ip()), v4.prefix())
                }
            }
            IpNetwork::V6(v6) => {
                if v6.prefix() == 128 {
                    // Single host, use INET
                    SqlValue::Inet(IpAddr::V6(v6.ip()))
                } else {
                    // Network range, use CIDR
                    SqlValue::Cidr(IpAddr::V6(v6.ip()), v6.prefix())
                }
            }
        }
    }
}
