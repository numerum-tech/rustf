//! Schema parser for YAML files

use super::{Result, Schema, SchemaError, SchemaMeta, Table};
use std::collections::HashMap;
use std::path::Path;

#[cfg(feature = "tokio")]
use tokio::fs;

/// Schema parser
pub struct SchemaParser;

impl SchemaParser {
    /// Parse a single YAML file
    #[cfg(feature = "tokio")]
    pub async fn parse_file(path: &Path) -> Result<HashMap<String, Table>> {
        let contents = fs::read_to_string(path).await?;
        let tables: HashMap<String, Table> = serde_yaml::from_str(&contents)
            .map_err(|e| SchemaError::Parse(format!("Failed to parse YAML file '{}': {}", path.display(), e)))?;
        Ok(tables)
    }
    
    /// Parse a single YAML file (sync version)
    #[cfg(not(feature = "tokio"))]
    pub fn parse_file_sync(path: &Path) -> Result<HashMap<String, Table>> {
        let contents = std::fs::read_to_string(path)?;
        let tables: HashMap<String, Table> = serde_yaml::from_str(&contents)
            .map_err(|e| SchemaError::Parse(format!("Failed to parse YAML file '{}': {}", path.display(), e)))?;
        Ok(tables)
    }
    
    /// Parse all YAML files in a directory (sync version)
    #[cfg(not(feature = "tokio"))]
    pub fn parse_directory_sync(dir: &Path) -> Result<Schema> {
        if !dir.exists() {
            return Err(SchemaError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Schema directory not found: {:?}", dir),
            )));
        }
        
        let mut schema = Schema::new();
        
        // Check for meta.yaml file
        let meta_path = dir.join("meta.yaml");
        if meta_path.exists() {
            let meta_content = std::fs::read_to_string(&meta_path)?;
            schema.meta = Some(serde_yaml::from_str(&meta_content)
                .map_err(|e| SchemaError::Parse(format!("Failed to parse meta file '{}': {}", meta_path.display(), e)))?);
        }
        
        // Find all YAML files except meta.yaml
        let yaml_files: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml") &&
                path.file_name().map_or(false, |name| name != "meta.yaml")
            })
            .collect();
        
        for yaml_file in yaml_files {
            let tables = Self::parse_file_sync(&yaml_file)?;
            
            // Assign field names from YAML keys
            for (table_name, mut table) in tables {
                for (field_name, field) in &mut table.fields {
                    if field.name.is_empty() {
                        field.name = field_name.clone();
                    }
                }
                schema.tables.insert(table_name, table);
            }
        }
        
        Ok(schema)
    }
    
    /// Parse all YAML files in a directory
    #[cfg(feature = "tokio")]
    pub async fn parse_directory(dir: &Path) -> Result<Schema> {
        if !dir.exists() {
            return Err(SchemaError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Schema directory not found: {:?}", dir),
            )));
        }
        
        let mut schema = Schema::new();
        let mut meta_path = None;
        let mut yaml_files = Vec::new();
        
        // Read directory entries
        let mut entries = fs::read_dir(dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.is_file() {
                if let Some(file_name) = path.file_name() {
                    let file_name_str = file_name.to_string_lossy();
                    
                    // Check for meta file
                    if file_name_str == "_meta.yaml" || file_name_str == "_meta.yml" {
                        meta_path = Some(path);
                    }
                    // Check for schema files
                    else if (file_name_str.ends_with(".yaml") || file_name_str.ends_with(".yml"))
                        && !file_name_str.starts_with('_') {
                        yaml_files.push(path);
                    }
                }
            }
        }
        
        // Parse meta file if exists
        if let Some(meta_path) = meta_path {
            let meta_content = fs::read_to_string(&meta_path).await?;
            schema.meta = Some(serde_yaml::from_str::<SchemaMeta>(&meta_content)
                .map_err(|e| SchemaError::Parse(format!("Failed to parse meta file '{}': {}", meta_path.display(), e)))?);
        }
        
        // Parse all schema files
        for yaml_file in yaml_files {
            let tables = Self::parse_file(&yaml_file).await?;
            
            // Merge tables into schema
            for (name, mut table) in tables {
                // Ensure table name is set
                if table.name.is_empty() {
                    table.name = name.clone();
                }
                
                // Use table name as database table name if not specified
                if table.table.is_empty() {
                    table.table = table.name.to_lowercase();
                }
                
                // Ensure field names are set from YAML keys
                for (field_name, field) in &mut table.fields {
                    if field.name.is_empty() {
                        field.name = field_name.clone();
                    }
                }
                
                schema.tables.insert(name, table);
            }
        }
        
        Ok(schema)
    }
    
    /// Parse schema from YAML string (for testing)
    pub fn parse_yaml(yaml: &str) -> Result<HashMap<String, Table>> {
        let tables: HashMap<String, Table> = serde_yaml::from_str(yaml)?;
        Ok(tables)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    
    #[test]
    fn test_parse_simple_table() {
        let yaml = r#"
User:
  table: users
  version: 1
  description: "Test users table"
  fields:
    id:
      type: int
      primary_key: true
      auto: true
    email:
      type: string(255)
      unique: true
      required: true
    name:
      type: string(100)
      required: true
"#;
        
        let result = SchemaParser::parse_yaml(yaml);
        assert!(result.is_ok());
        
        let tables = result.unwrap();
        assert_eq!(tables.len(), 1);
        assert!(tables.contains_key("User"));
        
        let user_table = &tables["User"];
        assert_eq!(user_table.table, "users");
        assert_eq!(user_table.version, 1);
        assert_eq!(user_table.fields.len(), 3);
        
        // Check fields
        assert!(user_table.fields.contains_key("id"));
        assert!(user_table.fields.contains_key("email"));
        assert!(user_table.fields.contains_key("name"));
        
        let id_field = &user_table.fields["id"];
        assert_eq!(id_field.constraints.primary_key, Some(true));
    }
    
    #[test]
    fn test_parse_relations() {
        let yaml = r#"
Order:
  table: orders
  version: 1
  fields:
    id:
      type: int
      primary_key: true
    user_id:
      type: int
      required: true
  relations:
    belongs_to:
      user:
        model: User
        local_field: user_id
        foreign_field: id
    has_many:
      items:
        model: OrderItem
        local_field: id
        foreign_field: order_id
        cascade: cascade
"#;
        
        let result = SchemaParser::parse_yaml(yaml);
        assert!(result.is_ok());
        
        let tables = result.unwrap();
        let order_table = &tables["Order"];
        
        // Check belongs_to relation
        assert!(order_table.relations.belongs_to.is_some());
        let belongs_to = order_table.relations.belongs_to.as_ref().unwrap();
        assert!(belongs_to.contains_key("user"));
        
        let user_relation = &belongs_to["user"];
        assert_eq!(user_relation.model, "User");
        assert_eq!(user_relation.local_field, "user_id");
        assert_eq!(user_relation.foreign_field, "id");
        
        // Check has_many relation
        assert!(order_table.relations.has_many.is_some());
        let has_many = order_table.relations.has_many.as_ref().unwrap();
        assert!(has_many.contains_key("items"));
        
        let items_relation = &has_many["items"];
        assert_eq!(items_relation.model, "OrderItem");
        assert!(matches!(items_relation.cascade, Some(ForeignKeyAction::Cascade)));
    }
}