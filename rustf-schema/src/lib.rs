//! RustF Schema - Database schema definitions and code generation
//! 
//! This crate provides a schema definition system that ensures AI agents
//! maintain consistency with database structures, field names, and types.
//! 
//! # Features
//! 
//! - **AI-friendly schema format** with rich metadata and hints
//! - **YAML-based schema definitions** that are human-readable
//! - **Schema validation** with relationship and constraint checking
//! - **Code generation** for SQLx models with business logic separation
//! - **Template-based generation** supporting multiple target formats
//! 
//! # Example
//! 
//! ```rust
//! use rustf_schema::Schema;
//! use std::path::Path;
//! 
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Load schema from YAML files
//! #[cfg(feature = "tokio")]
//! let schema = Schema::load_from_directory(Path::new("schemas")).await?;
//! 
//! // Validate schema consistency
//! schema.validate()?;
//! 
//! // Generate SQLx models
//! #[cfg(feature = "codegen")]
//! {
//!     use rustf_schema::codegen::SqlxGenerator;
//!     let generator = SqlxGenerator::new()?;
//!     let code = generator.generate_table("User", &schema.tables["User"], &schema)?;
//!     println!("{}", code);
//! }
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

pub mod parser;
pub mod types;
pub mod validator;

#[cfg(feature = "codegen")]
pub mod codegen;

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
    
    #[error("Code generation error: {0}")]
    CodeGen(String),
    
    #[error("Consistency error: {0}")]
    Consistency(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[cfg(feature = "codegen")]
    #[error("Template error: {0}")]
    Template(#[from] handlebars::RenderError),
}

pub type Result<T> = std::result::Result<T, SchemaError>;

/// Validation result containing all errors and warnings
#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Create a new empty validation result
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
    
    /// Add a validation error
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }
    
    /// Add a validation warning
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
    
    /// Check if there are any validation errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    
    /// Check if there are any validation warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
    
    /// Merge another validation result into this one
    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
    
    /// Get total count of issues (errors + warnings)
    pub fn total_issues(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }
    
    /// Convert to a single error if there are validation errors
    pub fn into_result(self) -> Result<()> {
        if self.has_errors() {
            Err(SchemaError::Validation(
                format!("Schema validation failed with {} error(s):\n{}", 
                    self.errors.len(),
                    self.errors.join("\n"))
            ))
        } else {
            Ok(())
        }
    }
}

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
    pub database_type: String,
    pub database_name: String,
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
    #[cfg(feature = "tokio")]
    pub async fn load_from_directory(path: &std::path::Path) -> Result<Self> {
        SchemaParser::parse_directory(path).await
    }
    
    /// Load schema from a directory (sync version)
    #[cfg(not(feature = "tokio"))]
    pub fn load_from_directory_sync(path: &std::path::Path) -> Result<Self> {
        SchemaParser::parse_directory_sync(path)
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
    
    /// Get schema checksum for consistency validation
    pub fn checksum(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        
        // Sort tables for consistent hashing
        let mut sorted_tables: Vec<_> = self.tables.iter().collect();
        sorted_tables.sort_by_key(|(name, _)| *name);
        
        for (name, table) in sorted_tables {
            name.hash(&mut hasher);
            table.version.hash(&mut hasher);
            
            // Hash field types and constraints
            let mut sorted_fields: Vec<_> = table.fields.iter().collect();
            sorted_fields.sort_by_key(|(name, _)| *name);
            
            for (field_name, field) in sorted_fields {
                field_name.hash(&mut hasher);
                field.field_type.base_type().hash(&mut hasher);
                field.constraints.primary_key.hash(&mut hasher);
                field.constraints.unique.hash(&mut hasher);
                field.constraints.required.hash(&mut hasher);
            }
        }
        
        format!("{:x}", hasher.finish())
    }
    
    /// Validate consistency with generated code checksums
    pub fn validate_consistency(&self, generated_checksums: &HashMap<String, String>) -> Result<()> {
        let schema_checksum = self.checksum();
        
        for (table_name, generated_checksum) in generated_checksums {
            if !self.tables.contains_key(table_name) {
                return Err(SchemaError::Consistency(
                    format!("Generated code exists for non-existent table '{}'", table_name)
                ));
            }
            
            if generated_checksum != &schema_checksum {
                return Err(SchemaError::Consistency(
                    format!("Schema checksum mismatch for table '{}'. Schema may have changed since code generation.", table_name)
                ));
            }
        }
        
        // Check for tables that should have generated code but don't
        for table_name in self.tables.keys() {
            if !generated_checksums.contains_key(table_name) {
                return Err(SchemaError::Consistency(
                    format!("No generated code found for table '{}'. Run code generation.", table_name)
                ));
            }
        }
        
        Ok(())
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}