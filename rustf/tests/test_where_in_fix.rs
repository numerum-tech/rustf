use rustf::database::types::SqlValue;
use rustf::models::query_builder::{DatabaseBackend, QueryBuilder};

#[test]
fn test_where_in_no_params() {
    let query = QueryBuilder::new(DatabaseBackend::Postgres)
        .from("users")
        .select(vec!["*"])
        .where_in("status", vec!["active", "pending"]);

    let (sql, params) = query.build().unwrap();

    // Should have IN clause in SQL
    assert!(
        sql.contains("IN ('active', 'pending')"),
        "Should have IN clause with values"
    );

    // Should NOT generate any parameters
    assert_eq!(params.len(), 0, "where_in should not generate parameters");
}

#[test]
fn test_or_where_in_no_params() {
    let query = QueryBuilder::new(DatabaseBackend::Postgres)
        .from("users")
        .select(vec!["*"])
        .where_eq("active", true)
        .or_where_in("role", vec!["admin", "moderator"]);

    let (sql, params) = query.build().unwrap();

    // Should have IN clause in SQL
    assert!(
        sql.contains("IN ('admin', 'moderator')"),
        "Should have IN clause with values"
    );

    // Should only have 1 parameter (for active = true)
    assert_eq!(
        params.len(),
        1,
        "Should only have 1 param for active condition"
    );
    assert!(matches!(&params[0], SqlValue::Bool(true)));
}

#[test]
fn test_where_not_in_no_params() {
    let query = QueryBuilder::new(DatabaseBackend::Postgres)
        .from("users")
        .select(vec!["*"])
        .where_not_in("status", vec!["deleted", "banned"]);

    let (sql, params) = query.build().unwrap();

    // Should have NOT IN clause in SQL
    assert!(
        sql.contains("NOT IN ('deleted', 'banned')"),
        "Should have NOT IN clause with values"
    );

    // Should NOT generate any parameters
    assert_eq!(
        params.len(),
        0,
        "where_not_in should not generate parameters"
    );
}

#[test]
fn test_where_between_no_params() {
    let query = QueryBuilder::new(DatabaseBackend::Postgres)
        .from("users")
        .select(vec!["*"])
        .where_between("age", 18, 65);

    let (sql, params) = query.build().unwrap();

    // Should have BETWEEN clause in SQL
    // Note: The values are stored as a string, so they get quoted
    assert!(
        sql.contains("BETWEEN '18 AND 65'"),
        "Should have BETWEEN clause with values"
    );

    // Should NOT generate any parameters
    assert_eq!(
        params.len(),
        0,
        "where_between should not generate parameters"
    );
}

#[test]
fn test_mixed_in_and_regular_conditions() {
    let query = QueryBuilder::new(DatabaseBackend::Postgres)
        .from("users")
        .select(vec!["*"])
        .where_eq("is_verified", true)
        .where_in("status", vec!["active", "pending"])
        .where_gt("created_at", "2024-01-01")
        .where_not_in("role", vec!["banned"])
        .where_lt("age", 100);

    let (sql, params) = query.build().unwrap();

    // Check IN clauses are in SQL
    assert!(sql.contains("IN ('active', 'pending')"));
    assert!(sql.contains("NOT IN ('banned')"));

    // Should have 3 params: is_verified, created_at, age (not the IN clauses)
    assert_eq!(
        params.len(),
        3,
        "Should have 3 params for non-IN conditions"
    );
    assert!(matches!(&params[0], SqlValue::Bool(true)));
    assert!(matches!(&params[1], SqlValue::String(s) if s == "2024-01-01"));
    assert!(matches!(&params[2], SqlValue::Int(100)));
}
