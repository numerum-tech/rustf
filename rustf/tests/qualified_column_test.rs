#[cfg(test)]
mod tests {
    use rustf::models::query_builder::{DatabaseBackend, OrderDirection, QueryBuilder};

    #[test]
    fn test_qualified_column_names_in_where() {
        // Test PostgreSQL
        let query = QueryBuilder::new(DatabaseBackend::Postgres)
            .from("settlement_banks")
            .select(vec!["settlement_banks.*", "system_nodes.name"])
            .left_join("system_nodes", "system_nodes.id = settlement_banks.node_id")
            .where_eq("settlement_banks.is_active", true)
            .order_by("system_nodes.created_at", OrderDirection::Desc)
            .limit(10);

        let (sql, _params) = query.build().unwrap();

        // Should NOT have quotes around the qualified column name
        assert!(!sql.contains(r#""settlement_banks.is_active""#));
        assert!(sql.contains("settlement_banks.is_active"));
        assert!(sql.contains("system_nodes.created_at"));

        // But should still quote simple table names
        assert!(sql.contains(r#""settlement_banks""#));
        assert!(sql.contains(r#""system_nodes""#));
    }

    #[test]
    fn test_simple_column_names_still_quoted() {
        // Test that simple column names are still quoted
        let query = QueryBuilder::new(DatabaseBackend::Postgres)
            .from("users")
            .where_eq("email", "test@example.com")
            .where_eq("active", true);

        let (sql, _params) = query.build().unwrap();

        // Simple table name should be quoted
        assert!(sql.contains(r#""users""#));
        // Simple column names should be quoted
        assert!(sql.contains(r#""email""#));
        assert!(sql.contains(r#""active""#));
    }

    #[test]
    fn test_mysql_qualified_columns() {
        let query = QueryBuilder::new(DatabaseBackend::MySQL)
            .from("posts")
            .left_join("users", "users.id = posts.user_id")
            .where_eq("posts.published", true)
            .where_eq("users.role", "admin");

        let (sql, _params) = query.build().unwrap();

        // MySQL should not quote qualified names
        assert!(sql.contains("posts.published"));
        assert!(sql.contains("users.role"));
        // But should quote simple table names with backticks
        assert!(sql.contains("`posts`"));
        assert!(sql.contains("`users`"));
    }
}
