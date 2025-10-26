//! Database schema definition system for AI-friendly consistency
//! 
//! This module provides a schema definition system that ensures AI agents
//! maintain consistency with database structures, field names, and types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

pub mod parser;
pub mod types;
pub mod validator;

pub use parser::SchemaParser;
pub use types::*;
pub use validator::SchemaValidator;

/// Schema system errors
#[derive(Error, Debug)]
pub enum SchemaError {
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Field not found: {table}.{field}")]
    FieldNotFound { table: String, field: String },
    
    #[error("Table not found: {0}")]
    TableNotFound(String),
    
    #[error("Invalid type: {0}")]
    InvalidType(String),
    
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

pub type Result<T> = std::result::Result<T, SchemaError>;

/// Main schema container holding all table definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    /// All tables indexed by name
    pub tables: HashMap<String, Table>,
    
    /// Global metadata
    pub meta: Option<SchemaMeta>,
}

/// Global schema metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMeta {
    pub version: String,
    pub database_type: Option<String>,
    pub database_name: Option<String>,
    pub description: Option<String>,
    pub ai_context: Option<String>,
}

impl Schema {
    /// Create a new empty schema
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
            meta: None,
        }
    }
    
    /// Load schema from a directory
    pub async fn load_from_directory(path: &std::path::Path) -> Result<Self> {
        SchemaParser::parse_directory(path).await
    }
    
    /// Validate the schema for consistency
    pub fn validate(&self) -> Result<()> {
        SchemaValidator::validate(self)
    }
    
    /// Get a table by name
    pub fn get_table(&self, name: &str) -> Option<&Table> {
        self.tables.get(name)
    }
    
    /// Get all table names
    pub fn table_names(&self) -> Vec<&str> {
        self.tables.keys().map(|s| s.as_str()).collect()
    }
    
    /// Resolve a field reference (e.g., "users.id")
    pub fn resolve_field_ref(&self, field_ref: &str) -> Result<(&Table, &Field)> {
        let parts: Vec<&str> = field_ref.split('.').collect();
        if parts.len() != 2 {
            return Err(SchemaError::Validation(
                format!("Invalid field reference: {}", field_ref)
            ));
        }
        
        // First try direct lookup by schema key (backward compatibility)
        if let Some(table) = self.tables.get(parts[0]) {
            let field = table.fields.get(parts[1])
                .ok_or_else(|| SchemaError::FieldNotFound {
                    table: parts[0].to_string(),
                    field: parts[1].to_string(),
                })?;
            return Ok((table, field));
        }
        
        // If not found, search by database table name
        let table = self.tables.values()
            .find(|t| t.table == parts[0])
            .ok_or_else(|| SchemaError::TableNotFound(parts[0].to_string()))?;
            
        let field = table.fields.get(parts[1])
            .ok_or_else(|| SchemaError::FieldNotFound {
                table: parts[0].to_string(),
                field: parts[1].to_string(),
            })?;
            
        Ok((table, field))
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_schema_creation() {
        let schema = Schema::new();
        assert!(schema.tables.is_empty());
        assert!(schema.meta.is_none());
    }
    
    #[test]
    fn test_field_resolution() {
        let mut schema = Schema::new();
        
        // Create a test table
        let mut table = Table {
            name: "users".to_string(),
            table: "users".to_string(),
            database_type: None,
            database_name: None,
            element_type: None,
            version: 1,
            description: None,
            tags: vec![],
            ai_context: None,
            fields: HashMap::new(),
            relations: Relations::default(),
            indexes: vec![],
            constraints: vec![],
        };
        
        // Add a field
        table.fields.insert("id".to_string(), Field {
            name: "id".to_string(),
            field_type: FieldType::Simple("int".to_string()),
            lang_type: Some("i32".to_string()),
            constraints: FieldConstraints::default(),
            ai: None,
            example: None,
        });
        
        schema.tables.insert("users".to_string(), table);
        
        // Test resolution
        let result = schema.resolve_field_ref("users.id");
        assert!(result.is_ok());
        
        let (table, field) = result.unwrap();
        assert_eq!(table.name, "users");
        assert_eq!(field.name, "id");
        
        // Test invalid references
        assert!(schema.resolve_field_ref("invalid.field").is_err());
        assert!(schema.resolve_field_ref("users.invalid").is_err());
        assert!(schema.resolve_field_ref("invalid").is_err());
    }
}