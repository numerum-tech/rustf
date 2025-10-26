//! Type registry for database type mappings
//!
//! This module provides a centralized registry of type mappings between
//! Rust types, SQL types, and database-specific types.

use super::value::SqlValue;
use std::collections::HashMap;

/// Rust type identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RustType {
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    String,
    Vec(Box<RustType>),
    Option(Box<RustType>),
    Json,
    Uuid,
    DateTime,
    Date,
    Time,
    Decimal,
}

/// SQL standard type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SqlType {
    // Boolean
    Boolean,

    // Numeric
    TinyInt,
    SmallInt,
    Integer,
    BigInt,
    Real,
    DoublePrecision,
    Decimal(Option<u8>, Option<u8>), // precision, scale
    Numeric(Option<u8>, Option<u8>),

    // Character
    Char(Option<u32>),
    VarChar(Option<u32>),
    Text,

    // Binary
    Binary(Option<u32>),
    VarBinary(Option<u32>),
    Blob,

    // Temporal
    Date,
    Time,
    Timestamp,
    DateTime,

    // Other
    Json,
    Uuid,
    Enum(Vec<String>),
}

/// Database-specific type information
#[derive(Debug, Clone)]
pub struct DatabaseTypeInfo {
    pub postgres_type: String,
    pub mysql_type: String,
    pub sqlite_type: String,
    pub sql_standard: SqlType,
}

/// Type mapping information
#[derive(Debug, Clone)]
pub struct TypeMapping {
    pub rust_type: RustType,
    pub sql_type: SqlType,
    pub database_info: DatabaseTypeInfo,
    pub nullable: bool,
    pub default_value: Option<SqlValue>,
}

/// Central type registry
pub struct TypeRegistry {
    mappings: HashMap<RustType, TypeMapping>,
    sql_to_rust: HashMap<SqlType, RustType>,
    postgres_types: HashMap<String, RustType>,
    mysql_types: HashMap<String, RustType>,
    sqlite_types: HashMap<String, RustType>,
}

impl TypeRegistry {
    /// Create a new type registry with default mappings
    pub fn new() -> Self {
        let mut registry = TypeRegistry {
            mappings: HashMap::new(),
            sql_to_rust: HashMap::new(),
            postgres_types: HashMap::new(),
            mysql_types: HashMap::new(),
            sqlite_types: HashMap::new(),
        };

        registry.initialize_default_mappings();
        registry
    }

    /// Initialize default type mappings
    fn initialize_default_mappings(&mut self) {
        // Boolean
        self.register_type(
            RustType::Bool,
            SqlType::Boolean,
            DatabaseTypeInfo {
                postgres_type: "BOOLEAN".to_string(),
                mysql_type: "BOOLEAN".to_string(),
                sqlite_type: "INTEGER".to_string(),
                sql_standard: SqlType::Boolean,
            },
        );

        // Small integers
        self.register_type(
            RustType::I8,
            SqlType::TinyInt,
            DatabaseTypeInfo {
                postgres_type: "SMALLINT".to_string(), // PostgreSQL doesn't have TINYINT
                mysql_type: "TINYINT".to_string(),
                sqlite_type: "INTEGER".to_string(),
                sql_standard: SqlType::TinyInt,
            },
        );

        self.register_type(
            RustType::I16,
            SqlType::SmallInt,
            DatabaseTypeInfo {
                postgres_type: "SMALLINT".to_string(),
                mysql_type: "SMALLINT".to_string(),
                sqlite_type: "INTEGER".to_string(),
                sql_standard: SqlType::SmallInt,
            },
        );

        // Standard integers
        self.register_type(
            RustType::I32,
            SqlType::Integer,
            DatabaseTypeInfo {
                postgres_type: "INTEGER".to_string(),
                mysql_type: "INT".to_string(),
                sqlite_type: "INTEGER".to_string(),
                sql_standard: SqlType::Integer,
            },
        );

        self.register_type(
            RustType::I64,
            SqlType::BigInt,
            DatabaseTypeInfo {
                postgres_type: "BIGINT".to_string(),
                mysql_type: "BIGINT".to_string(),
                sqlite_type: "INTEGER".to_string(),
                sql_standard: SqlType::BigInt,
            },
        );

        // Unsigned integers
        self.register_type(
            RustType::U8,
            SqlType::TinyInt,
            DatabaseTypeInfo {
                postgres_type: "SMALLINT".to_string(),
                mysql_type: "TINYINT UNSIGNED".to_string(),
                sqlite_type: "INTEGER".to_string(),
                sql_standard: SqlType::TinyInt,
            },
        );

        self.register_type(
            RustType::U16,
            SqlType::SmallInt,
            DatabaseTypeInfo {
                postgres_type: "INTEGER".to_string(),
                mysql_type: "SMALLINT UNSIGNED".to_string(),
                sqlite_type: "INTEGER".to_string(),
                sql_standard: SqlType::SmallInt,
            },
        );

        self.register_type(
            RustType::U32,
            SqlType::Integer,
            DatabaseTypeInfo {
                postgres_type: "BIGINT".to_string(),
                mysql_type: "INT UNSIGNED".to_string(),
                sqlite_type: "INTEGER".to_string(),
                sql_standard: SqlType::Integer,
            },
        );

        self.register_type(
            RustType::U64,
            SqlType::BigInt,
            DatabaseTypeInfo {
                postgres_type: "NUMERIC(20,0)".to_string(),
                mysql_type: "BIGINT UNSIGNED".to_string(),
                sqlite_type: "INTEGER".to_string(),
                sql_standard: SqlType::BigInt,
            },
        );

        // Floating point
        self.register_type(
            RustType::F32,
            SqlType::Real,
            DatabaseTypeInfo {
                postgres_type: "REAL".to_string(),
                mysql_type: "FLOAT".to_string(),
                sqlite_type: "REAL".to_string(),
                sql_standard: SqlType::Real,
            },
        );

        self.register_type(
            RustType::F64,
            SqlType::DoublePrecision,
            DatabaseTypeInfo {
                postgres_type: "DOUBLE PRECISION".to_string(),
                mysql_type: "DOUBLE".to_string(),
                sqlite_type: "REAL".to_string(),
                sql_standard: SqlType::DoublePrecision,
            },
        );

        // String
        self.register_type(
            RustType::String,
            SqlType::Text,
            DatabaseTypeInfo {
                postgres_type: "TEXT".to_string(),
                mysql_type: "TEXT".to_string(),
                sqlite_type: "TEXT".to_string(),
                sql_standard: SqlType::Text,
            },
        );

        // JSON
        self.register_type(
            RustType::Json,
            SqlType::Json,
            DatabaseTypeInfo {
                postgres_type: "JSONB".to_string(),
                mysql_type: "JSON".to_string(),
                sqlite_type: "TEXT".to_string(),
                sql_standard: SqlType::Json,
            },
        );

        // UUID
        self.register_type(
            RustType::Uuid,
            SqlType::Uuid,
            DatabaseTypeInfo {
                postgres_type: "UUID".to_string(),
                mysql_type: "CHAR(36)".to_string(),
                sqlite_type: "TEXT".to_string(),
                sql_standard: SqlType::Uuid,
            },
        );

        // Temporal types
        self.register_type(
            RustType::DateTime,
            SqlType::Timestamp,
            DatabaseTypeInfo {
                postgres_type: "TIMESTAMP WITH TIME ZONE".to_string(),
                mysql_type: "DATETIME".to_string(),
                sqlite_type: "TEXT".to_string(),
                sql_standard: SqlType::Timestamp,
            },
        );

        self.register_type(
            RustType::Date,
            SqlType::Date,
            DatabaseTypeInfo {
                postgres_type: "DATE".to_string(),
                mysql_type: "DATE".to_string(),
                sqlite_type: "TEXT".to_string(),
                sql_standard: SqlType::Date,
            },
        );

        self.register_type(
            RustType::Time,
            SqlType::Time,
            DatabaseTypeInfo {
                postgres_type: "TIME".to_string(),
                mysql_type: "TIME".to_string(),
                sqlite_type: "TEXT".to_string(),
                sql_standard: SqlType::Time,
            },
        );

        // Decimal
        self.register_type(
            RustType::Decimal,
            SqlType::Decimal(None, None),
            DatabaseTypeInfo {
                postgres_type: "NUMERIC".to_string(),
                mysql_type: "DECIMAL".to_string(),
                sqlite_type: "TEXT".to_string(),
                sql_standard: SqlType::Decimal(None, None),
            },
        );
    }

    /// Register a type mapping
    fn register_type(
        &mut self,
        rust_type: RustType,
        sql_type: SqlType,
        database_info: DatabaseTypeInfo,
    ) {
        // Register in Rust->SQL mapping
        self.mappings.insert(
            rust_type.clone(),
            TypeMapping {
                rust_type: rust_type.clone(),
                sql_type: sql_type.clone(),
                database_info: database_info.clone(),
                nullable: false,
                default_value: None,
            },
        );

        // Register in SQL->Rust mapping
        self.sql_to_rust.insert(sql_type, rust_type.clone());

        // Register database-specific mappings
        self.postgres_types.insert(
            database_info.postgres_type.to_uppercase(),
            rust_type.clone(),
        );
        self.mysql_types
            .insert(database_info.mysql_type.to_uppercase(), rust_type.clone());
        self.sqlite_types
            .insert(database_info.sqlite_type.to_uppercase(), rust_type.clone());
    }

    /// Get type mapping for a Rust type
    pub fn get_mapping(&self, rust_type: &RustType) -> Option<&TypeMapping> {
        self.mappings.get(rust_type)
    }

    /// Get Rust type from SQL type
    pub fn rust_type_from_sql(&self, sql_type: &SqlType) -> Option<&RustType> {
        self.sql_to_rust.get(sql_type)
    }

    /// Get Rust type from PostgreSQL type name
    pub fn rust_type_from_postgres(&self, type_name: &str) -> Option<&RustType> {
        let normalized = type_name.to_uppercase();
        self.postgres_types.get(&normalized).or_else(|| {
            // Handle common PostgreSQL type aliases
            match normalized.as_str() {
                "BOOL" => self.postgres_types.get("BOOLEAN"),
                "INT2" => self.postgres_types.get("SMALLINT"),
                "INT4" => self.postgres_types.get("INTEGER"),
                "INT8" => self.postgres_types.get("BIGINT"),
                "FLOAT4" => self.postgres_types.get("REAL"),
                "FLOAT8" => self.postgres_types.get("DOUBLE PRECISION"),
                "VARCHAR" | "CHAR" | "BPCHAR" | "NAME" => self.postgres_types.get("TEXT"),
                "TIMESTAMPTZ" => self.postgres_types.get("TIMESTAMP WITH TIME ZONE"),
                _ => None,
            }
        })
    }

    /// Get Rust type from MySQL type name
    pub fn rust_type_from_mysql(&self, type_name: &str) -> Option<&RustType> {
        let normalized = type_name.to_uppercase();
        self.mysql_types.get(&normalized).or_else(|| {
            // Handle common MySQL type variations
            match normalized.as_str() {
                "INTEGER" => self.mysql_types.get("INT"),
                "BOOL" => self.mysql_types.get("BOOLEAN"),
                "MEDIUMINT" => self.mysql_types.get("INT"),
                "DOUBLE PRECISION" | "REAL" => self.mysql_types.get("DOUBLE"),
                "VARCHAR" | "CHAR" | "TINYTEXT" | "MEDIUMTEXT" | "LONGTEXT" => {
                    self.mysql_types.get("TEXT")
                }
                "TIMESTAMP" => self.mysql_types.get("DATETIME"),
                _ => None,
            }
        })
    }

    /// Get Rust type from SQLite type name
    pub fn rust_type_from_sqlite(&self, type_name: &str) -> Option<&RustType> {
        let normalized = type_name.to_uppercase();
        // SQLite has type affinity, so we need to be more flexible
        if normalized.contains("INT") {
            self.sqlite_types.get("INTEGER")
        } else if normalized.contains("CHAR")
            || normalized.contains("CLOB")
            || normalized.contains("TEXT")
        {
            self.sqlite_types.get("TEXT")
        } else if normalized.contains("BLOB") {
            None // We'll handle this specially
        } else if normalized.contains("REAL")
            || normalized.contains("FLOA")
            || normalized.contains("DOUB")
        {
            self.sqlite_types.get("REAL")
        } else {
            self.sqlite_types.get(&normalized)
        }
    }

    /// Check if a type should be nullable
    pub fn is_nullable(&self, rust_type: &RustType) -> bool {
        matches!(rust_type, RustType::Option(_))
    }

    /// Get the inner type of an Option
    pub fn unwrap_option<'a>(&self, rust_type: &'a RustType) -> &'a RustType {
        match rust_type {
            RustType::Option(inner) => inner,
            _ => rust_type,
        }
    }
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
