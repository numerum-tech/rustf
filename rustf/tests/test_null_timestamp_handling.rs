use rustf::database::types::SqlValue;
use rustf::models::query_builder::{DatabaseBackend, QueryBuilder};
use std::collections::HashMap;

#[test]
fn test_null_in_insert() {
    let mut data = HashMap::new();
    data.insert("name".to_string(), SqlValue::String("John".to_string()));
    data.insert("email".to_string(), SqlValue::Null);
    data.insert("age".to_string(), SqlValue::from(Some(25)));
    data.insert("phone".to_string(), SqlValue::from(None::<String>));

    let query = QueryBuilder::new(DatabaseBackend::Postgres).from("users");

    let (sql, params) = query.build_insert(&data).unwrap();

    // NULL should appear directly in SQL, not as a parameter
    assert!(sql.contains("NULL"), "SQL should contain literal NULL");

    // Only non-null values should be in params
    let null_count = params
        .iter()
        .filter(|p| matches!(p, SqlValue::Null))
        .count();
    assert_eq!(
        null_count, 0,
        "NULL should not be in params, it should be in SQL directly"
    );

    // Should have 2 params: name and age (email and phone are NULL)
    assert_eq!(params.len(), 2, "Should only have 2 non-null params");
}

#[test]
fn test_where_null_conditions() {
    let query = QueryBuilder::new(DatabaseBackend::Postgres)
        .from("users")
        .select(vec!["*"])
        .where_null("deleted_at")
        .where_not_null("verified_at");

    let (sql, params) = query.build().unwrap();

    // Check SQL contains IS NULL and IS NOT NULL
    assert!(sql.contains("IS NULL"), "Should have IS NULL in SQL");
    assert!(
        sql.contains("IS NOT NULL"),
        "Should have IS NOT NULL in SQL"
    );

    // No parameters should be generated for NULL checks
    assert_eq!(
        params.len(),
        0,
        "NULL checks should not generate parameters"
    );
}

#[test]
fn test_mixed_null_and_values() {
    let query = QueryBuilder::new(DatabaseBackend::Postgres)
        .from("users")
        .select(vec!["*"])
        .where_eq("status", "active")
        .where_null("deleted_at")
        .where_eq("role", "admin");

    let (sql, params) = query.build().unwrap();

    // Should have IS NULL in SQL
    assert!(sql.contains("IS NULL"), "Should have IS NULL in SQL");

    // Should have 2 params (status and role, not deleted_at)
    assert_eq!(
        params.len(),
        2,
        "Should have 2 params for non-null conditions"
    );
    assert!(matches!(&params[0], SqlValue::String(s) if s == "active"));
    assert!(matches!(&params[1], SqlValue::String(s) if s == "admin"));
}

#[test]
fn test_update_with_null() {
    let mut data = HashMap::new();
    data.insert("name".to_string(), SqlValue::String("Jane".to_string()));
    data.insert("deleted_at".to_string(), SqlValue::Null);
    data.insert(
        "updated_at".to_string(),
        SqlValue::DateTime("2024-01-01T12:00:00Z".to_string()),
    );

    let query = QueryBuilder::new(DatabaseBackend::Postgres)
        .from("users")
        .where_eq("id", 1);

    let (sql, params) = query.build_update(&data).unwrap();

    // NULL should appear in SET clause
    assert!(
        sql.contains("= NULL"),
        "Should have = NULL in UPDATE SET clause"
    );

    // Should have 3 params: name, updated_at, and id (deleted_at is NULL)
    let null_count = params
        .iter()
        .filter(|p| matches!(p, SqlValue::Null))
        .count();
    assert_eq!(null_count, 0, "NULL should not be in params");
    assert_eq!(params.len(), 3, "Should have 3 non-null params");
}

#[test]
fn test_timestamp_handling() {
    let mut data = HashMap::new();
    data.insert("created_at".to_string(), SqlValue::Timestamp(1704067200)); // Unix timestamp
    data.insert(
        "updated_at".to_string(),
        SqlValue::DateTime("2024-01-01T12:00:00Z".to_string()),
    );

    let query = QueryBuilder::new(DatabaseBackend::Postgres).from("events");

    let (sql, params) = query.build_insert(&data).unwrap();

    // Both timestamps should be parameters
    assert_eq!(params.len(), 2, "Should have 2 timestamp params");

    // Check timestamp types are preserved
    let has_timestamp = params.iter().any(|p| matches!(p, SqlValue::Timestamp(_)));
    let has_datetime = params.iter().any(|p| matches!(p, SqlValue::DateTime(_)));
    assert!(has_timestamp, "Should have Timestamp value");
    assert!(has_datetime, "Should have DateTime value");
}

#[test]
fn test_option_none_becomes_null() {
    // Test that Option::None becomes SqlValue::Null
    let none_value: Option<String> = None;
    let sql_value: SqlValue = none_value.into();
    assert!(
        matches!(sql_value, SqlValue::Null),
        "None should become SqlValue::Null"
    );

    // Test that Some(value) becomes the value
    let some_value: Option<String> = Some("test".to_string());
    let sql_value: SqlValue = some_value.into();
    assert!(
        matches!(sql_value, SqlValue::String(s) if s == "test"),
        "Some should unwrap to value"
    );
}

#[test]
fn test_null_in_where_in() {
    // where_in with mixed values including NULL handling
    let query = QueryBuilder::new(DatabaseBackend::Postgres)
        .from("users")
        .select(vec!["*"])
        .where_in(
            "status",
            vec![
                SqlValue::String("active".to_string()),
                SqlValue::String("pending".to_string()),
            ],
        )
        .or_where_null("status");

    let (sql, params) = query.build().unwrap();

    // Should have IN clause and IS NULL
    assert!(sql.contains("IN ("), "Should have IN clause");
    assert!(sql.contains("IS NULL"), "Should have IS NULL");

    // IN values are inlined, not parameterized in current implementation
    // The or_where_null should not add a parameter
    assert_eq!(
        params.len(),
        0,
        "where_in and where_null don't use params in current impl"
    );
}

#[test]
fn test_null_not_stringified() {
    // Ensure NULL is never converted to 'NULL' string
    let mut data = HashMap::new();
    data.insert("value".to_string(), SqlValue::Null);

    let query = QueryBuilder::new(DatabaseBackend::Postgres).from("test");

    let (sql, _params) = query.build_insert(&data).unwrap();

    // Should have NULL, not 'NULL'
    assert!(sql.contains("NULL"), "Should contain NULL");
    assert!(
        !sql.contains("'NULL'"),
        "Should NOT contain 'NULL' as string"
    );
}

#[test]
fn test_mysql_null_handling() {
    // Test that MySQL also handles NULL correctly
    let mut data = HashMap::new();
    data.insert("name".to_string(), SqlValue::String("Test".to_string()));
    data.insert("optional".to_string(), SqlValue::Null);

    let query = QueryBuilder::new(DatabaseBackend::MySQL).from("test");

    let (sql, params) = query.build_insert(&data).unwrap();

    // MySQL should also have NULL in SQL, not as parameter
    assert!(sql.contains("NULL"), "MySQL should have NULL in SQL");
    assert!(
        !params.iter().any(|p| matches!(p, SqlValue::Null)),
        "NULL shouldn't be in params"
    );
}

#[test]
fn test_delete_with_null_condition() {
    let query = QueryBuilder::new(DatabaseBackend::Postgres)
        .from("users")
        .where_null("deleted_at")
        .where_eq("status", "inactive");

    let (sql, params) = query.build_delete().unwrap();

    // Should have IS NULL in WHERE
    assert!(
        sql.contains("IS NULL"),
        "DELETE should have IS NULL condition"
    );

    // Only status should be a parameter
    assert_eq!(params.len(), 1, "Should have 1 param for status");
    assert!(matches!(&params[0], SqlValue::String(s) if s == "inactive"));
}
