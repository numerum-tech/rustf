use rustf::database::types::SqlValue;
use rustf::models::query_builder::{DatabaseBackend, QueryBuilder};

#[test]
fn test_enum_in_select_where() {
    let query = QueryBuilder::new(DatabaseBackend::Postgres)
        .from("users")
        .select(vec!["id", "name", "status"])
        .where_eq("status", "ACTIVE::user_status")
        .where_eq("role", "ADMIN::user_role");

    let (sql, params) = query.build().unwrap();

    // Check SQL contains type casting
    assert!(
        sql.contains("$1::user_status"),
        "Missing type cast for status enum"
    );
    assert!(
        sql.contains("$2::user_role"),
        "Missing type cast for role enum"
    );

    // Check params have enum values
    assert_eq!(params.len(), 2);
    match &params[0] {
        SqlValue::Enum(s) => assert_eq!(s, "ACTIVE::user_status"),
        _ => panic!("Expected SqlValue::Enum for status"),
    }
    match &params[1] {
        SqlValue::Enum(s) => assert_eq!(s, "ADMIN::user_role"),
        _ => panic!("Expected SqlValue::Enum for role"),
    }
}

#[test]
fn test_enum_in_update_where() {
    use std::collections::HashMap;

    let mut data = HashMap::new();
    data.insert("name".to_string(), SqlValue::String("John Doe".to_string()));
    data.insert(
        "updated_at".to_string(),
        SqlValue::String("2024-01-01".to_string()),
    );

    let query = QueryBuilder::new(DatabaseBackend::Postgres)
        .from("users")
        .where_eq("status", "ACTIVE::user_status")
        .where_eq("role", "ADMIN::user_role");

    let (sql, params) = query.build_update(&data).unwrap();

    // Check SQL contains type casting in WHERE clause
    assert!(
        sql.contains("$3::user_status"),
        "Missing type cast for status enum in UPDATE WHERE"
    );
    assert!(
        sql.contains("$4::user_role"),
        "Missing type cast for role enum in UPDATE WHERE"
    );

    // Check params: first 2 are UPDATE values, next 2 are WHERE enums
    assert!(params.len() >= 4);
    match &params[2] {
        SqlValue::Enum(s) => assert_eq!(s, "ACTIVE::user_status"),
        _ => panic!("Expected SqlValue::Enum for status in UPDATE WHERE"),
    }
    match &params[3] {
        SqlValue::Enum(s) => assert_eq!(s, "ADMIN::user_role"),
        _ => panic!("Expected SqlValue::Enum for role in UPDATE WHERE"),
    }
}

#[test]
fn test_enum_in_delete_where() {
    let query = QueryBuilder::new(DatabaseBackend::Postgres)
        .from("users")
        .where_eq("status", "INACTIVE::user_status")
        .where_eq("role", "GUEST::user_role");

    let (sql, params) = query.build_delete().unwrap();

    // Check SQL contains type casting in WHERE clause
    assert!(
        sql.contains("$1::user_status"),
        "Missing type cast for status enum in DELETE WHERE"
    );
    assert!(
        sql.contains("$2::user_role"),
        "Missing type cast for role enum in DELETE WHERE"
    );

    // Check params have enum values
    assert_eq!(params.len(), 2);
    match &params[0] {
        SqlValue::Enum(s) => assert_eq!(s, "INACTIVE::user_status"),
        _ => panic!("Expected SqlValue::Enum for status in DELETE"),
    }
    match &params[1] {
        SqlValue::Enum(s) => assert_eq!(s, "GUEST::user_role"),
        _ => panic!("Expected SqlValue::Enum for role in DELETE"),
    }
}

#[test]
fn test_enum_in_insert() {
    use std::collections::HashMap;

    let mut data = HashMap::new();
    data.insert("name".to_string(), SqlValue::String("Jane Doe".to_string()));
    data.insert(
        "status".to_string(),
        SqlValue::Enum("ACTIVE::user_status".to_string()),
    );
    data.insert(
        "role".to_string(),
        SqlValue::Enum("USER::user_role".to_string()),
    );

    let query = QueryBuilder::new(DatabaseBackend::Postgres).from("users");

    let (sql, params) = query.build_insert(&data).unwrap();

    // Check SQL contains type casting for enum values
    // The order of fields might vary due to HashMap, but we should see type casts
    assert!(
        sql.contains("::user_status") || sql.contains("::user_role"),
        "Missing type cast for enums in INSERT"
    );

    // Check params contain enum values
    let enum_count = params
        .iter()
        .filter(|p| matches!(p, SqlValue::Enum(_)))
        .count();
    assert_eq!(enum_count, 2, "Should have 2 enum values in INSERT params");
}

#[test]
fn test_enum_in_update_set_and_where() {
    use std::collections::HashMap;

    let mut data = HashMap::new();
    data.insert(
        "status".to_string(),
        SqlValue::Enum("PENDING::user_status".to_string()),
    );
    data.insert(
        "updated_at".to_string(),
        SqlValue::String("2024-01-01".to_string()),
    );

    let query = QueryBuilder::new(DatabaseBackend::Postgres)
        .from("users")
        .where_eq("role", "ADMIN::user_role");

    let (sql, params) = query.build_update(&data).unwrap();

    // Check for type casting in both SET and WHERE
    assert!(
        sql.contains("::user_status"),
        "Missing type cast for status enum in UPDATE SET"
    );
    assert!(
        sql.contains("::user_role"),
        "Missing type cast for role enum in UPDATE WHERE"
    );

    // Verify params
    let enum_count = params
        .iter()
        .filter(|p| matches!(p, SqlValue::Enum(_)))
        .count();
    assert_eq!(
        enum_count, 2,
        "Should have 2 enum values (1 in SET, 1 in WHERE)"
    );
}

#[test]
fn test_mixed_where_conditions_with_enum() {
    let query = QueryBuilder::new(DatabaseBackend::Postgres)
        .from("users")
        .select(vec!["*"])
        .where_eq("status", "ACTIVE::user_status")
        .where_gt("age", 18)
        .where_like("email", "%@example.com")
        .or_where_eq("role", "SUPERADMIN::user_role");

    let (sql, params) = query.build().unwrap();

    // Check type casting for enums only
    assert!(
        sql.contains("$1::user_status"),
        "Missing type cast for status enum"
    );
    assert!(
        sql.contains("$4::user_role"),
        "Missing type cast for role enum"
    );
    assert!(!sql.contains("$2::"), "Should not have type cast for age");
    assert!(
        !sql.contains("$3::"),
        "Should not have type cast for email pattern"
    );

    // Verify param order and types
    assert_eq!(params.len(), 4);
    assert!(matches!(&params[0], SqlValue::Enum(_)));
    assert!(matches!(&params[1], SqlValue::Int(18)));
    assert!(matches!(&params[2], SqlValue::String(_)));
    assert!(matches!(&params[3], SqlValue::Enum(_)));
}

#[test]
fn test_enum_without_type_info() {
    // Test that regular enum values without :: work normally
    let query = QueryBuilder::new(DatabaseBackend::Postgres)
        .from("users")
        .select(vec!["*"])
        .where_eq("status", SqlValue::Enum("ACTIVE".to_string()));

    let (sql, params) = query.build().unwrap();

    // Should not have type casting
    assert!(
        !sql.contains("::"),
        "Should not have type cast for enum without type info"
    );

    // But should still be an enum
    assert_eq!(params.len(), 1);
    match &params[0] {
        SqlValue::Enum(s) => assert_eq!(s, "ACTIVE"),
        _ => panic!("Expected SqlValue::Enum"),
    }
}

#[test]
fn test_enum_mysql_no_type_cast() {
    // MySQL doesn't support :: type casting
    let query = QueryBuilder::new(DatabaseBackend::MySQL)
        .from("users")
        .select(vec!["*"])
        .where_eq("status", "ACTIVE::user_status");

    let (sql, params) = query.build().unwrap();

    // MySQL should not have type casting syntax
    assert!(
        !sql.contains("::user_status"),
        "MySQL should not have PostgreSQL type cast syntax"
    );
    assert!(sql.contains("?"), "MySQL should use ? placeholders");

    // But param should still be an enum
    assert_eq!(params.len(), 1);
    match &params[0] {
        SqlValue::Enum(s) => assert_eq!(s, "ACTIVE::user_status"),
        _ => panic!("Expected SqlValue::Enum"),
    }
}
