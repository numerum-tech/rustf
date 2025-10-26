//! Common structures and utilities for database introspection

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TableInfo {
    pub name: String,
    pub schema: Option<String>,
    pub table_type: String,
    pub row_count: Option<i64>,
    pub size_bytes: Option<i64>,
    pub comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub column_type: Option<String>,  // Full column type for enum values
    pub postgres_type_name: Option<String>,  // PostgreSQL enum type name (e.g., "user_role")
    pub is_nullable: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
    pub is_unique: bool,
    pub is_foreign_key: bool,
    pub foreign_table: Option<String>,
    pub foreign_column: Option<String>,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
    pub comment: Option<String>,
    pub max_length: Option<i32>,
    pub numeric_precision: Option<i32>,
    pub numeric_scale: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableDescription {
    pub table: TableInfo,
    pub columns: Vec<ColumnInfo>,
    pub indexes: Vec<IndexInfo>,
    pub constraints: Vec<ConstraintInfo>,
    pub triggers: Vec<TriggerInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub is_primary: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConstraintInfo {
    pub name: String,
    pub constraint_type: String,
    pub columns: Vec<String>,
    pub foreign_table: Option<String>,
    pub foreign_columns: Option<Vec<String>>,
    pub check_expression: Option<String>,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TriggerInfo {
    pub name: String,
    pub event: String,  // INSERT, UPDATE, DELETE
    pub timing: String, // BEFORE, AFTER
    pub for_each: String, // ROW, STATEMENT
    pub condition: Option<String>,
    pub body: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug)]
pub enum DatabaseType {
    PostgreSQL,
    MySQL,
    SQLite,
}

impl DatabaseType {
    pub fn from_url(url: &str) -> Result<Self> {
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            Ok(DatabaseType::PostgreSQL)
        } else if url.starts_with("mysql://") {
            Ok(DatabaseType::MySQL)
        } else if url.starts_with("sqlite://") {
            Ok(DatabaseType::SQLite)
        } else {
            anyhow::bail!("Unsupported database URL: {}", url)
        }
    }
}

/// Convert table name to PascalCase for model names
pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

/// Generate AI context hint based on table name
pub fn generate_table_ai_context(table_name: &str) -> String {
    let name_lower = table_name.to_lowercase();
    
    if name_lower.contains("user") || name_lower.contains("account") {
        "User authentication and profile management. Handle passwords securely with bcrypt."
    } else if name_lower.contains("auth") || name_lower.contains("session") {
        "Authentication and session management. Ensure secure token handling."
    } else if name_lower.contains("log") || name_lower.contains("audit") {
        "Audit trail and activity logging. Append-only, never update or delete records."
    } else if name_lower.contains("config") || name_lower.contains("setting") {
        "Application configuration and settings. Cache frequently accessed values."
    } else if name_lower.contains("payment") || name_lower.contains("transaction") {
        "Financial transactions. Ensure ACID compliance and proper decimal precision."
    } else if name_lower.contains("product") || name_lower.contains("item") {
        "Product catalog management. Consider caching for frequently accessed items."
    } else if name_lower.contains("order") || name_lower.contains("purchase") {
        "Order processing and fulfillment. Maintain state consistency."
    } else if name_lower.contains("comment") || name_lower.contains("review") {
        "User-generated content. Validate and sanitize input."
    } else if name_lower.contains("message") || name_lower.contains("notification") {
        "Messaging and notifications. Consider queueing for delivery."
    } else {
        "Database entity for {} management and data storage"
    }.to_string().replace("{}", table_name)
}

/// Generate AI hint for field based on its name and type
pub fn generate_field_ai_hint(field_name: &str, field_type: &str, is_foreign_key: bool) -> String {
    let name_lower = field_name.to_lowercase();
    
    if is_foreign_key {
        return format!("Foreign key reference to {} table", field_name.trim_end_matches("_id"));
    }
    
    if name_lower.contains("password") {
        "Always store as bcrypt hash. Never store plain passwords!"
    } else if name_lower.contains("email") {
        "Valid email format required. Used for authentication and communication."
    } else if name_lower.contains("phone") || name_lower.contains("mobile") {
        "Phone number with international format support"
    } else if name_lower.contains("url") || name_lower.contains("link") {
        "URL field - validate format and accessibility"
    } else if name_lower.contains("token") || name_lower.contains("secret") {
        "Sensitive token - store securely and never log"
    } else if name_lower.contains("created_at") {
        "Record creation timestamp - automatically set on insert"
    } else if name_lower.contains("updated_at") {
        "Last modification timestamp - automatically updated on change"
    } else if name_lower.contains("deleted_at") {
        "Soft deletion timestamp - null means active record"
    } else if name_lower.contains("is_active") || name_lower.contains("active") {
        "Status flag - false indicates soft deletion or deactivation"
    } else if name_lower.contains("verified") {
        "Verification status flag for validation workflows"
    } else if name_lower.contains("expires") {
        "Expiration timestamp - check before use"
    } else if name_lower.contains("count") || name_lower.contains("quantity") {
        "Integer value for counting or quantity tracking"
    } else if name_lower.contains("amount") || name_lower.contains("price") {
        "Monetary value - use proper decimal precision"
    } else if name_lower.contains("status") || name_lower.contains("state") {
        "Status indicator - validate against allowed values"
    } else if name_lower.contains("description") {
        "Detailed description text - supports rich formatting"
    } else if name_lower.contains("name") || name_lower.contains("title") {
        "Display name for user interface and identification"
    } else if name_lower.contains("code") || name_lower.contains("identifier") {
        "Unique identifier code - ensure uniqueness constraints"
    } else if name_lower.contains("json") || field_type.contains("json") {
        "Structured JSON data - validate schema before storage"
    } else if field_type.contains("bool") {
        "Boolean flag for binary state tracking"
    } else if field_type.contains("enum") {
        "Enumerated value - validate against allowed options"
    } else if field_type.contains("text") {
        "Large text field for extended content"
    } else if field_type.contains("decimal") {
        "Precise decimal number for financial calculations"
    } else if field_type.contains("timestamp") || field_type.contains("datetime") {
        "Timestamp field for temporal data tracking"
    } else if field_type.contains("date") {
        return format!("Database field of type {}", field_type);
    } else if field_type.contains("int") {
        "Integer value for counting or identification"
    } else if field_type.contains("varchar") || field_type.contains("string") {
        "String field with length constraints"
    } else {
        return format!("Database field of type {}", field_type);
    }.to_string()
}

/// Escape field names that conflict with YAML schema keywords
pub fn escape_yaml_field_name(field_name: &str) -> String {
    match field_name {
        "type" => "field_type".to_string(),
        "enum" => "enum_field".to_string(),
        "values" => "values_field".to_string(),
        _ => field_name.to_string(),
    }
}

/// Generate YAML for table relations based on foreign keys
pub fn generate_relations_yaml(columns: &[ColumnInfo]) -> String {
    let mut yaml = String::new();
    let mut belongs_to_relations = Vec::new();
    
    for column in columns {
        if column.is_foreign_key {
            if let (Some(foreign_table), Some(foreign_column)) = (&column.foreign_table, &column.foreign_column) {
                let relation_name = column.name.trim_end_matches("_id");
                belongs_to_relations.push((relation_name, foreign_table, &column.name, foreign_column));
            }
        }
    }
    
    if !belongs_to_relations.is_empty() {
        yaml.push_str("    belongs_to:\n");
        for (name, table, local, foreign) in belongs_to_relations {
            yaml.push_str(&format!("      {}:\n", name));
            yaml.push_str(&format!("        model: {}\n", to_pascal_case(table)));
            yaml.push_str(&format!("        local_field: {}\n", local));
            yaml.push_str(&format!("        foreign_field: {}\n", foreign));
            yaml.push_str(&format!("        ai: \"Reference to {} for relational data integrity\"\n", table));
        }
    }
    
    yaml
}