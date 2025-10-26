//! PostgreSQL database introspection implementation

use super::{common::*, DatabaseIntrospector};
use anyhow::Result;
use async_trait::async_trait;
use sqlx::{Pool, Postgres, Row};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tokio::fs;

pub struct PostgresIntrospector {
    pool: Pool<Postgres>,
    db_name: String,
}

impl PostgresIntrospector {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
            
        // Get database name
        let db_name_row = sqlx::query("SELECT current_database() as db_name")
            .fetch_one(&pool)
            .await?;
        let db_name: String = db_name_row.try_get("db_name")?;
        
        Ok(Self { pool, db_name })
    }
    
    async fn get_enum_values(&self, type_name: &str) -> Result<Vec<String>> {
        let enum_rows = sqlx::query(
            r#"
            SELECT enumlabel
            FROM pg_enum
            WHERE enumtypid = (
                SELECT oid FROM pg_type WHERE typname = $1
            )
            ORDER BY enumsortorder
            "#
        )
        .bind(type_name)
        .fetch_all(&self.pool)
        .await?;
        
        let mut values = Vec::new();
        for row in enum_rows {
            let value: String = row.try_get("enumlabel")?;
            values.push(value);
        }
        Ok(values)
    }
    
    async fn get_all_custom_types(&self) -> Result<HashMap<String, Vec<String>>> {
        // Get all enum types in the database
        let type_rows = sqlx::query(
            r#"
            SELECT t.typname, array_agg(e.enumlabel ORDER BY e.enumsortorder) as values
            FROM pg_type t
            JOIN pg_enum e ON t.oid = e.enumtypid
            WHERE t.typtype = 'e'  -- 'e' for enum types
            GROUP BY t.typname
            "#
        )
        .fetch_all(&self.pool)
        .await?;
        
        let mut types = HashMap::new();
        for row in type_rows {
            let type_name: String = row.try_get("typname")?;
            let values: Vec<String> = row.try_get("values")?;
            types.insert(type_name, values);
        }
        
        Ok(types)
    }
    
    fn generate_field_yaml(&self, column: &ColumnInfo) -> String {
        let mut field = String::new();
        let escaped_field_name = escape_yaml_field_name(&column.name);
        field.push_str(&format!("    {}:\n", escaped_field_name));
        
        // Check if this is a PostgreSQL enum type (USER-DEFINED with custom type name)
        let is_enum = column.data_type == "USER-DEFINED" && column.postgres_type_name.is_some();
        
        // Handle PostgreSQL-specific data types
        let data_type = if is_enum {
            // For USER-DEFINED types (enums), generate inline enum type with values
            // This preserves backward compatibility while the types section provides reference
            "enum".to_string()
        } else {
            let data_type = match column.data_type.as_str() {
            "character varying" => {
                if let Some(max_length) = column.max_length {
                    format!("string({})", max_length)
                } else {
                    "string".to_string()
                }
            },
            "character" => {
                if let Some(max_length) = column.max_length {
                    format!("string({})", max_length)
                } else {
                    "string".to_string()
                }
            },
            "text" => "text".to_string(),
            "integer" => "int".to_string(),
            "bigint" => "bigint".to_string(),
            "smallint" => "smallint".to_string(),
            "numeric" => {
                if let (Some(precision), Some(scale)) = (column.numeric_precision, column.numeric_scale) {
                    format!("decimal({},{})", precision, scale)
                } else {
                    "decimal".to_string()
                }
            },
            "real" => "float".to_string(),
            "double precision" => "double".to_string(),
            "boolean" => "boolean".to_string(),
            "timestamp without time zone" => "timestamp".to_string(),
            "timestamp with time zone" => "timestamp".to_string(),
            "date" => "date".to_string(),
            "time without time zone" => "time".to_string(),
            "time with time zone" => "time".to_string(),
            "uuid" => "uuid".to_string(),
            "json" => "json".to_string(),
            "jsonb" => "json".to_string(),
            "bytea" => "blob".to_string(),
            "inet" => "inet".to_string(),
            "cidr" => "cidr".to_string(),
            // Handle PostgreSQL-specific types that don't map well to RustF
            "ARRAY" => {
                // Handle array types using the udt_name
                if let Some(udt_name) = &column.postgres_type_name {
                    // PostgreSQL array type names start with underscore
                    // e.g., _text for text[], _int4 for integer[], _currency for currency[]
                    match udt_name.as_str() {
                        "_text" | "_varchar" | "_char" | "_bpchar" => "array<string>".to_string(),
                        "_int2" | "_int4" | "_int8" => "array<integer>".to_string(),
                        "_float4" | "_float8" | "_numeric" => "array<float>".to_string(),
                        "_bool" | "_boolean" => "array<boolean>".to_string(),
                        "_uuid" => "array<uuid>".to_string(),
                        "_date" => "array<date>".to_string(),
                        "_timestamp" | "_timestamptz" => "array<timestamp>".to_string(),
                        "_json" | "_jsonb" => "array<json>".to_string(),
                        // For custom enum arrays, check if it's an enum type
                        custom_type if custom_type.starts_with("_") => {
                            // Try to determine if it's an enum array
                            let base_type = &custom_type[1..]; // Remove leading underscore
                            format!("array<{}>", base_type)
                        }
                        _ => "array".to_string() // Generic array
                    }
                } else {
                    "array".to_string() // Fallback to generic array
                }
            },
            "USER-DEFINED" => "string".to_string(), // Non-enum custom types as strings
            _ => {
                // For unknown types, default to string and add a warning comment
                if column.data_type.starts_with("_") {
                    // PostgreSQL array types start with underscore (shouldn't reach here)
                    "array".to_string()
                } else {
                    "string".to_string()
                }
            },
            };
            data_type
        };
        
        field.push_str(&format!("      type: {}\n", data_type));
        
        // Add enum values for enum types (for backward compatibility)
        if is_enum && column.column_type.is_some() {
            // column_type contains JSON array of enum values
            if let Some(column_type) = &column.column_type {
                // Try to parse as JSON array
                if let Ok(values) = serde_json::from_str::<Vec<String>>(column_type) {
                    if !values.is_empty() {
                        let values_yaml = values.iter()
                            .map(|v| format!("\"{}\"", v))
                            .collect::<Vec<_>>()
                            .join(", ");
                        field.push_str(&format!("      values: [{}]\n", values_yaml));
                    }
                }
            }
        }
        
        // Add PostgreSQL enum type name if available
        if let Some(postgres_type) = &column.postgres_type_name {
            field.push_str(&format!("      postgres_type_name: {}\n", postgres_type));
        }
        
        // Add default value if present
        if let Some(default) = &column.default_value {
            if !default.is_empty() && default != "NULL" {
                // Clean PostgreSQL casting syntax (e.g., 'value'::type -> value)
                let clean_default = self.clean_default_value(default);
                field.push_str(&format!("      default: {}\n", clean_default));
            }
        }
        
        // Add lang_type
        let lang_type = self.map_to_rust_type(&column.data_type, column.is_nullable);
        field.push_str(&format!("      lang_type: {}\n", lang_type));
        
        // Add nullable flag
        if column.is_nullable {
            field.push_str("      nullable: true\n");
        } else {
            field.push_str("      required: true\n");
        }
        
        // Add primary key flag
        if column.is_primary_key {
            field.push_str("      primary_key: true\n");
        }
        
        // Add unique flag
        if column.is_unique {
            field.push_str("      unique: true\n");
        }
        
        // Add AI hint
        let ai_hint = generate_field_ai_hint(&column.name, &column.data_type, column.is_foreign_key);
        field.push_str(&format!("      ai: \"{}\"\n", ai_hint));
        
        // Add foreign key reference
        if column.is_foreign_key {
            if let (Some(table), Some(col)) = (&column.foreign_table, &column.foreign_column) {
                field.push_str(&format!("      foreign_key: {}.{}\n", table, col));
            }
        }
        
        // Add column comment if available
        if let Some(comment) = &column.comment {
            if !comment.is_empty() {
                field.push_str(&format!("      column_comment: \"{}\"\n", comment));
            }
        }
        
        field.push_str("\n");
        field
    }
    
    /// Clean PostgreSQL default values by removing casting syntax
    fn clean_default_value(&self, default: &str) -> String {
        // Handle ARRAY[] syntax first - arrays can have type casts inside elements
        // e.g., ARRAY['XOF'::currency] or ARRAY['XOF'::currency]::currency[]
        if default.starts_with("ARRAY[") {
            // Find the closing bracket of the array
            if let Some(close_bracket) = default.rfind(']') {
                // Keep everything up to and including the closing bracket
                let array_part = &default[..=close_bracket];
                
                // Check if there's an external cast after the array (e.g., ]::currency[])
                if close_bracket + 1 < default.len() && default[close_bracket + 1..].starts_with("::") {
                    // There's an external cast, return just the array part
                    return array_part.to_string();
                } else {
                    // No external cast, return the whole array including internal casts
                    return array_part.to_string();
                }
            }
            // If no closing bracket found, return as-is (shouldn't happen with valid SQL)
            return default.to_string();
        }
        
        // Handle PostgreSQL casting syntax for non-array values: 'value'::type or value::type
        if let Some(cast_pos) = default.find("::") {
            let value_part = &default[..cast_pos];
            
            // If it's a quoted string, return it as-is (PostgreSQL format)
            if (value_part.starts_with('\'') && value_part.ends_with('\'')) ||
               (value_part.starts_with('"') && value_part.ends_with('"')) {
                return value_part.to_string();
            }
            
            // For unquoted values, return as-is
            return value_part.to_string();
        }
        
        // For other cases, return as-is
        default.to_string()
    }
    
    fn map_to_rust_type(&self, data_type: &str, is_nullable: bool) -> String {
        let base_type = match data_type {
            "integer" | "int4" | "serial" => "i32",
            "bigint" | "int8" | "bigserial" => "i64",
            "smallint" | "int2" => "i16",
            "real" | "float4" => "f32",
            "double precision" | "float8" => "f64",
            "numeric" | "decimal" => "Decimal",
            "boolean" | "bool" => "bool",
            "character varying" | "varchar" | "character" | "char" | "text" => "String",
            "date" => "NaiveDate",
            "time without time zone" | "time with time zone" => "NaiveTime",
            "timestamp without time zone" | "timestamp with time zone" => "DateTime<Utc>",
            "json" | "jsonb" => "serde_json::Value",
            "uuid" => "Uuid",
            "bytea" => "Vec<u8>",
            "inet" => "ipnetwork::IpNetwork",
            "cidr" => "String", // CIDR needs prefix length, stored as string
            // Handle PostgreSQL-specific types
            "ARRAY" => "Vec<String>", // Generic array fallback
            "USER-DEFINED" => "String", // Custom enums as strings
            "enum" => "String", // PostgreSQL enum types mapped to String
            _ => {
                // Handle array types (prefixed with _)
                if data_type.starts_with("_") {
                    // PostgreSQL array types start with underscore
                    match data_type {
                        "_uuid" => "Vec<Uuid>",
                        "_text" | "_varchar" | "_char" | "_bpchar" => "Vec<String>",
                        "_int2" => "Vec<i16>",
                        "_int4" => "Vec<i32>",
                        "_int8" => "Vec<i64>",
                        "_float4" => "Vec<f32>",
                        "_float8" => "Vec<f64>",
                        "_bool" => "Vec<bool>",
                        "_date" => "Vec<NaiveDate>",
                        "_time" | "_timetz" => "Vec<NaiveTime>",
                        "_timestamp" | "_timestamptz" => "Vec<DateTime<Utc>>",
                        "_numeric" | "_decimal" => "Vec<Decimal>",
                        "_json" | "_jsonb" => "Vec<serde_json::Value>",
                        "_bytea" => "Vec<Vec<u8>>",
                        "_inet" => "Vec<ipnetwork::IpNetwork>",
                        _ => "Vec<String>", // Custom enum arrays and unknown types
                    }
                // Handle our custom array<type> format from schema
                } else if data_type.starts_with("array<") && data_type.ends_with(">") {
                    // Extract the inner type from array<type>
                    let inner = &data_type[6..data_type.len()-1];
                    match inner {
                        "uuid" => "Vec<Uuid>",
                        "string" | "text" => "Vec<String>",
                        "int" | "integer" => "Vec<i32>",
                        "bigint" => "Vec<i64>",
                        "smallint" => "Vec<i16>",
                        "tinyint" => "Vec<i8>",
                        "float" => "Vec<f32>",
                        "double" => "Vec<f64>",
                        "bool" | "boolean" => "Vec<bool>",
                        "date" => "Vec<NaiveDate>",
                        "time" => "Vec<NaiveTime>",
                        "timestamp" | "datetime" => "Vec<DateTime<Utc>>",
                        "decimal" => "Vec<Decimal>",
                        "json" => "Vec<serde_json::Value>",
                        "binary" | "bytea" => "Vec<Vec<u8>>",
                        _ => "Vec<String>", // Custom types as string arrays
                    }
                } else {
                    "String"
                }
            },
        };
        
        if is_nullable {
            format!("Option<{}>", base_type)
        } else {
            base_type.to_string()
        }
    }
    
    async fn generate_table_schema_yaml(&self, description: &TableDescription) -> Result<String> {
        let mut yaml = String::new();
        
        // Add header comment
        yaml.push_str(&format!("# {} entity - Generated from database\n\n", description.table.name));
        
        // Model name (convert table name to PascalCase)
        let model_name = to_pascal_case(&description.table.name);
        yaml.push_str(&format!("{}:\n", model_name));
        yaml.push_str(&format!("  table: {}\n", description.table.name));
        yaml.push_str(&format!("  database_type: postgres\n"));
        yaml.push_str(&format!("  database_name: {}\n", self.db_name));
        
        // Determine element type (table or view)
        let element_type = if description.table.table_type.to_lowercase().contains("view") {
            "view"
        } else {
            "table"
        };
        yaml.push_str(&format!("  element_type: {}\n", element_type));
        yaml.push_str("  version: 1\n");
        
        // Add description if available
        if let Some(comment) = &description.table.comment {
            if !comment.is_empty() {
                yaml.push_str(&format!("  description: \"{}\"\n", comment));
            }
        }
        
        // Add AI context based on table name
        let ai_context = generate_table_ai_context(&description.table.name);
        yaml.push_str(&format!("  ai_context: \"{}\"\n", ai_context));
        
        // Fetch all custom types and find which ones are used in this table
        let all_types = self.get_all_custom_types().await.unwrap_or_default();
        let mut used_types = HashSet::new();
        
        // Collect types used in this table
        for column in &description.columns {
            if column.data_type == "USER-DEFINED" {
                if let Some(type_name) = &column.postgres_type_name {
                    used_types.insert(type_name.clone());
                }
            } else if column.data_type == "ARRAY" {
                // Check if it's an array of a custom type
                if let Some(array_type) = &column.postgres_type_name {
                    // Remove underscore prefix to get base type name
                    if array_type.starts_with("_") {
                        let base_type = &array_type[1..];
                        if all_types.contains_key(base_type) {
                            used_types.insert(base_type.to_string());
                        }
                    }
                }
            }
        }
        
        // Add types section if there are custom types used
        if !used_types.is_empty() {
            yaml.push_str("  \n  types:\n");
            let mut sorted_types: Vec<_> = used_types.iter().collect();
            sorted_types.sort();
            
            for type_name in sorted_types {
                if let Some(values) = all_types.get(type_name) {
                    yaml.push_str(&format!("    {}:\n", type_name));
                    yaml.push_str("      kind: enum\n");
                    yaml.push_str("      values:\n");
                    for value in values {
                        yaml.push_str(&format!("        - '{}'\n", value));
                    }
                }
            }
        }
        
        yaml.push_str("  \n  fields:\n");
        
        // Generate fields
        for column in &description.columns {
            yaml.push_str(&self.generate_field_yaml(column));
        }
        
        // Generate relations if foreign keys exist
        let relations = generate_relations_yaml(&description.columns);
        if !relations.is_empty() {
            yaml.push_str("  \n  relations:\n");
            yaml.push_str(&relations);
        }
        
        // Generate indexes
        if !description.indexes.is_empty() {
            yaml.push_str("  \n  indexes:\n");
            for index in &description.indexes {
                if index.columns.len() == 1 {
                    yaml.push_str(&format!("    - {}\n", index.columns[0]));
                } else {
                    yaml.push_str(&format!("    - [{}]\n", index.columns.join(", ")));
                }
            }
        }
        
        yaml.push_str("\n");
        Ok(yaml)
    }
}

#[async_trait]
impl DatabaseIntrospector for PostgresIntrospector {
    async fn list_tables(&self, metadata: bool) -> Result<Vec<TableInfo>> {
        let query = if metadata {
            r#"
            SELECT 
                t.table_name,
                t.table_schema,
                t.table_type,
                NULL as row_count,
                NULL as size_bytes,
                obj_description(c.oid) as comment
            FROM information_schema.tables t
            LEFT JOIN pg_class c ON c.relname = t.table_name
            WHERE t.table_schema = 'public'
            ORDER BY t.table_name
            "#
        } else {
            r#"
            SELECT 
                table_name,
                table_schema,
                table_type,
                NULL as row_count,
                NULL as size_bytes,
                NULL as comment
            FROM information_schema.tables
            WHERE table_schema = 'public'
            ORDER BY table_name
            "#
        };
        
        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await?;
            
        let mut tables = Vec::new();
        for row in rows {
            tables.push(TableInfo {
                name: row.try_get("table_name")?,
                schema: row.try_get("table_schema").ok(),
                table_type: row.try_get("table_type")?,
                row_count: row.try_get("row_count").ok().flatten(),
                size_bytes: row.try_get("size_bytes").ok().flatten(),
                comment: row.try_get("comment").ok().flatten(),
            });
        }
        
        Ok(tables)
    }
    
    async fn describe_table(&self, table_name: &str) -> Result<TableDescription> {
        // Get table info
        let table_row = sqlx::query(
            r#"
            SELECT 
                t.table_name,
                t.table_schema,
                t.table_type,
                NULL as row_count,
                NULL as size_bytes,
                obj_description(c.oid) as comment
            FROM information_schema.tables t
            LEFT JOIN pg_class c ON c.relname = t.table_name
            WHERE t.table_name = $1 AND t.table_schema = 'public'
            "#
        )
        .bind(table_name)
        .fetch_optional(&self.pool)
        .await?;
        
        let table_info = match table_row {
            Some(row) => TableInfo {
                name: row.try_get("table_name")?,
                schema: row.try_get("table_schema").ok(),
                table_type: row.try_get("table_type")?,
                row_count: row.try_get("row_count").ok().flatten(),
                size_bytes: row.try_get("size_bytes").ok().flatten(),
                comment: row.try_get("comment").ok().flatten(),
            },
            None => anyhow::bail!("Table '{}' not found", table_name),
        };
        
        // Get column information including custom type names
        let column_rows = sqlx::query(
            r#"
            SELECT 
                c.column_name,
                c.data_type,
                c.is_nullable,
                c.column_default,
                c.character_maximum_length,
                c.numeric_precision,
                c.numeric_scale,
                col_description(pgc.oid, c.ordinal_position) as column_comment,
                CASE 
                    WHEN c.data_type IN ('USER-DEFINED', 'ARRAY') THEN c.udt_name
                    ELSE NULL
                END as custom_type_name
            FROM information_schema.columns c
            LEFT JOIN pg_class pgc ON pgc.relname = c.table_name
            WHERE c.table_name = $1 AND c.table_schema = 'public'
            ORDER BY c.ordinal_position
            "#
        )
        .bind(table_name)
        .fetch_all(&self.pool)
        .await?;
        
        // Get primary key information
        let pk_rows = sqlx::query(
            r#"
            SELECT kcu.column_name
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage kcu
                ON tc.constraint_name = kcu.constraint_name
                AND tc.table_schema = kcu.table_schema
            WHERE tc.table_name = $1
                AND tc.table_schema = 'public'
                AND tc.constraint_type = 'PRIMARY KEY'
            "#
        )
        .bind(table_name)
        .fetch_all(&self.pool)
        .await?;
        
        let mut primary_keys = std::collections::HashSet::new();
        for pk_row in pk_rows {
            let column_name: String = pk_row.try_get("column_name")?;
            primary_keys.insert(column_name);
        }
        
        // Get foreign key information
        let fk_rows = sqlx::query(
            r#"
            SELECT 
                kcu.column_name,
                ccu.table_name as foreign_table_name,
                ccu.column_name as foreign_column_name,
                rc.delete_rule,
                rc.update_rule
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage kcu
                ON tc.constraint_name = kcu.constraint_name
            JOIN information_schema.constraint_column_usage ccu
                ON ccu.constraint_name = tc.constraint_name
            JOIN information_schema.referential_constraints rc
                ON tc.constraint_name = rc.constraint_name
            WHERE tc.table_name = $1
                AND tc.table_schema = 'public'
                AND tc.constraint_type = 'FOREIGN KEY'
            "#
        )
        .bind(table_name)
        .fetch_all(&self.pool)
        .await?;
        
        // Build foreign key lookup
        let mut foreign_keys = HashMap::new();
        for fk_row in fk_rows {
            let column_name: String = fk_row.try_get("column_name")?;
            let foreign_table: String = fk_row.try_get("foreign_table_name")?;
            let foreign_column: String = fk_row.try_get("foreign_column_name")?;
            let on_delete: Option<String> = fk_row.try_get("delete_rule").ok();
            let on_update: Option<String> = fk_row.try_get("update_rule").ok();
            foreign_keys.insert(column_name, (foreign_table, foreign_column, on_delete, on_update));
        }
        
        // Get unique constraints
        let unique_rows = sqlx::query(
            r#"
            SELECT kcu.column_name
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage kcu
                ON tc.constraint_name = kcu.constraint_name
            WHERE tc.table_name = $1
                AND tc.table_schema = 'public'
                AND tc.constraint_type = 'UNIQUE'
            "#
        )
        .bind(table_name)
        .fetch_all(&self.pool)
        .await?;
        
        let mut unique_keys = std::collections::HashSet::new();
        for unique_row in unique_rows {
            let column_name: String = unique_row.try_get("column_name")?;
            unique_keys.insert(column_name);
        }
        
        let mut columns = Vec::new();
        for row in column_rows {
            let column_name: String = row.try_get("column_name")?;
            let is_primary_key = primary_keys.contains(&column_name);
            let is_unique = unique_keys.contains(&column_name);
            let is_foreign_key = foreign_keys.contains_key(&column_name);
            
            let (foreign_table, foreign_column, on_delete, on_update) = if is_foreign_key {
                let (table, column, del, upd) = foreign_keys.get(&column_name).unwrap();
                (Some(table.clone()), Some(column.clone()), del.clone(), upd.clone())
            } else {
                (None, None, None, None)
            };
            
            let data_type: String = row.try_get("data_type")?;
            let custom_type_name: Option<String> = row.try_get("custom_type_name").ok();
            
            // For USER-DEFINED types, fetch enum values and store as JSON in column_type
            let column_type_with_values = if data_type == "USER-DEFINED" && custom_type_name.is_some() {
                let type_name = custom_type_name.as_ref().unwrap();
                match self.get_enum_values(type_name).await {
                    Ok(values) if !values.is_empty() => {
                        // Store enum values as JSON for later use in field generation
                        Some(serde_json::to_string(&values).unwrap_or_else(|_| type_name.clone()))
                    },
                    _ => custom_type_name.clone(), // Fallback to just the type name
                }
            } else {
                custom_type_name.clone()
            };
            
            columns.push(ColumnInfo {
                name: column_name,
                data_type,
                column_type: column_type_with_values,
                postgres_type_name: custom_type_name, // Store the PostgreSQL enum type name
                is_nullable: row.try_get::<&str, _>("is_nullable")? == "YES",
                default_value: row.try_get("column_default").ok(),
                is_primary_key,
                is_unique,
                is_foreign_key,
                foreign_table,
                foreign_column,
                on_delete,
                on_update,
                comment: row.try_get("column_comment").ok(),
                max_length: row.try_get("character_maximum_length").ok(),
                numeric_precision: row.try_get("numeric_precision").ok(),
                numeric_scale: row.try_get("numeric_scale").ok(),
            });
        }
        
        // Get indexes
        let index_rows = sqlx::query(
            r#"
            SELECT 
                i.relname as index_name,
                a.attname as column_name,
                idx.indisunique as is_unique,
                idx.indisprimary as is_primary
            FROM pg_class t
            JOIN pg_index idx ON t.oid = idx.indrelid
            JOIN pg_class i ON i.oid = idx.indexrelid
            JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(idx.indkey)
            WHERE t.relname = $1
            ORDER BY i.relname, a.attnum
            "#
        )
        .bind(table_name)
        .fetch_all(&self.pool)
        .await?;
        
        // Group indexes by name
        let mut index_map: HashMap<String, IndexInfo> = HashMap::new();
        for row in index_rows {
            let index_name: String = row.try_get("index_name")?;
            let column_name: String = row.try_get("column_name")?;
            let is_unique: bool = row.try_get("is_unique")?;
            let is_primary: bool = row.try_get("is_primary")?;
            
            index_map.entry(index_name.clone())
                .or_insert_with(|| IndexInfo {
                    name: index_name.clone(),
                    columns: Vec::new(),
                    is_unique,
                    is_primary,
                })
                .columns.push(column_name);
        }
        let indexes: Vec<IndexInfo> = index_map.into_values().collect();
        
        Ok(TableDescription {
            table: table_info,
            columns,
            indexes,
            constraints: Vec::new(), // Can be extended later if needed
            triggers: Vec::new(),    // Can be extended later if needed
        })
    }
    
    async fn generate_schemas(
        &self,
        output_dir: &PathBuf,
        force: bool,
        filter_tables: &[String],
    ) -> Result<()> {
        // Create output directory if it doesn't exist
        if !output_dir.exists() {
            fs::create_dir_all(output_dir).await?;
        }
        
        let tables = self.list_tables(true).await?;
        println!("📋 Found {} tables", tables.len());
        
        let mut generated_count = 0;
        let mut skipped_count = 0;
        
        for table in tables {
            // Filter tables if specified
            if !filter_tables.is_empty() && !filter_tables.contains(&table.name) {
                continue;
            }
            
            let schema_file = output_dir.join(format!("{}.yaml", table.name));
            
            // Skip if file exists and force is false
            if schema_file.exists() && !force {
                println!("⚠️  Skipping existing schema: {}", schema_file.display());
                skipped_count += 1;
                continue;
            }
            
            // Get detailed table description
            let description = self.describe_table(&table.name).await?;
            
            // Generate YAML schema
            let yaml_content = self.generate_table_schema_yaml(&description).await?;
            
            // Write schema file
            fs::write(&schema_file, yaml_content).await?;
            println!("✅ Generated schema: {}", schema_file.display());
            generated_count += 1;
        }
        
        // Generate _meta.yaml
        let meta_file = output_dir.join("_meta.yaml");
        if !meta_file.exists() || force {
            let meta_content = self.generate_meta_yaml().await?;
            fs::write(&meta_file, meta_content).await?;
            println!("✅ Generated metadata: {}", meta_file.display());
        }
        
        println!("📊 Generated {} schemas, skipped {}", generated_count, skipped_count);
        Ok(())
    }
    
    async fn generate_meta_yaml(&self) -> Result<String> {
        let mut yaml = String::new();
        yaml.push_str("# Global schema configuration - Generated from PostgreSQL database\n\n");
        yaml.push_str("version: \"1.0\"\n");
        yaml.push_str("database_type: postgres\n");
        yaml.push_str(&format!("database_name: {}\n", self.db_name));
        yaml.push_str(&format!("description: \"Schema generated from PostgreSQL database '{}'\"\n", self.db_name));
        yaml.push_str("ai_context: \"Generated schemas from existing PostgreSQL database structure with intelligent field mapping\"\n");
        yaml.push_str("\n");
        yaml.push_str("# Global field defaults\n");
        yaml.push_str("field_defaults:\n");
        yaml.push_str("  string:\n");
        yaml.push_str("    max_length: 255\n");
        yaml.push_str("    charset: utf8\n");
        yaml.push_str("    \n");
        yaml.push_str("  timestamp:\n");
        yaml.push_str("    default: now\n");
        yaml.push_str("    on_update: now\n");
        yaml.push_str("    \n");
        yaml.push_str("# Code generation settings\n");
        yaml.push_str("generation:\n");
        yaml.push_str("  base_class: \"BaseModel\"\n");
        yaml.push_str("  use_traits: [HasTimestamps]\n");
        yaml.push_str("\n");
        
        Ok(yaml)
    }
    
    async fn get_database_name(&self) -> Result<String> {
        Ok(self.db_name.clone())
    }
    
    async fn export_data(&self, query: &str, _format: &str) -> Result<String> {
        let _rows = sqlx::query(query).fetch_all(&self.pool).await?;
        // TODO: Implement actual data export formatting
        Ok("[]".to_string())
    }
}