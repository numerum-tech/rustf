//! Schema validation system

use super::{Result, Schema, SchemaError, Table, FieldType, Field, AutoGenerate, ValidationResult};
use std::collections::HashSet;

/// Schema validator
pub struct SchemaValidator;

impl SchemaValidator {
    /// Validate the entire schema (legacy method for backward compatibility)
    pub fn validate(schema: &Schema) -> Result<()> {
        let result = Self::validate_comprehensive(schema)?;
        result.into_result()
    }
    
    /// Validate the entire schema and return all validation results
    pub fn validate_comprehensive(schema: &Schema) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();
        
        // 1. Validate each table individually
        for (name, table) in &schema.tables {
            let table_result = Self::validate_table_comprehensive(name, table);
            result.merge(table_result);
        }
        
        // 2. Validate foreign key references
        let fk_result = Self::validate_foreign_keys_comprehensive(schema);
        result.merge(fk_result);
        
        // 3. Validate relations
        let relations_result = Self::validate_relations_comprehensive(schema);
        result.merge(relations_result);
        
        // 4. Check for circular dependencies
        let circular_result = Self::check_circular_dependencies_comprehensive(schema);
        result.merge(circular_result);
        
        Ok(result)
    }
    
    /// Validate a single table
    pub fn validate_table(name: &str, table: &Table) -> Result<()> {
        // Check that table has at least one field
        if table.fields.is_empty() {
            return Err(SchemaError::Validation(
                format!("Table '{}' has no fields", name)
            ));
        }
        
        // Check that tables (but not views) have a primary key
        let is_view = table.element_type.as_deref() == Some("view");
        if !is_view {
            let has_primary_key = table.fields.values()
                .any(|field| field.constraints.primary_key == Some(true));
                
            if !has_primary_key {
                return Err(SchemaError::Validation(
                    format!("Table '{}' has no primary key", name)
                ));
            }
        }
        
        // Validate each field
        for (field_name, field) in &table.fields {
            Self::validate_field(name, field_name, field)?;
        }
        
        Ok(())
    }
    
    /// Validate a single table and collect all errors
    fn validate_table_comprehensive(name: &str, table: &Table) -> ValidationResult {
        let mut result = ValidationResult::new();
        
        // Check that table has at least one field
        if table.fields.is_empty() {
            result.add_error(format!("Table '{}' has no fields", name));
        }
        
        // Check that tables (but not views) have a primary key
        let is_view = table.element_type.as_deref() == Some("view");
        if !is_view {
            let has_primary_key = table.fields.values()
                .any(|field| field.constraints.primary_key == Some(true));
                
            if !has_primary_key {
                result.add_error(format!("Table '{}' has no primary key", name));
            }
        }
        
        // Validate each field
        for (field_name, field) in &table.fields {
            let field_result = Self::validate_field_comprehensive(name, field_name, field);
            result.merge(field_result);
        }
        
        result
    }
    
    /// Validate a single field
    fn validate_field(table_name: &str, field_name: &str, field: &Field) -> Result<()> {
        // Validate field type
        Self::validate_field_type(&field.field_type)?;
        
        // Check constraints consistency
        if field.constraints.required == Some(true) && field.constraints.nullable == Some(true) {
            return Err(SchemaError::Validation(
                format!("Field '{}.{}' cannot be both required and nullable", 
                    table_name, field_name)
            ));
        }
        
        // Validate auto generation
        if let Some(auto) = &field.constraints.auto {
            match auto {
                AutoGenerate::Boolean(true) => {
                    // Must be primary key or have specific type
                    if field.constraints.primary_key != Some(true) 
                        && !matches!(field.field_type.base_type(), "serial" | "uuid") {
                        return Err(SchemaError::Validation(
                            format!("Field '{}.{}' with auto=true must be primary key or serial/uuid type", 
                                table_name, field_name)
                        ));
                    }
                },
                AutoGenerate::Boolean(false) => {
                    // No validation needed for auto=false
                },
                AutoGenerate::Type(t) => {
                    match t.as_str() {
                        "create" | "update" => {
                            if !matches!(field.field_type.base_type(), "timestamp" | "datetime") {
                                return Err(SchemaError::Validation(
                                    format!("Field '{}.{}' with auto='{}' must be timestamp type", 
                                        table_name, field_name, t)
                                ));
                            }
                        },
                        _ => return Err(SchemaError::Validation(
                            format!("Invalid auto type '{}' for field '{}.{}'", 
                                t, table_name, field_name)
                        )),
                    }
                },
            }
        }
        
        // Validate foreign key constraints
        if let Some(fk) = &field.constraints.foreign_key {
            if !fk.contains('.') {
                return Err(SchemaError::Validation(
                    format!("Foreign key '{}' in field '{}.{}' must be in format 'table.field'", 
                        fk, table_name, field_name)
                ));
            }
        }
        
        Ok(())
    }
    
    /// Validate a single field and collect all errors
    fn validate_field_comprehensive(table_name: &str, field_name: &str, field: &Field) -> ValidationResult {
        let mut result = ValidationResult::new();
        
        // Validate field type
        if let Err(e) = Self::validate_field_type(&field.field_type) {
            result.add_error(format!("{}", e));
        }
        
        // Check constraints consistency
        if field.constraints.required == Some(true) && field.constraints.nullable == Some(true) {
            result.add_error(format!("Field '{}.{}' cannot be both required and nullable", 
                table_name, field_name));
        }
        
        // Validate auto generation
        if let Some(auto) = &field.constraints.auto {
            match auto {
                AutoGenerate::Boolean(true) => {
                    // Must be primary key or have specific type
                    if field.constraints.primary_key != Some(true) 
                        && !matches!(field.field_type.base_type(), "serial" | "uuid") {
                        result.add_error(format!("Field '{}.{}' with auto=true must be primary key or serial/uuid type", 
                            table_name, field_name));
                    }
                },
                AutoGenerate::Boolean(false) => {
                    // No validation needed for auto=false
                },
                AutoGenerate::Type(t) => {
                    match t.as_str() {
                        "create" | "update" => {
                            if !matches!(field.field_type.base_type(), "timestamp" | "datetime") {
                                result.add_error(format!("Field '{}.{}' with auto='{}' must be timestamp type", 
                                    table_name, field_name, t));
                            }
                        },
                        _ => {
                            result.add_error(format!("Invalid auto type '{}' for field '{}.{}'", 
                                t, table_name, field_name));
                        }
                    }
                },
            }
        }
        
        // Validate foreign key constraints
        if let Some(fk) = &field.constraints.foreign_key {
            if !fk.contains('.') {
                result.add_error(format!("Foreign key '{}' in field '{}.{}' must be in format 'table.field'", 
                    fk, table_name, field_name));
            }
        }
        
        result
    }
    
    /// Validate field type
    pub fn validate_field_type(field_type: &FieldType) -> Result<()> {
        match field_type {
            FieldType::Simple(type_name) => {
                // Validate known types
                match type_name.as_str() {
                    "int" | "integer" | "serial" | "bigint" | "smallint" | "tinyint" | "mediumint" |
                    "string" | "text" | "tinytext" | "mediumtext" | "longtext" | "varchar" |
                    "decimal" | "float" | "double" |
                    "boolean" | "bool" |
                    "timestamp" | "datetime" | "date" | "time" |
                    "json" | "jsonb" | "uuid" | "blob" | "enum" |
                    "inet" | "cidr" => Ok(()),
                    _ => Err(SchemaError::InvalidType(format!("Unknown type: {}", type_name)))
                }
            },
            FieldType::Parameterized { base_type, params } => {
                // Validate parameterized types
                match base_type.as_str() {
                    "string" | "varchar" => {
                        if params.len() != 1 {
                            return Err(SchemaError::InvalidType(
                                format!("Type '{}' requires exactly 1 parameter", base_type)
                            ));
                        }
                    },
                    "decimal" | "numeric" => {
                        if params.len() != 2 {
                            return Err(SchemaError::InvalidType(
                                format!("Type '{}' requires exactly 2 parameters (precision, scale)", base_type)
                            ));
                        }
                    },
                    _ => return Err(SchemaError::InvalidType(
                        format!("Type '{}' cannot be parameterized", base_type)
                    )),
                }
                Ok(())
            },
            FieldType::Enum { values, transitions, .. } => {
                if values.is_empty() {
                    return Err(SchemaError::InvalidType("Enum type must have at least one value".to_string()));
                }
                
                // Validate transitions if present
                if let Some(trans) = transitions {
                    for (from, to_states) in trans {
                        if !values.contains(from) {
                            return Err(SchemaError::InvalidType(
                                format!("Transition from state '{}' not in enum values", from)
                            ));
                        }
                        for to in to_states {
                            if !values.contains(to) {
                                return Err(SchemaError::InvalidType(
                                    format!("Transition to state '{}' not in enum values", to)
                                ));
                            }
                        }
                    }
                }
                Ok(())
            },
            FieldType::Json { .. } => Ok(()),
        }
    }
    
    /// Validate foreign key references
    #[allow(dead_code)]
    fn validate_foreign_keys(schema: &Schema) -> Result<()> {
        for (table_name, table) in &schema.tables {
            for (field_name, field) in &table.fields {
                if let Some(fk) = &field.constraints.foreign_key {
                    let (_target_table, target_field) = schema.resolve_field_ref(fk)
                        .map_err(|_| SchemaError::Validation(
                            format!("Foreign key '{}' in '{}.{}' references non-existent field", 
                                fk, table_name, field_name)
                        ))?;
                    
                    // Check that target field is suitable for foreign key
                    if target_field.constraints.primary_key != Some(true) 
                        && target_field.constraints.unique != Some(true) {
                        return Err(SchemaError::Validation(
                            format!("Foreign key '{}' in '{}.{}' must reference a primary key or unique field", 
                                fk, table_name, field_name)
                        ));
                    }
                }
            }
        }
        Ok(())
    }
    
    /// Validate foreign key references and collect all errors
    fn validate_foreign_keys_comprehensive(schema: &Schema) -> ValidationResult {
        let mut result = ValidationResult::new();
        
        for (table_name, table) in &schema.tables {
            for (field_name, field) in &table.fields {
                if let Some(fk) = &field.constraints.foreign_key {
                    match schema.resolve_field_ref(fk) {
                        Ok((_target_table, target_field)) => {
                            // Check that target field is suitable for foreign key
                            if target_field.constraints.primary_key != Some(true) 
                                && target_field.constraints.unique != Some(true) {
                                result.add_error(format!("Foreign key '{}' in '{}.{}' must reference a primary key or unique field", 
                                    fk, table_name, field_name));
                            }
                        },
                        Err(_) => {
                            result.add_error(format!("Foreign key '{}' in '{}.{}' references non-existent field", 
                                fk, table_name, field_name));
                        }
                    }
                }
            }
        }
        
        result
    }
    
    /// Validate relations
    #[allow(dead_code)]
    fn validate_relations(schema: &Schema) -> Result<()> {
        for (table_name, table) in &schema.tables {
            // Validate belongs_to relations
            if let Some(belongs_to) = &table.relations.belongs_to {
                for (relation_name, relation) in belongs_to {
                    // Check that target model exists
                    if !schema.tables.contains_key(&relation.model) {
                        return Err(SchemaError::Validation(
                            format!("Relation '{}.{}' references non-existent model '{}'", 
                                table_name, relation_name, relation.model)
                        ));
                    }
                    
                    // Check that local field exists
                    if !table.fields.contains_key(&relation.local_field) {
                        return Err(SchemaError::Validation(
                            format!("Relation '{}.{}' references non-existent local field '{}'", 
                                table_name, relation_name, relation.local_field)
                        ));
                    }
                    
                    // Check that foreign field exists
                    let target_table = &schema.tables[&relation.model];
                    if !target_table.fields.contains_key(&relation.foreign_field) {
                        return Err(SchemaError::Validation(
                            format!("Relation '{}.{}' references non-existent foreign field '{}.{}'", 
                                table_name, relation_name, relation.model, relation.foreign_field)
                        ));
                    }
                }
            }
            
            // Validate has_many relations
            if let Some(has_many) = &table.relations.has_many {
                for (relation_name, relation) in has_many {
                    // Check that target model exists
                    if !schema.tables.contains_key(&relation.model) {
                        return Err(SchemaError::Validation(
                            format!("Relation '{}.{}' references non-existent model '{}'", 
                                table_name, relation_name, relation.model)
                        ));
                    }
                    
                    // Check that local field exists
                    if !table.fields.contains_key(&relation.local_field) {
                        return Err(SchemaError::Validation(
                            format!("Relation '{}.{}' references non-existent local field '{}'", 
                                table_name, relation_name, relation.local_field)
                        ));
                    }
                    
                    // Check that foreign field exists
                    let target_table = &schema.tables[&relation.model];
                    if !target_table.fields.contains_key(&relation.foreign_field) {
                        return Err(SchemaError::Validation(
                            format!("Relation '{}.{}' references non-existent foreign field '{}.{}'", 
                                table_name, relation_name, relation.model, relation.foreign_field)
                        ));
                    }
                }
            }
        }
        Ok(())
    }
    
    /// Check for circular dependencies in relations
    #[allow(dead_code)]
    fn check_circular_dependencies(schema: &Schema) -> Result<()> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        
        for table_name in schema.tables.keys() {
            if !visited.contains(table_name) {
                if Self::has_cycle(schema, table_name, &mut visited, &mut rec_stack)? {
                    return Err(SchemaError::CircularDependency(
                        format!("Circular dependency detected involving table '{}'", table_name)
                    ));
                }
            }
        }
        
        Ok(())
    }
    
    /// Recursive cycle detection
    fn has_cycle(
        schema: &Schema, 
        table_name: &str, 
        visited: &mut HashSet<String>, 
        rec_stack: &mut HashSet<String>
    ) -> Result<bool> {
        visited.insert(table_name.to_string());
        rec_stack.insert(table_name.to_string());
        
        if let Some(table) = schema.tables.get(table_name) {
            // Check belongs_to relations (these create dependencies)
            if let Some(belongs_to) = &table.relations.belongs_to {
                for relation in belongs_to.values() {
                    let target = &relation.model;
                    
                    // Skip self-references (they are valid for hierarchical structures)
                    if target == table_name {
                        continue;
                    }
                    
                    if !visited.contains(target) {
                        if Self::has_cycle(schema, target, visited, rec_stack)? {
                            return Ok(true);
                        }
                    } else if rec_stack.contains(target) {
                        return Ok(true);
                    }
                }
            }
        }
        
        rec_stack.remove(table_name);
        Ok(false)
    }
    
    /// Validate relations and collect all errors
    fn validate_relations_comprehensive(schema: &Schema) -> ValidationResult {
        let mut result = ValidationResult::new();
        
        for (table_name, table) in &schema.tables {
            // Validate belongs_to relations
            if let Some(belongs_to) = &table.relations.belongs_to {
                for (relation_name, relation) in belongs_to {
                    // Check that target model exists
                    if !schema.tables.contains_key(&relation.model) {
                        result.add_error(format!("Relation '{}.{}' references non-existent model '{}'", 
                            table_name, relation_name, relation.model));
                    } else {
                        // Check that local field exists
                        if !table.fields.contains_key(&relation.local_field) {
                            result.add_error(format!("Relation '{}.{}' references non-existent local field '{}'", 
                                table_name, relation_name, relation.local_field));
                        }
                        
                        // Check that foreign field exists
                        let target_table = &schema.tables[&relation.model];
                        if !target_table.fields.contains_key(&relation.foreign_field) {
                            result.add_error(format!("Relation '{}.{}' references non-existent foreign field '{}.{}'", 
                                table_name, relation_name, relation.model, relation.foreign_field));
                        }
                    }
                }
            }
            
            // Validate has_many relations
            if let Some(has_many) = &table.relations.has_many {
                for (relation_name, relation) in has_many {
                    // Check that target model exists
                    if !schema.tables.contains_key(&relation.model) {
                        result.add_error(format!("HasMany relation '{}.{}' references non-existent model '{}'", 
                            table_name, relation_name, relation.model));
                    } else {
                        // Check that local field exists
                        if !table.fields.contains_key(&relation.local_field) {
                            result.add_error(format!("HasMany relation '{}.{}' references non-existent local field '{}'", 
                                table_name, relation_name, relation.local_field));
                        }
                        
                        // Check that foreign field exists
                        let target_table = &schema.tables[&relation.model];
                        if !target_table.fields.contains_key(&relation.foreign_field) {
                            result.add_error(format!("HasMany relation '{}.{}' references non-existent foreign field '{}.{}'", 
                                table_name, relation_name, relation.model, relation.foreign_field));
                        }
                    }
                }
            }
        }
        
        result
    }
    
    /// Check for circular dependencies and collect all errors
    fn check_circular_dependencies_comprehensive(schema: &Schema) -> ValidationResult {
        let mut result = ValidationResult::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        
        for table_name in schema.tables.keys() {
            if !visited.contains(table_name) {
                if let Err(_cycle_error) = Self::has_cycle(schema, table_name, &mut visited, &mut rec_stack) {
                    result.add_error(format!("Circular dependency detected involving table '{}'", table_name));
                    // Don't clear visited - it should persist across the entire traversal
                    // rec_stack is managed by has_cycle function itself
                }
            }
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use std::collections::HashMap;
    
    #[test]
    fn test_validate_simple_table() {
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
        
        // Add primary key field
        table.fields.insert("id".to_string(), Field {
            name: "id".to_string(),
            field_type: FieldType::Simple("int".to_string()),
            lang_type: None,
            postgres_type_name: None,
            constraints: FieldConstraints {
                primary_key: Some(true),
                auto: Some(AutoGenerate::Boolean(true)),
                ..Default::default()
            },
            ai: None,
            example: None,
        });
        
        let result = SchemaValidator::validate_table("users", &table);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_validate_table_without_primary_key() {
        let mut table = Table {
            name: "test".to_string(),
            table: "test".to_string(),
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
        
        // Add field without primary key
        table.fields.insert("name".to_string(), Field {
            name: "name".to_string(),
            field_type: FieldType::Simple("string".to_string()),
            lang_type: None,
            postgres_type_name: None,
            constraints: FieldConstraints::default(),
            ai: None,
            example: None,
        });
        
        let result = SchemaValidator::validate_table("test", &table);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no primary key"));
    }
}