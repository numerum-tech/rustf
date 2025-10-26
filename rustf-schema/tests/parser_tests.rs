//! Tests for the schema parser module

use rustf_schema::{Schema, SchemaParser, Table, Field, FieldType, FieldConstraints, Relations, SchemaMeta};
use std::collections::HashMap;
use std::path::Path;
use tempfile::TempDir;
use tokio::fs;

/// Helper function to create a test schema directory
async fn create_test_schema_dir() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let schema_dir = temp_dir.path();
    
    // Create meta.yaml
    let meta_content = r#"
version: "1.0"
database: "test_db"
description: "Test database schema"
ai_context: "Testing schema parser functionality"
"#;
    fs::write(schema_dir.join("meta.yaml"), meta_content).await.unwrap();
    
    // Create users.yaml
    let users_content = r#"
User:
  table: users
  version: 1
  description: "User accounts"
  ai_context: "Main user table"
  fields:
    id:
      type: serial
      primary_key: true
      auto: true
      ai: "Unique user identifier"
    email:
      type: string(255)
      unique: true
      required: true
      ai: "User email address"
    name:
      type: string(100)
      required: true
      ai: "User display name"
    created_at:
      type: timestamp
      auto: create
      ai: "Account creation timestamp"
"#;
    fs::write(schema_dir.join("users.yaml"), users_content).await.unwrap();
    
    // Create posts.yaml
    let posts_content = r#"
Post:
  table: posts
  version: 1
  description: "User posts"
  fields:
    id:
      type: serial
      primary_key: true
      auto: true
    user_id:
      type: integer
      required: true
      foreign_key: "User.id"
    title:
      type: string(200)
      required: true
    content:
      type: text
      nullable: true
    created_at:
      type: timestamp
      auto: create
  relations:
    belongs_to:
      user:
        model: User
        local_field: user_id
        foreign_field: id
"#;
    fs::write(schema_dir.join("posts.yaml"), posts_content).await.unwrap();
    
    temp_dir
}

#[tokio::test]
async fn test_parse_single_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.yaml");
    
    let content = r#"
TestTable:
  table: test_table
  version: 1
  fields:
    id:
      type: integer
      primary_key: true
    name:
      type: string
"#;
    
    fs::write(&file_path, content).await.unwrap();
    
    let tables = SchemaParser::parse_file(&file_path).await.unwrap();
    
    assert_eq!(tables.len(), 1);
    assert!(tables.contains_key("TestTable"));
    
    let table = &tables["TestTable"];
    assert_eq!(table.table, "test_table");
    assert_eq!(table.version, 1);
    assert_eq!(table.fields.len(), 2);
    assert!(table.fields.contains_key("id"));
    assert!(table.fields.contains_key("name"));
}

#[tokio::test]
async fn test_parse_directory() {
    let temp_dir = create_test_schema_dir().await;
    let schema = SchemaParser::parse_directory(temp_dir.path()).await.unwrap();
    
    // Check meta
    assert!(schema.meta.is_some());
    let meta = schema.meta.unwrap();
    assert_eq!(meta.version, "1.0");
    assert_eq!(meta.database_name, "test_db");
    assert_eq!(meta.description, Some("Test database schema".to_string()));
    
    // Check tables
    assert_eq!(schema.tables.len(), 2);
    assert!(schema.tables.contains_key("User"));
    assert!(schema.tables.contains_key("Post"));
    
    // Check user table
    let user_table = &schema.tables["User"];
    assert_eq!(user_table.table, "users");
    assert_eq!(user_table.fields.len(), 4);
    assert!(user_table.fields.contains_key("id"));
    assert!(user_table.fields.contains_key("email"));
    
    // Check field names are assigned
    let id_field = &user_table.fields["id"];
    assert_eq!(id_field.name, "id");
    assert!(id_field.constraints.primary_key.unwrap_or(false));
    
    // Check post table relations
    let post_table = &schema.tables["Post"];
    assert!(post_table.relations.belongs_to.is_some());
    let belongs_to = post_table.relations.belongs_to.as_ref().unwrap();
    assert!(belongs_to.contains_key("user"));
    let user_relation = &belongs_to["user"];
    assert_eq!(user_relation.model, "User");
    assert_eq!(user_relation.local_field, "user_id");
    assert_eq!(user_relation.foreign_field, "id");
}

#[tokio::test]
async fn test_parse_directory_not_found() {
    let non_existent_path = Path::new("/non/existent/path");
    let result = SchemaParser::parse_directory(non_existent_path).await;
    
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("not found"));
}

#[tokio::test]
async fn test_parse_invalid_yaml() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("invalid.yaml");
    
    let invalid_content = r#"
InvalidYAML:
  - this is not
  - a valid table definition
    missing_colon "error"
"#;
    
    fs::write(&file_path, invalid_content).await.unwrap();
    
    let result = SchemaParser::parse_file(&file_path).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_field_name_assignment() {
    let temp_dir = TempDir::new().unwrap();
    let schema_dir = temp_dir.path();
    
    let content = r#"
TestTable:
  table: test_table
  version: 1
  fields:
    user_id:
      type: integer
      primary_key: true
    display_name:
      type: string(100)
      required: true
"#;
    
    fs::write(schema_dir.join("test.yaml"), content).await.unwrap();
    
    let schema = SchemaParser::parse_directory(schema_dir).await.unwrap();
    let table = &schema.tables["TestTable"];
    
    // Check that field names are properly assigned from YAML keys
    assert_eq!(table.fields["user_id"].name, "user_id");
    assert_eq!(table.fields["display_name"].name, "display_name");
}

#[tokio::test]
async fn test_multiple_yaml_files() {
    let temp_dir = TempDir::new().unwrap();
    let schema_dir = temp_dir.path();
    
    // Create multiple files
    let table1_content = r#"
Table1:
  table: table1
  version: 1
  fields:
    id:
      type: integer
      primary_key: true
"#;
    
    let table2_content = r#"
Table2:
  table: table2
  version: 1
  fields:
    id:
      type: integer
      primary_key: true
    table1_id:
      type: integer
      foreign_key: "Table1.id"
"#;
    
    fs::write(schema_dir.join("table1.yaml"), table1_content).await.unwrap();
    fs::write(schema_dir.join("table2.yaml"), table2_content).await.unwrap();
    
    let schema = SchemaParser::parse_directory(schema_dir).await.unwrap();
    
    assert_eq!(schema.tables.len(), 2);
    assert!(schema.tables.contains_key("Table1"));
    assert!(schema.tables.contains_key("Table2"));
}

#[tokio::test]
async fn test_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let schema = SchemaParser::parse_directory(temp_dir.path()).await.unwrap();
    
    assert!(schema.tables.is_empty());
    assert!(schema.meta.is_none());
}

#[tokio::test]
async fn test_meta_yaml_only() {
    let temp_dir = TempDir::new().unwrap();
    let schema_dir = temp_dir.path();
    
    let meta_content = r#"
version: "2.0"
database: "meta_only_db"
description: "Database with only meta"
"#;
    fs::write(schema_dir.join("meta.yaml"), meta_content).await.unwrap();
    
    let schema = SchemaParser::parse_directory(schema_dir).await.unwrap();
    
    assert!(schema.tables.is_empty());
    assert!(schema.meta.is_some());
    
    let meta = schema.meta.unwrap();
    assert_eq!(meta.version, "2.0");
    assert_eq!(meta.database_name, "meta_only_db");
}

#[tokio::test]
async fn test_complex_field_types() {
    let temp_dir = TempDir::new().unwrap();
    let schema_dir = temp_dir.path();
    
    let content = r#"
ComplexTable:
  table: complex_table
  version: 1
  fields:
    id:
      type: serial
      primary_key: true
    varchar_field:
      type: string(255)
      required: true
    decimal_field:
      type: decimal(10,2)
      nullable: true
    enum_field:
      type:
        enum:
          values: ["active", "inactive", "pending"]
          transitions:
            active: ["inactive"]
            inactive: ["active", "pending"]
            pending: ["active", "inactive"]
    json_field:
      type:
        json:
          schema: |
            {
              "type": "object",
              "properties": {
                "name": {"type": "string"},
                "age": {"type": "number"}
              }
            }
"#;
    
    fs::write(schema_dir.join("complex.yaml"), content).await.unwrap();
    
    let schema = SchemaParser::parse_directory(schema_dir).await.unwrap();
    let table = &schema.tables["ComplexTable"];
    
    // Check different field types
    let varchar_field = &table.fields["varchar_field"];
    match &varchar_field.field_type {
        FieldType::Parameterized { base_type, params } => {
            assert_eq!(base_type, "string");
            assert_eq!(params.len(), 1);
        }
        _ => panic!("Expected parameterized type"),
    }
    
    let enum_field = &table.fields["enum_field"];
    match &enum_field.field_type {
        FieldType::Enum { values, transitions, .. } => {
            assert_eq!(values.len(), 3);
            assert!(values.contains(&"active".to_string()));
            assert!(transitions.is_some());
        }
        _ => panic!("Expected enum type"),
    }
    
    let json_field = &table.fields["json_field"];
    match &json_field.field_type {
        FieldType::Json { schema, .. } => {
            assert!(schema.is_some());
        }
        _ => panic!("Expected JSON type"),
    }
}