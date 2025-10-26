//! Tests for the schema validator module

use rustf_schema::{
    Schema, SchemaValidator, Table, Field, FieldType, FieldConstraints, Relations, 
    AutoGenerate, BelongsTo, SchemaError
};
use std::collections::HashMap;

/// Helper function to create a basic valid table
fn create_basic_table(name: &str, table: &str) -> Table {
    let mut fields = HashMap::new();
    
    // Add primary key field
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
    
    Table {
        name: name.to_string(),
        table: table.to_string(),
        version: 1,
        database_type: Some("mysql".to_string()),
        database_name: Some("test_db".to_string()),
        element_type: Some("table".to_string()),
        description: Some("Test table".to_string()),
        tags: vec![],
        ai_context: None,
        fields,
        relations: Relations::default(),
        indexes: vec![],
        constraints: vec![],
    }
}

#[test]
fn test_validate_basic_table() {
    let table = create_basic_table("User", "users");
    let result = SchemaValidator::validate_table("User", &table);
    assert!(result.is_ok());
}

#[test]
fn test_validate_table_without_fields() {
    let table = Table {
        name: "Empty".to_string(),
        table: "empty".to_string(),
        version: 1,
        database_type: Some("mysql".to_string()),
        database_name: Some("test_db".to_string()),
        element_type: Some("table".to_string()),
        description: None,
        tags: vec![],
        ai_context: None,
        fields: HashMap::new(),
        relations: Relations::default(),
        indexes: vec![],
        constraints: vec![],
    };
    
    let result = SchemaValidator::validate_table("Empty", &table);
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert!(matches!(error, SchemaError::Validation(_)));
    assert!(error.to_string().contains("no fields"));
}

#[test]
fn test_validate_table_without_primary_key() {
    let mut fields = HashMap::new();
    fields.insert("name".to_string(), Field {
        name: "name".to_string(),
        field_type: FieldType::Simple("string".to_string()),
        lang_type: None,
        postgres_type_name: None,
        constraints: FieldConstraints::default(),
        ai: None,
        example: None,
    });
    
    let table = Table {
        name: "NoPK".to_string(),
        table: "no_pk".to_string(),
        version: 1,
        database_type: Some("mysql".to_string()),
        database_name: Some("test_db".to_string()),
        element_type: Some("table".to_string()),
        description: None,
        tags: vec![],
        ai_context: None,
        fields,
        relations: Relations::default(),
        indexes: vec![],
        constraints: vec![],
    };
    
    let result = SchemaValidator::validate_table("NoPK", &table);
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert!(error.to_string().contains("no primary key"));
}

#[test]
fn test_validate_field_constraints() {
    let mut schema = Schema::new();
    let mut table = create_basic_table("Test", "test");
    
    // Add a field that is both required and nullable (invalid)
    table.fields.insert("invalid_field".to_string(), Field {
        name: "invalid_field".to_string(),
        field_type: FieldType::Simple("string".to_string()),
        lang_type: None,
        postgres_type_name: None,
        constraints: FieldConstraints {
            required: Some(true),
            nullable: Some(true),
            ..Default::default()
        },
        ai: None,
        example: None,
    });
    
    schema.tables.insert("Test".to_string(), table);
    
    let result = SchemaValidator::validate(&schema);
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert!(error.to_string().contains("both required and nullable"));
}

#[test]
fn test_validate_auto_generation() {
    let mut schema = Schema::new();
    let mut table = create_basic_table("Test", "test");
    
    // Add field with auto=true but not primary key or serial/uuid
    table.fields.insert("invalid_auto".to_string(), Field {
        name: "invalid_auto".to_string(),
        field_type: FieldType::Simple("string".to_string()),
        lang_type: None,
        postgres_type_name: None,
        constraints: FieldConstraints {
            auto: Some(AutoGenerate::Boolean(true)),
            ..Default::default()
        },
        ai: None,
        example: None,
    });
    
    schema.tables.insert("Test".to_string(), table);
    
    let result = SchemaValidator::validate(&schema);
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert!(error.to_string().contains("auto=true must be primary key"));
}

#[test]
fn test_validate_auto_timestamp() {
    let mut schema = Schema::new();
    let mut table = create_basic_table("Test", "test");
    
    // Add field with auto="create" but not timestamp type
    table.fields.insert("invalid_timestamp".to_string(), Field {
        name: "invalid_timestamp".to_string(),
        field_type: FieldType::Simple("string".to_string()),
        lang_type: None,
        postgres_type_name: None,
        constraints: FieldConstraints {
            auto: Some(AutoGenerate::Type("create".to_string())),
            ..Default::default()
        },
        ai: None,
        example: None,
    });
    
    schema.tables.insert("Test".to_string(), table);
    
    let result = SchemaValidator::validate(&schema);
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert!(error.to_string().contains("must be timestamp type"));
}

#[test]
fn test_validate_field_types() {
    // Test valid field types
    let valid_types = vec![
        "int", "integer", "serial", "bigint",
        "string", "text", "varchar",
        "decimal", "float", "double",
        "boolean", "bool",
        "timestamp", "datetime", "date", "time",
        "json", "jsonb", "uuid", "blob", "enum"
    ];
    
    for type_name in valid_types {
        let field_type = FieldType::Simple(type_name.to_string());
        let result = SchemaValidator::validate_field_type(&field_type);
        assert!(result.is_ok(), "Type '{}' should be valid", type_name);
    }
    
    // Test invalid field type
    let invalid_type = FieldType::Simple("invalid_type".to_string());
    let result = SchemaValidator::validate_field_type(&invalid_type);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown type"));
}

#[test]
fn test_validate_parameterized_types() {
    use rustf_schema::types::TypeParam;
    
    // Valid string with length
    let string_type = FieldType::Parameterized {
        base_type: "string".to_string(),
        params: vec![TypeParam::Number(255)],
    };
    let result = SchemaValidator::validate_field_type(&string_type);
    assert!(result.is_ok());
    
    // Invalid string with no parameters
    let invalid_string = FieldType::Parameterized {
        base_type: "string".to_string(),
        params: vec![],
    };
    let result = SchemaValidator::validate_field_type(&invalid_string);
    assert!(result.is_err());
    
    // Valid decimal with precision and scale
    let decimal_type = FieldType::Parameterized {
        base_type: "decimal".to_string(),
        params: vec![TypeParam::Number(10), TypeParam::Number(2)],
    };
    let result = SchemaValidator::validate_field_type(&decimal_type);
    assert!(result.is_ok());
    
    // Invalid decimal with wrong number of parameters
    let invalid_decimal = FieldType::Parameterized {
        base_type: "decimal".to_string(),
        params: vec![TypeParam::Number(10)],
    };
    let result = SchemaValidator::validate_field_type(&invalid_decimal);
    assert!(result.is_err());
}

#[test]
fn test_validate_enum_type() {
    // Valid enum
    let enum_type = FieldType::Enum {
        type_name: "status".to_string(),
        values: vec!["active".to_string(), "inactive".to_string()],
        transitions: None,
    };
    let result = SchemaValidator::validate_field_type(&enum_type);
    assert!(result.is_ok());
    
    // Empty enum (invalid)
    let empty_enum = FieldType::Enum {
        type_name: "empty".to_string(),
        values: vec![],
        transitions: None,
    };
    let result = SchemaValidator::validate_field_type(&empty_enum);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("at least one value"));
}

#[test]
fn test_validate_enum_transitions() {
    let mut transitions = HashMap::new();
    transitions.insert("active".to_string(), vec!["inactive".to_string()]);
    transitions.insert("invalid_state".to_string(), vec!["active".to_string()]); // Invalid state
    
    let enum_type = FieldType::Enum {
        type_name: "status".to_string(),
        values: vec!["active".to_string(), "inactive".to_string()],
        transitions: Some(transitions),
    };
    
    let result = SchemaValidator::validate_field_type(&enum_type);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not in enum values"));
}

#[test]
fn test_validate_foreign_keys() {
    let mut schema = Schema::new();
    
    // Create user table
    let user_table = create_basic_table("User", "users");
    schema.tables.insert("User".to_string(), user_table);
    
    // Create post table with foreign key to user
    let mut post_table = create_basic_table("Post", "posts");
    post_table.fields.insert("user_id".to_string(), Field {
        name: "user_id".to_string(),
        field_type: FieldType::Simple("integer".to_string()),
        lang_type: None,
        postgres_type_name: None,
        constraints: FieldConstraints {
            foreign_key: Some("User.id".to_string()),
            required: Some(true),
            ..Default::default()
        },
        ai: None,
        example: None,
    });
    schema.tables.insert("Post".to_string(), post_table);
    
    let result = SchemaValidator::validate(&schema);
    assert!(result.is_ok());
}

#[test]
fn test_validate_invalid_foreign_key() {
    let mut schema = Schema::new();
    
    // Create table with foreign key to non-existent table
    let mut table = create_basic_table("Post", "posts");
    table.fields.insert("user_id".to_string(), Field {
        name: "user_id".to_string(),
        field_type: FieldType::Simple("integer".to_string()),
        lang_type: None,
        postgres_type_name: None,
        constraints: FieldConstraints {
            foreign_key: Some("NonExistent.id".to_string()),
            ..Default::default()
        },
        ai: None,
        example: None,
    });
    schema.tables.insert("Post".to_string(), table);
    
    let result = SchemaValidator::validate(&schema);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("non-existent field"));
}

#[test]
fn test_validate_relations() {
    let mut schema = Schema::new();
    
    // Create user table
    let user_table = create_basic_table("User", "users");
    schema.tables.insert("User".to_string(), user_table);
    
    // Create post table with relation to user
    let mut post_table = create_basic_table("Post", "posts");
    post_table.fields.insert("user_id".to_string(), Field {
        name: "user_id".to_string(),
        field_type: FieldType::Simple("integer".to_string()),
        lang_type: None,
        postgres_type_name: None,
        constraints: FieldConstraints {
            required: Some(true),
            ..Default::default()
        },
        ai: None,
        example: None,
    });
    
    // Add belongs_to relation
    let mut belongs_to = HashMap::new();
    belongs_to.insert("user".to_string(), BelongsTo {
        model: "User".to_string(),
        local_field: "user_id".to_string(),
        foreign_field: "id".to_string(),
        on_delete: None,
        on_update: None,
        ai: None,
    });
    post_table.relations.belongs_to = Some(belongs_to);
    
    schema.tables.insert("Post".to_string(), post_table);
    
    let result = SchemaValidator::validate(&schema);
    assert!(result.is_ok());
}

#[test]
fn test_validate_invalid_relation() {
    let mut schema = Schema::new();
    
    // Create table with relation to non-existent model
    let mut table = create_basic_table("Post", "posts");
    
    // Add belongs_to relation to non-existent model
    let mut belongs_to = HashMap::new();
    belongs_to.insert("user".to_string(), BelongsTo {
        model: "NonExistent".to_string(),
        local_field: "user_id".to_string(),
        foreign_field: "id".to_string(),
        on_delete: None,
        on_update: None,
        ai: None,
    });
    table.relations.belongs_to = Some(belongs_to);
    
    schema.tables.insert("Post".to_string(), table);
    
    let result = SchemaValidator::validate(&schema);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("non-existent model"));
}

#[test]
fn test_circular_dependency_detection() {
    let mut schema = Schema::new();
    
    // Create tables with circular dependency
    let mut table_a = create_basic_table("TableA", "table_a");
    let mut belongs_to_a = HashMap::new();
    belongs_to_a.insert("table_b".to_string(), BelongsTo {
        model: "TableB".to_string(),
        local_field: "table_b_id".to_string(),
        foreign_field: "id".to_string(),
        on_delete: None,
        on_update: None,
        ai: None,
    });
    table_a.relations.belongs_to = Some(belongs_to_a);
    table_a.fields.insert("table_b_id".to_string(), Field {
        name: "table_b_id".to_string(),
        field_type: FieldType::Simple("integer".to_string()),
        lang_type: None,
        postgres_type_name: None,
        constraints: FieldConstraints::default(),
        ai: None,
        example: None,
    });
    
    let mut table_b = create_basic_table("TableB", "table_b");
    let mut belongs_to_b = HashMap::new();
    belongs_to_b.insert("table_a".to_string(), BelongsTo {
        model: "TableA".to_string(),
        local_field: "table_a_id".to_string(),
        foreign_field: "id".to_string(),
        on_delete: None,
        on_update: None,
        ai: None,
    });
    table_b.relations.belongs_to = Some(belongs_to_b);
    table_b.fields.insert("table_a_id".to_string(), Field {
        name: "table_a_id".to_string(),
        field_type: FieldType::Simple("integer".to_string()),
        lang_type: None,
        postgres_type_name: None,
        constraints: FieldConstraints::default(),
        ai: None,
        example: None,
    });
    
    schema.tables.insert("TableA".to_string(), table_a);
    schema.tables.insert("TableB".to_string(), table_b);
    
    let result = SchemaValidator::validate(&schema);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Circular dependency"));
}

#[test]
fn test_self_reference_allowed() {
    let mut schema = Schema::new();
    
    // Create table with self-reference (should be allowed)
    let mut table = create_basic_table("Category", "categories");
    table.fields.insert("parent_id".to_string(), Field {
        name: "parent_id".to_string(),
        field_type: FieldType::Simple("integer".to_string()),
        lang_type: None,
        postgres_type_name: None,
        constraints: FieldConstraints {
            nullable: Some(true),
            ..Default::default()
        },
        ai: None,
        example: None,
    });
    
    let mut belongs_to = HashMap::new();
    belongs_to.insert("parent".to_string(), BelongsTo {
        model: "Category".to_string(),
        local_field: "parent_id".to_string(),
        foreign_field: "id".to_string(),
        on_delete: None,
        on_update: None,
        ai: None,
    });
    table.relations.belongs_to = Some(belongs_to);
    
    schema.tables.insert("Category".to_string(), table);
    
    // Self-references should be allowed
    let result = SchemaValidator::validate(&schema);
    assert!(result.is_ok());
}

#[test]
fn test_validate_complete_schema() {
    let mut schema = Schema::new();
    
    // Add multiple tables with relations
    schema.tables.insert("User".to_string(), create_basic_table("User", "users"));
    
    let mut post_table = create_basic_table("Post", "posts");
    post_table.fields.insert("user_id".to_string(), Field {
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
    
    let mut belongs_to = HashMap::new();
    belongs_to.insert("user".to_string(), BelongsTo {
        model: "User".to_string(),
        local_field: "user_id".to_string(),
        foreign_field: "id".to_string(),
        on_delete: None,
        on_update: None,
        ai: None,
    });
    post_table.relations.belongs_to = Some(belongs_to);
    
    schema.tables.insert("Post".to_string(), post_table);
    
    // Should validate successfully
    let result = SchemaValidator::validate(&schema);
    assert!(result.is_ok());
}