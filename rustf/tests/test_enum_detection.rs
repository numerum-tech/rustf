use rustf::models::query_builder::{DatabaseBackend, QueryBuilder, SqlValue};

#[test]
fn test_enum_auto_detection_from_string() {
    // Test that strings with :: are automatically detected as enums
    let value1: SqlValue = "SETTLEMENT::node_type_enum".into();
    match value1 {
        SqlValue::Enum(s) => assert_eq!(s, "SETTLEMENT::node_type_enum"),
        _ => panic!("Expected SqlValue::Enum, got {:?}", value1),
    }

    // Test that regular strings remain strings
    let value2: SqlValue = "regular_string".into();
    match value2 {
        SqlValue::String(s) => assert_eq!(s, "regular_string"),
        _ => panic!("Expected SqlValue::String, got {:?}", value2),
    }
}

#[test]
fn test_enum_auto_detection_from_str() {
    // Test &str conversion
    let value1: SqlValue = ("ADMIN::user_role" as &str).into();
    match value1 {
        SqlValue::Enum(s) => assert_eq!(s, "ADMIN::user_role"),
        _ => panic!("Expected SqlValue::Enum, got {:?}", value1),
    }
}

#[test]
fn test_enum_in_where_clause_postgres() {
    // Test that enum values work correctly in WHERE clauses for PostgreSQL
    let query = QueryBuilder::new(DatabaseBackend::Postgres)
        .from("system_nodes")
        .where_eq("node_type", "SETTLEMENT::node_type_enum")
        .where_eq("operation_mode", "AUTONOMOUS::operation_mode_enum");

    let (sql, params) = query.build().unwrap();

    // Check SQL contains proper enum casting
    assert!(
        sql.contains("$1::node_type_enum"),
        "SQL should contain enum cast: {}",
        sql
    );
    assert!(
        sql.contains("$2::operation_mode_enum"),
        "SQL should contain enum cast: {}",
        sql
    );

    // Check parameters are SqlValue::Enum
    assert_eq!(params.len(), 2);
    match &params[0] {
        SqlValue::Enum(s) => assert!(s.contains("SETTLEMENT")),
        _ => panic!(
            "Expected SqlValue::Enum for first param, got {:?}",
            params[0]
        ),
    }
    match &params[1] {
        SqlValue::Enum(s) => assert!(s.contains("AUTONOMOUS")),
        _ => panic!(
            "Expected SqlValue::Enum for second param, got {:?}",
            params[1]
        ),
    }
}

#[test]
fn test_enum_in_where_clause_mysql() {
    // Test that enum values work for non-PostgreSQL databases (no casting)
    let query = QueryBuilder::new(DatabaseBackend::MySQL)
        .from("system_nodes")
        .where_eq("status", "ACTIVE::status_enum");

    let (sql, params) = query.build().unwrap();

    // MySQL should not have type casting
    assert!(
        !sql.contains("::status_enum"),
        "MySQL SQL should not contain type cast: {}",
        sql
    );

    // But parameter should still be SqlValue::Enum
    assert_eq!(params.len(), 1);
    match &params[0] {
        SqlValue::Enum(s) => assert_eq!(s, "ACTIVE::status_enum"),
        _ => panic!("Expected SqlValue::Enum, got {:?}", params[0]),
    }
}
