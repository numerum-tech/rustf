//! SQLx code generator for RustF schema
//! 
//! Generates Rust structs and implementations for SQLx with:
//! - Model structs with proper types
//! - CRUD operations
//! - Query builders
//! - Relationship helpers
//! - Business logic hooks

use crate::{Schema, Table, FieldType, Result};
use crate::codegen::{CodeGenerator, TemplateGenerator, GenerationContext};
use std::collections::HashMap;

/// SQLx code generator
pub struct SqlxGenerator {
    template_generator: TemplateGenerator,
}

impl SqlxGenerator {
    /// Create a new SQLx generator
    pub fn new() -> Result<Self> {
        let mut template_generator = TemplateGenerator::new();
        
        // Register SQLx templates
        template_generator.register_template("model", include_str!("templates/sqlx_model.hbs"))?;
        template_generator.register_template("crud", include_str!("templates/sqlx_crud.hbs"))?;
        template_generator.register_template("relations", include_str!("templates/sqlx_relations.hbs"))?;
        
        Ok(Self { template_generator })
    }
    
    /// Generate a complete model file with CRUD operations
    pub fn generate_model(&self, table_name: &str, table: &Table, schema: &Schema) -> Result<String> {
        let mut context = GenerationContext {
            schema: schema.clone(),
            table: table.clone(),
            table_name: table_name.to_string(),
            variables: HashMap::new(),
        };
        
        // Add SQLx-specific variables
        let rust_fields = self.generate_rust_fields(table)?;
        context.variables.insert("rust_fields".to_string(), 
            serde_json::to_value(&rust_fields)?);
        context.variables.insert("primary_key".to_string(), 
            serde_json::to_value(self.find_primary_key(table))?);
        context.variables.insert("insert_fields".to_string(), 
            serde_json::to_value(self.generate_insert_fields(table)?)?);
        context.variables.insert("update_fields".to_string(), 
            serde_json::to_value(self.generate_update_fields(table)?)?);
        
        // Add type constants for AI agent reference
        context.variables.insert("type_constants".to_string(),
            serde_json::to_value(self.generate_type_constants(&rust_fields)?)?);
        
        // Add dependency flags for imports in Types module
        context.variables.insert("needs_chrono".to_string(),
            serde_json::to_value(self.needs_chrono_import(&rust_fields))?);
        context.variables.insert("needs_decimal".to_string(),
            serde_json::to_value(self.needs_decimal_import(&rust_fields))?);
        context.variables.insert("needs_uuid".to_string(),
            serde_json::to_value(self.needs_uuid_import(&rust_fields))?);
        context.variables.insert("needs_json".to_string(),
            serde_json::to_value(self.needs_json_import(&rust_fields))?);
        
        self.template_generator.render("model", &context)
    }
    
    /// Generate CRUD operations for a table
    pub fn generate_crud(&self, table_name: &str, table: &Table, schema: &Schema) -> Result<String> {
        let context = GenerationContext {
            schema: schema.clone(),
            table: table.clone(),
            table_name: table_name.to_string(),
            variables: HashMap::new(),
        };
        
        self.template_generator.render("crud", &context)
    }
    
    /// Generate relationship helpers
    pub fn generate_relations(&self, table_name: &str, table: &Table, schema: &Schema) -> Result<String> {
        let context = GenerationContext {
            schema: schema.clone(),
            table: table.clone(),
            table_name: table_name.to_string(),
            variables: HashMap::new(),
        };
        
        self.template_generator.render("relations", &context)
    }
    
    /// Convert field type to Rust type
    pub fn field_type_to_rust(&self, field_type: &FieldType, nullable: bool) -> String {
        let base_type = match field_type {
            FieldType::Simple(t) => {
                match t.as_str() {
                    "int" | "integer" => "i32",
                    "bigint" => "i64",
                    "serial" => "i32",
                    "string" | "varchar" | "text" => "String",
                    "decimal" => "rust_decimal::Decimal",
                    "float" => "f32",
                    "double" => "f64",
                    "boolean" | "bool" => "bool",
                    "timestamp" | "datetime" => "chrono::DateTime<chrono::Utc>",
                    "date" => "chrono::NaiveDate",
                    "time" => "chrono::NaiveTime",
                    "json" | "jsonb" => "serde_json::Value",
                    "uuid" => "uuid::Uuid",
                    "blob" => "Vec<u8>",
                    _ => "String",
                }
            },
            FieldType::Parameterized { base_type, .. } => {
                match base_type.as_str() {
                    "string" | "varchar" => "String",
                    "decimal" => "rust_decimal::Decimal",
                    _ => "String",
                }
            },
            FieldType::Enum { .. } => "String", // TODO: Generate actual enum types
            FieldType::Json { .. } => "serde_json::Value",
        };
        
        if nullable {
            format!("Option<{}>", base_type)
        } else {
            base_type.to_string()
        }
    }
    
    /// Convert field type to SQLx type annotation
    pub fn field_type_to_sqlx(&self, field_type: &FieldType) -> String {
        match field_type {
            FieldType::Simple(t) => {
                match t.as_str() {
                    "timestamp" | "datetime" => "TIMESTAMPTZ".to_string(),
                    "json" | "jsonb" => "JSON".to_string(), 
                    "uuid" => "UUID".to_string(),
                    "blob" => "BYTEA".to_string(),
                    _ => field_type.base_type().to_uppercase(),
                }
            },
            FieldType::Parameterized { base_type, .. } => {
                base_type.to_uppercase()
            },
            FieldType::Enum { .. } => "TEXT".to_string(),
            FieldType::Json { .. } => "JSON".to_string(),
        }
    }
    
    fn generate_rust_fields(&self, table: &Table) -> Result<Vec<RustField>> {
        let mut fields = Vec::new();
        
        for (field_name, field) in &table.fields {
            let rust_type = self.field_type_to_rust(&field.field_type, field.constraints.nullable.unwrap_or(false));
            let sqlx_type = self.field_type_to_sqlx(&field.field_type);
            
            fields.push(RustField {
                name: field_name.clone(),
                rust_type,
                sqlx_type,
                nullable: field.constraints.nullable.unwrap_or(false),
                primary_key: field.constraints.primary_key.unwrap_or(false),
                auto_increment: matches!(field.constraints.auto, Some(crate::types::AutoGenerate::Boolean(true))),
                unique: field.constraints.unique.unwrap_or(false),
                default: field.constraints.default.clone(),
                ai_hint: field.ai.clone(),
            });
        }
        
        Ok(fields)
    }
    
    fn find_primary_key(&self, table: &Table) -> Option<String> {
        for (field_name, field) in &table.fields {
            if field.constraints.primary_key.unwrap_or(false) {
                return Some(field_name.clone());
            }
        }
        None
    }
    
    fn generate_insert_fields(&self, table: &Table) -> Result<Vec<String>> {
        let mut fields = Vec::new();
        
        for (field_name, field) in &table.fields {
            // Skip auto-increment primary keys
            if field.constraints.primary_key.unwrap_or(false) && 
               matches!(field.constraints.auto, Some(crate::types::AutoGenerate::Boolean(true))) {
                continue;
            }
            
            fields.push(field_name.clone());
        }
        
        Ok(fields)
    }
    
    fn generate_update_fields(&self, table: &Table) -> Result<Vec<String>> {
        let mut fields = Vec::new();
        
        for (field_name, field) in &table.fields {
            // Skip primary keys from updates
            if field.constraints.primary_key.unwrap_or(false) {
                continue;
            }
            
            fields.push(field_name.clone());
        }
        
        Ok(fields)
    }
    
    /// Generate type constants for AI agent reference
    fn generate_type_constants(&self, rust_fields: &[RustField]) -> Result<Vec<TypeConstant>> {
        let mut type_constants = Vec::new();
        
        for field in rust_fields {
            type_constants.push(TypeConstant {
                name: field.name.clone(),
                rust_type: field.rust_type.clone(),
            });
        }
        
        Ok(type_constants)
    }
    
    /// Check if chrono import is needed in Types module
    fn needs_chrono_import(&self, rust_fields: &[RustField]) -> bool {
        rust_fields.iter().any(|field| {
            field.rust_type.contains("chrono::") || 
            field.rust_type.contains("DateTime") ||
            field.rust_type.contains("NaiveDate") ||
            field.rust_type.contains("NaiveTime")
        })
    }
    
    /// Check if decimal import is needed in Types module
    fn needs_decimal_import(&self, rust_fields: &[RustField]) -> bool {
        rust_fields.iter().any(|field| {
            field.rust_type.contains("rust_decimal::Decimal") ||
            field.rust_type.contains("Decimal")
        })
    }
    
    /// Check if UUID import is needed in Types module
    fn needs_uuid_import(&self, rust_fields: &[RustField]) -> bool {
        rust_fields.iter().any(|field| {
            field.rust_type.contains("uuid::Uuid") ||
            field.rust_type.contains("Uuid")
        })
    }
    
    /// Check if JSON import is needed in Types module
    fn needs_json_import(&self, rust_fields: &[RustField]) -> bool {
        rust_fields.iter().any(|field| {
            field.rust_type.contains("serde_json::Value") ||
            field.rust_type.contains("Value")
        })
    }
}

impl Default for SqlxGenerator {
    fn default() -> Self {
        Self::new().expect("Failed to create SQLx generator")
    }
}

impl CodeGenerator for SqlxGenerator {
    fn generate_table(&self, table_name: &str, table: &Table, schema: &Schema) -> Result<String> {
        self.generate_model(table_name, table, schema)
    }
}

/// Rust field representation for template generation
#[derive(Debug, Clone, serde::Serialize)]
pub struct RustField {
    pub name: String,
    pub rust_type: String,
    pub sqlx_type: String,
    pub nullable: bool,
    pub primary_key: bool,
    pub auto_increment: bool,
    pub unique: bool,
    pub default: Option<serde_json::Value>,
    pub ai_hint: Option<String>,
}

/// Type constant representation for AI agent reference
#[derive(Debug, Clone, serde::Serialize)]
pub struct TypeConstant {
    pub name: String,
    pub rust_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use std::collections::HashMap;
    
    #[test]
    fn test_field_type_conversion() {
        let generator = SqlxGenerator::new().unwrap();
        
        assert_eq!(generator.field_type_to_rust(&FieldType::Simple("integer".to_string()), false), "i32");
        assert_eq!(generator.field_type_to_rust(&FieldType::Simple("string".to_string()), true), "Option<String>");
        assert_eq!(generator.field_type_to_rust(&FieldType::Simple("timestamp".to_string()), false), "chrono::DateTime<chrono::Utc>");
    }
    
    #[test]
    fn test_primary_key_detection() {
        let generator = SqlxGenerator::new().unwrap();
        
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
        
        let id_field = Field {
            name: "id".to_string(),
            field_type: FieldType::Simple("integer".to_string()),
            lang_type: None,
            postgres_type_name: None,
            constraints: FieldConstraints {
                primary_key: Some(true),
                ..Default::default()
            },
            ai: None,
            example: None,
        };
        
        table.fields.insert("id".to_string(), id_field);
        
        assert_eq!(generator.find_primary_key(&table), Some("id".to_string()));
    }
}