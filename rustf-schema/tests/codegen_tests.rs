//! Tests for the code generation module

#[cfg(feature = "codegen")]
mod tests {
    use rustf_schema::{
        Schema, Table, Field, FieldType, FieldConstraints, Relations, AutoGenerate,
        codegen::{SqlxGenerator, CodeGenerator, TemplateGenerator, GenerationContext},
    };
    use std::collections::HashMap;

    /// Helper function to create a test table
    fn create_test_table() -> (String, Table) {
        let mut fields = HashMap::new();
        
        // Primary key
        fields.insert("id".to_string(), Field {
            name: "id".to_string(),
            field_type: FieldType::Simple("serial".to_string()),
            lang_type: None,
            postgres_type_name: None,
            constraints: FieldConstraints {
                primary_key: Some(true),
                auto: Some(AutoGenerate::Boolean(true)),
                ..Default::default()
            },
            ai: Some("Primary key".to_string()),
            example: None,
        });
        
        // String field
        fields.insert("name".to_string(), Field {
            name: "name".to_string(),
            field_type: FieldType::Parameterized {
                base_type: "string".to_string(),
                params: vec![rustf_schema::types::TypeParam::Number(100)],
            },
            lang_type: None,
            postgres_type_name: None,
            constraints: FieldConstraints {
                required: Some(true),
                ..Default::default()
            },
            ai: Some("User name".to_string()),
            example: Some(serde_json::Value::String("John Doe".to_string())),
        });
        
        // Optional email field
        fields.insert("email".to_string(), Field {
            name: "email".to_string(),
            field_type: FieldType::Parameterized {
                base_type: "string".to_string(),
                params: vec![rustf_schema::types::TypeParam::Number(255)],
            },
            lang_type: None,
            postgres_type_name: None,
            constraints: FieldConstraints {
                nullable: Some(true),
                unique: Some(true),
                ..Default::default()
            },
            ai: Some("User email address".to_string()),
            example: Some(serde_json::Value::String("john@example.com".to_string())),
        });
        
        // Timestamp field
        fields.insert("created_at".to_string(), Field {
            name: "created_at".to_string(),
            field_type: FieldType::Simple("timestamp".to_string()),
            lang_type: None,
            postgres_type_name: None,
            constraints: FieldConstraints {
                auto: Some(AutoGenerate::Type("create".to_string())),
                ..Default::default()
            },
            ai: Some("Creation timestamp".to_string()),
            example: None,
        });
        
        let table = Table {
            name: "User".to_string(),
            table: "users".to_string(),
            version: 1,
        database_type: Some("mysql".to_string()),
        database_name: Some("test_db".to_string()),
        element_type: Some("table".to_string()),
            description: Some("User accounts".to_string()),
            tags: vec!["core".to_string()],
            ai_context: Some("Main user table for authentication".to_string()),
            fields,
            relations: Relations::default(),
            indexes: vec![],
            constraints: vec![],
        };
        
        ("User".to_string(), table)
    }

    #[test]
    fn test_sqlx_generator_creation() {
        let generator = SqlxGenerator::new();
        assert!(generator.is_ok());
    }

    #[test]
    fn test_field_type_to_rust() {
        let generator = SqlxGenerator::new().unwrap();
        
        // Test basic types
        assert_eq!(generator.field_type_to_rust(&FieldType::Simple("integer".to_string()), false), "i32");
        assert_eq!(generator.field_type_to_rust(&FieldType::Simple("string".to_string()), false), "String");
        assert_eq!(generator.field_type_to_rust(&FieldType::Simple("boolean".to_string()), false), "bool");
        assert_eq!(generator.field_type_to_rust(&FieldType::Simple("timestamp".to_string()), false), "chrono::DateTime<chrono::Utc>");
        
        // Test nullable types
        assert_eq!(generator.field_type_to_rust(&FieldType::Simple("string".to_string()), true), "Option<String>");
        
        // Test parameterized types
        let varchar_type = FieldType::Parameterized {
            base_type: "string".to_string(),
            params: vec![rustf_schema::types::TypeParam::Number(255)],
        };
        assert_eq!(generator.field_type_to_rust(&varchar_type, false), "String");
        
        // Test enum type
        let enum_type = FieldType::Enum {
            type_name: "status".to_string(),
            values: vec!["active".to_string(), "inactive".to_string()],
            transitions: None,
        };
        assert_eq!(generator.field_type_to_rust(&enum_type, false), "String");
    }

    #[test]
    fn test_field_type_to_sqlx() {
        let generator = SqlxGenerator::new().unwrap();
        
        assert_eq!(generator.field_type_to_sqlx(&FieldType::Simple("timestamp".to_string())), "TIMESTAMPTZ");
        assert_eq!(generator.field_type_to_sqlx(&FieldType::Simple("json".to_string())), "JSON");
        assert_eq!(generator.field_type_to_sqlx(&FieldType::Simple("uuid".to_string())), "UUID");
        assert_eq!(generator.field_type_to_sqlx(&FieldType::Simple("integer".to_string())), "INTEGER");
    }

    #[test]
    fn test_generate_model() {
        let generator = SqlxGenerator::new().unwrap();
        let (table_name, table) = create_test_table();
        let schema = Schema {
            tables: {
                let mut tables = HashMap::new();
                tables.insert(table_name.clone(), table.clone());
                tables
            },
            meta: None,
        };
        
        let result = generator.generate_model(&table_name, &table, &schema);
        assert!(result.is_ok());
        
        let code = result.unwrap();
        
        // Check that generated code contains expected elements
        assert!(code.contains("pub struct User"));
        assert!(code.contains("pub id: i32"));
        assert!(code.contains("pub name: String"));
        assert!(code.contains("pub email: Option<String>"));
        assert!(code.contains("pub created_at: chrono::DateTime<chrono::Utc>"));
        assert!(code.contains("impl User"));
        assert!(code.contains("pub fn new()"));
        assert!(code.contains("pub async fn insert"));
        assert!(code.contains("pub async fn update"));
        assert!(code.contains("pub async fn delete"));
        assert!(code.contains("pub async fn find("));
        assert!(code.contains("pub async fn find_all"));
        assert!(code.contains("pub async fn count"));
        
        // Check for AI hints in comments
        assert!(code.contains("/// Primary key"));
        assert!(code.contains("/// User name"));
        assert!(code.contains("/// User email address"));
        
        // Check for proper derives
        assert!(code.contains("#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]"));
        
        // Check for schema metadata
        assert!(code.contains("#[rustf_schema(table = \"users\", version = 1)]"));
    }

    #[test]
    fn test_generate_crud() {
        let generator = SqlxGenerator::new().unwrap();
        let (table_name, table) = create_test_table();
        let schema = Schema {
            tables: {
                let mut tables = HashMap::new();
                tables.insert(table_name.clone(), table.clone());
                tables
            },
            meta: None,
        };
        
        let result = generator.generate_crud(&table_name, &table, &schema);
        assert!(result.is_ok());
        
        let code = result.unwrap();
        
        // Check that CRUD code contains expected elements
        assert!(code.contains("pub struct UserRepository"));
        assert!(code.contains("pub fn new(pool: PgPool)"));
        assert!(code.contains("pub async fn create"));
        assert!(code.contains("pub async fn get_by_id"));
        assert!(code.contains("pub async fn get_all"));
        assert!(code.contains("pub async fn update"));
        assert!(code.contains("pub async fn delete"));
        assert!(code.contains("pub async fn delete_by_id"));
        assert!(code.contains("pub async fn count"));
    }

    #[test]
    fn test_template_generator() {
        let mut generator = TemplateGenerator::new();
        
        // Register a simple template
        let template = "Hello {{name}}!";
        let result = generator.register_template("greeting", template);
        assert!(result.is_ok());
        
        // Create generation context
        let context = GenerationContext {
            schema: Schema::new(),
            table: create_test_table().1,
            table_name: "User".to_string(),
            variables: {
                let mut vars = HashMap::new();
                vars.insert("name".to_string(), serde_json::Value::String("World".to_string()));
                vars
            },
        };
        
        // Render template
        let result = generator.render("greeting", &context);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello World!");
    }

    #[test]
    fn test_template_helpers() {
        use rustf_schema::codegen::{to_snake_case, to_camel_case, to_pascal_case, pluralize};
        
        // Test string transformation helpers
        assert_eq!(to_snake_case("UserAccount"), "user_account");
        assert_eq!(to_snake_case("XMLHttpRequest"), "x_m_l_http_request");
        
        assert_eq!(to_camel_case("user_account"), "userAccount");
        assert_eq!(to_camel_case("xml-http-request"), "xmlHttpRequest");
        
        assert_eq!(to_pascal_case("user_account"), "UserAccount");
        assert_eq!(to_pascal_case("xml-http-request"), "XmlHttpRequest");
        
        // Test pluralization
        assert_eq!(pluralize("user"), "users");
        assert_eq!(pluralize("category"), "categories");
        assert_eq!(pluralize("box"), "boxes");
        assert_eq!(pluralize("knife"), "knives");
    }

    #[test]
    fn test_generate_with_relations() {
        let generator = SqlxGenerator::new().unwrap();
        
        // Create tables with relations
        let mut schema = Schema::new();
        let (user_name, user_table) = create_test_table();
        schema.tables.insert(user_name.clone(), user_table);
        
        // Create post table with relation to user
        let mut post_fields = HashMap::new();
        post_fields.insert("id".to_string(), Field {
            name: "id".to_string(),
            field_type: FieldType::Simple("serial".to_string()),
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
        
        post_fields.insert("user_id".to_string(), Field {
            name: "user_id".to_string(),
            field_type: FieldType::Simple("integer".to_string()),
            lang_type: None,
            postgres_type_name: None,
            constraints: FieldConstraints {
                required: Some(true),
                foreign_key: Some("User.id".to_string()),
                ..Default::default()
            },
            ai: None,
            example: None,
        });
        
        let mut relations = Relations::default();
        let mut belongs_to = HashMap::new();
        belongs_to.insert("user".to_string(), rustf_schema::BelongsTo {
            model: "User".to_string(),
            local_field: "user_id".to_string(),
            foreign_field: "id".to_string(),
            on_delete: None,
            on_update: None,
            ai: None,
        });
        relations.belongs_to = Some(belongs_to);
        
        let post_table = Table {
            name: "Post".to_string(),
            table: "posts".to_string(),
            version: 1,
        database_type: Some("mysql".to_string()),
        database_name: Some("test_db".to_string()),
        element_type: Some("table".to_string()),
            description: Some("User posts".to_string()),
            tags: vec![],
            ai_context: None,
            fields: post_fields,
            relations,
            indexes: vec![],
            constraints: vec![],
        };
        
        schema.tables.insert("Post".to_string(), post_table.clone());
        
        // Generate model with relations
        let result = generator.generate_model("Post", &post_table, &schema);
        assert!(result.is_ok());
        
        let code = result.unwrap();
        assert!(code.contains("pub struct Post"));
        assert!(code.contains("pub user_id: i32"));
    }

    #[test]
    fn test_generate_relations() {
        let generator = SqlxGenerator::new().unwrap();
        
        // Create schema with relations
        let mut schema = Schema::new();
        let (user_name, mut user_table) = create_test_table();
        
        // Add has_many relation to posts
        let mut has_many = HashMap::new();
        has_many.insert("posts".to_string(), rustf_schema::HasMany {
            model: "Post".to_string(),
            local_field: "id".to_string(),
            foreign_field: "user_id".to_string(),
            cascade: None,
            ai: None,
        });
        user_table.relations.has_many = Some(has_many);
        
        schema.tables.insert(user_name.clone(), user_table.clone());
        
        let result = generator.generate_relations(&user_name, &user_table, &schema);
        assert!(result.is_ok());
        
        let code = result.unwrap();
        assert!(code.contains("impl User"));
        assert!(code.contains("pub async fn get_posts"));
        assert!(code.contains("pub async fn count_posts"));
    }

    #[test]
    fn test_code_generator_trait() {
        let generator = SqlxGenerator::new().unwrap();
        let (table_name, table) = create_test_table();
        let schema = Schema {
            tables: {
                let mut tables = HashMap::new();
                tables.insert(table_name.clone(), table.clone());
                tables
            },
            meta: None,
        };
        
        // Test the CodeGenerator trait implementation
        let result = generator.generate_table(&table_name, &table, &schema);
        assert!(result.is_ok());
        
        let code = result.unwrap();
        assert!(code.contains("pub struct User"));
        
        // Test generating entire schema
        let results = generator.generate_schema(&schema);
        assert!(results.is_ok());
        
        let all_code = results.unwrap();
        assert_eq!(all_code.len(), 1);
        assert!(all_code.contains_key("User"));
    }

    #[test]
    fn test_complex_field_types() {
        let generator = SqlxGenerator::new().unwrap();
        
        // Test decimal type
        let decimal_type = FieldType::Parameterized {
            base_type: "decimal".to_string(),
            params: vec![
                rustf_schema::types::TypeParam::Number(10),
                rustf_schema::types::TypeParam::Number(2)
            ],
        };
        assert_eq!(generator.field_type_to_rust(&decimal_type, false), "rust_decimal::Decimal");
        
        // Test JSON type
        let json_type = FieldType::Json {
            type_name: "json".to_string(),
            schema: Some(serde_json::Value::String(r#"{"type": "object"}"#.to_string())),
        };
        assert_eq!(generator.field_type_to_rust(&json_type, false), "serde_json::Value");
        
        // Test UUID type
        assert_eq!(generator.field_type_to_rust(&FieldType::Simple("uuid".to_string()), false), "uuid::Uuid");
    }

    #[test]
    fn test_generation_context() {
        let (table_name, table) = create_test_table();
        let schema = Schema::new();
        
        let context = GenerationContext {
            schema: schema.clone(),
            table: table.clone(),
            table_name: table_name.clone(),
            variables: HashMap::new(),
        };
        
        assert_eq!(context.table_name, "User");
        assert_eq!(context.table.table, "users");
        assert_eq!(context.table.fields.len(), 4);
    }
}

// Tests that don't require the codegen feature
#[test]
fn test_codegen_feature_disabled() {
    #[cfg(not(feature = "codegen"))]
    {
        // When codegen feature is disabled, the codegen module should not be available
        // This test just ensures the crate compiles without the codegen feature
        use rustf_schema::Schema;
        let schema = Schema::new();
        assert!(schema.tables.is_empty());
    }
}