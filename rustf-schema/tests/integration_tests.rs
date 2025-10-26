//! Integration tests for the complete schema workflow

use rustf_schema::{Schema, SchemaError};
use std::collections::HashMap;
use tempfile::TempDir;
use tokio::fs;

/// Test the complete workflow: parse -> validate -> generate -> consistency check
#[tokio::test]
async fn test_complete_workflow() {
    // Create test schema directory
    let temp_dir = create_comprehensive_schema().await;
    
    // 1. Parse schema from directory
    let schema = Schema::load_from_directory(temp_dir.path()).await.unwrap();
    
    // Verify parsing
    assert_eq!(schema.tables.len(), 4);
    assert!(schema.tables.contains_key("User"));
    assert!(schema.tables.contains_key("Post"));
    assert!(schema.tables.contains_key("Category"));
    assert!(schema.tables.contains_key("Tag"));
    
    // Check meta information
    assert!(schema.meta.is_some());
    let meta = schema.meta.as_ref().unwrap();
    assert_eq!(meta.database_name, "blog_system");
    assert_eq!(meta.version, "1.0");
    
    // 2. Validate schema
    let validation_result = schema.validate();
    assert!(validation_result.is_ok(), "Schema validation failed: {:?}", validation_result);
    
    // 3. Test schema checksum generation
    let checksum1 = schema.checksum();
    let checksum2 = schema.checksum();
    assert_eq!(checksum1, checksum2, "Checksum should be deterministic");
    
    // 4. Test consistency validation
    let mut generated_checksums = HashMap::new();
    for table_name in schema.table_names() {
        generated_checksums.insert(table_name.to_string(), checksum1.clone());
    }
    
    let consistency_result = schema.validate_consistency(&generated_checksums);
    assert!(consistency_result.is_ok(), "Consistency validation failed: {:?}", consistency_result);
    
    // 5. Test code generation (if codegen feature is enabled)
    #[cfg(feature = "codegen")]
    {
        use rustf_schema::codegen::{SqlxGenerator, CodeGenerator};
        
        let generator = SqlxGenerator::new().unwrap();
        
        // Generate code for each table
        for (table_name, table) in &schema.tables {
            let result = generator.generate_table(table_name, table, &schema);
            assert!(result.is_ok(), "Code generation failed for table {}: {:?}", table_name, result);
            
            let code = result.unwrap();
            
            // Basic checks that the generated code contains expected elements
            assert!(code.contains(&format!("pub struct {}", table_name)));
            assert!(code.contains("impl"));
            assert!(code.contains("pub fn new()"));
            
            if table.fields.values().any(|f| f.constraints.primary_key.unwrap_or(false)) {
                assert!(code.contains("pub async fn insert"));
                assert!(code.contains("pub async fn update"));
                assert!(code.contains("pub async fn delete"));
            }
        }
        
        // Generate entire schema
        let all_code = generator.generate_schema(&schema).unwrap();
        assert_eq!(all_code.len(), schema.tables.len());
    }
}

#[tokio::test]
async fn test_consistency_validation_scenarios() {
    let temp_dir = create_basic_schema().await;
    let schema = Schema::load_from_directory(temp_dir.path()).await.unwrap();
    let schema_checksum = schema.checksum();
    
    // Test 1: Valid consistency - all tables have matching checksums
    let mut valid_checksums = HashMap::new();
    valid_checksums.insert("User".to_string(), schema_checksum.clone());
    valid_checksums.insert("Post".to_string(), schema_checksum.clone());
    
    let result = schema.validate_consistency(&valid_checksums);
    assert!(result.is_ok());
    
    // Test 2: Checksum mismatch - schema changed since generation
    let mut mismatched_checksums = HashMap::new();
    mismatched_checksums.insert("User".to_string(), "old_checksum".to_string());
    mismatched_checksums.insert("Post".to_string(), schema_checksum.clone());
    
    let result = schema.validate_consistency(&mismatched_checksums);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SchemaError::Consistency(_)));
    
    // Test 3: Missing generated code
    let mut incomplete_checksums = HashMap::new();
    incomplete_checksums.insert("User".to_string(), schema_checksum.clone());
    // Missing Post table
    
    let result = schema.validate_consistency(&incomplete_checksums);
    assert!(result.is_err());
    
    // Test 4: Extra generated code for non-existent table
    let mut extra_checksums = HashMap::new();
    extra_checksums.insert("User".to_string(), schema_checksum.clone());
    extra_checksums.insert("Post".to_string(), schema_checksum.clone());
    extra_checksums.insert("NonExistent".to_string(), schema_checksum.clone());
    
    let result = schema.validate_consistency(&extra_checksums);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_schema_modification_detection() {
    let temp_dir = create_basic_schema().await;
    let schema1 = Schema::load_from_directory(temp_dir.path()).await.unwrap();
    let checksum1 = schema1.checksum();
    
    // Modify schema by adding a field
    let user_content = r#"
User:
  table: users
  version: 1
  description: "User accounts"
  fields:
    id:
      type: serial
      primary_key: true
      auto: true
    email:
      type: string(255)
      unique: true
      required: true
    name:
      type: string(100)
      required: true
    age:
      type: integer
      nullable: true
      ai: "User age (new field)"
"#;
    
    fs::write(temp_dir.path().join("users.yaml"), user_content).await.unwrap();
    
    let schema2 = Schema::load_from_directory(temp_dir.path()).await.unwrap();
    let checksum2 = schema2.checksum();
    
    // Checksums should be different
    assert_ne!(checksum1, checksum2, "Schema checksum should change when schema is modified");
}

#[tokio::test]
async fn test_error_handling() {
    // Test parsing non-existent directory
    let result = Schema::load_from_directory(std::path::Path::new("/non/existent/path")).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SchemaError::Io(_)));
    
    // Test parsing invalid YAML
    let temp_dir = TempDir::new().unwrap();
    let invalid_yaml = r#"
InvalidTable:
  - this is not
  - a valid schema
  missing: colon here "error"
"#;
    fs::write(temp_dir.path().join("invalid.yaml"), invalid_yaml).await.unwrap();
    
    let result = Schema::load_from_directory(temp_dir.path()).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SchemaError::Yaml(_)));
}

#[tokio::test]
async fn test_schema_field_resolution() {
    let temp_dir = create_basic_schema().await;
    let schema = Schema::load_from_directory(temp_dir.path()).await.unwrap();
    
    // Test valid field resolution
    let result = schema.resolve_field_ref("User.id");
    assert!(result.is_ok());
    let (table, field) = result.unwrap();
    assert_eq!(table.name, "User");
    assert_eq!(field.name, "id");
    
    // Test invalid field reference format
    let result = schema.resolve_field_ref("InvalidFormat");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SchemaError::Validation(_)));
    
    // Test non-existent table
    let result = schema.resolve_field_ref("NonExistent.field");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SchemaError::TableNotFound(_)));
    
    // Test non-existent field
    let result = schema.resolve_field_ref("User.nonexistent");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SchemaError::FieldNotFound { .. }));
}

#[tokio::test]
async fn test_complex_relations() {
    let temp_dir = create_comprehensive_schema().await;
    let schema = Schema::load_from_directory(temp_dir.path()).await.unwrap();
    
    // Validate schema with complex relations
    let result = schema.validate();
    assert!(result.is_ok(), "Complex schema validation failed: {:?}", result);
    
    // Check specific relations
    let post_table = schema.get_table("Post").unwrap();
    
    // Check belongs_to relations
    assert!(post_table.relations.belongs_to.is_some());
    let belongs_to = post_table.relations.belongs_to.as_ref().unwrap();
    assert!(belongs_to.contains_key("user"));
    assert!(belongs_to.contains_key("category"));
    
    // Check many_to_many relations
    assert!(post_table.relations.many_to_many.is_some());
    let many_to_many = post_table.relations.many_to_many.as_ref().unwrap();
    assert!(many_to_many.contains_key("tags"));
}

/// Helper function to create a basic test schema
async fn create_basic_schema() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let schema_dir = temp_dir.path();
    
    // Create meta.yaml
    let meta_content = r#"
version: "1.0"
database: "test_db"
description: "Basic test schema"
"#;
    fs::write(schema_dir.join("meta.yaml"), meta_content).await.unwrap();
    
    // Create users.yaml
    let users_content = r#"
User:
  table: users
  version: 1
  description: "User accounts"
  fields:
    id:
      type: serial
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

/// Helper function to create a comprehensive test schema with complex relations
async fn create_comprehensive_schema() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let schema_dir = temp_dir.path();
    
    // Create meta.yaml
    let meta_content = r#"
version: "1.0"
database: "blog_system"
description: "Comprehensive blog system schema"
ai_context: "Full-featured blog with users, posts, categories, and tags"
"#;
    fs::write(schema_dir.join("meta.yaml"), meta_content).await.unwrap();
    
    // Create users.yaml
    let users_content = r#"
User:
  table: users
  version: 1
  description: "User accounts"
  ai_context: "Main user table for authentication and profile"
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
      ai: "User email for login"
    name:
      type: string(100)
      required: true
      ai: "Display name"
    created_at:
      type: timestamp
      auto: create
      ai: "Account creation time"
  relations:
    has_many:
      posts:
        model: Post
        local_field: id
        foreign_field: user_id
"#;
    fs::write(schema_dir.join("users.yaml"), users_content).await.unwrap();
    
    // Create categories.yaml
    let categories_content = r#"
Category:
  table: categories
  version: 1
  description: "Post categories"
  fields:
    id:
      type: serial
      primary_key: true
      auto: true
    name:
      type: string(100)
      unique: true
      required: true
    slug:
      type: string(100)
      unique: true
      required: true
    parent_id:
      type: integer
      nullable: true
      foreign_key: "Category.id"
  relations:
    belongs_to:
      parent:
        model: Category
        local_field: parent_id
        foreign_field: id
    has_many:
      children:
        model: Category
        local_field: id
        foreign_field: parent_id
      posts:
        model: Post
        local_field: id
        foreign_field: category_id
"#;
    fs::write(schema_dir.join("categories.yaml"), categories_content).await.unwrap();
    
    // Create tags.yaml
    let tags_content = r#"
Tag:
  table: tags
  version: 1
  description: "Post tags"
  fields:
    id:
      type: serial
      primary_key: true
      auto: true
    name:
      type: string(50)
      unique: true
      required: true
    color:
      type: string(7)
      nullable: true
      ai: "Hex color code for tag display"
  relations:
    many_to_many:
      posts:
        model: Post
        through_table: post_tags
        local_field: id
        foreign_field: id
        through_local_field: tag_id
        through_foreign_field: post_id
"#;
    fs::write(schema_dir.join("tags.yaml"), tags_content).await.unwrap();
    
    // Create posts.yaml
    let posts_content = r#"
Post:
  table: posts
  version: 2
  description: "Blog posts"
  fields:
    id:
      type: serial
      primary_key: true
      auto: true
    user_id:
      type: integer
      required: true
      foreign_key: "User.id"
    category_id:
      type: integer
      nullable: true
      foreign_key: "Category.id"
    title:
      type: string(200)
      required: true
    slug:
      type: string(200)
      unique: true
      required: true
    content:
      type: text
      nullable: true
    status:
      type:
        enum:
          values: ["draft", "published", "archived"]
          transitions:
            draft: ["published", "archived"]
            published: ["archived"]
            archived: ["draft"]
      default: "draft"
    meta:
      type:
        json:
          schema: |
            {
              "type": "object",
              "properties": {
                "seo_title": {"type": "string"},
                "seo_description": {"type": "string"},
                "featured_image": {"type": "string"}
              }
            }
      nullable: true
    created_at:
      type: timestamp
      auto: create
    updated_at:
      type: timestamp
      auto: update
  relations:
    belongs_to:
      user:
        model: User
        local_field: user_id
        foreign_field: id
      category:
        model: Category
        local_field: category_id
        foreign_field: id
    many_to_many:
      tags:
        model: Tag
        through_table: post_tags
        local_field: id
        foreign_field: id
        through_local_field: post_id
        through_foreign_field: tag_id
"#;
    fs::write(schema_dir.join("posts.yaml"), posts_content).await.unwrap();
    
    temp_dir
}