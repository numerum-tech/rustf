#[cfg(test)]
mod tests {
    use rustf::models::ModelFilter;

    #[test]
    fn test_filter_creation_basic() {
        // Test that filters can be created and chained
        let filter = ModelFilter::new()
            .where_eq("status", "active")
            .where_gt("age", 18)
            .where_not_null("email");

        assert!(!filter.is_empty());
        assert_eq!(filter.len(), 3);
    }

    #[test]
    fn test_filter_chaining_and_combination() {
        // Test creating complex filters through chaining
        let base_filter = ModelFilter::new().where_eq("tenant_id", 123);

        let active_filter = ModelFilter::new()
            .where_eq("is_active", true)
            .where_not_null("verified_at");

        let combined = base_filter.and(active_filter);

        // Should have all 3 conditions
        assert_eq!(combined.len(), 3);
    }

    #[test]
    fn test_filter_with_in_clause() {
        let filter = ModelFilter::new()
            .where_in("role", vec!["admin", "moderator", "editor"])
            .where_not_in("status", vec!["banned", "deleted"]);

        assert_eq!(filter.len(), 2);

        // The filter should contain both IN conditions
    }

    #[test]
    fn test_filter_between_clause() {
        let filter = ModelFilter::new()
            .where_between("age", 18, 65)
            .where_between("created_at", "2024-01-01", "2024-12-31");

        assert_eq!(filter.len(), 2);

        // The filter should contain both BETWEEN conditions
    }

    #[test]
    fn test_filter_like_patterns() {
        let filter = ModelFilter::new()
            .where_like("email", "%@gmail.com")
            .where_not_like("username", "test%")
            .where_like("name", "%john%");

        assert_eq!(filter.len(), 3);

        // The filter should contain all LIKE conditions
    }

    #[test]
    fn test_empty_filter_behavior() {
        let filter = ModelFilter::new();

        assert!(filter.is_empty());
        assert_eq!(filter.len(), 0);

        // Empty filter should be ready to use without errors
    }

    #[test]
    fn test_filter_with_null_checks() {
        let filter = ModelFilter::new()
            .where_null("deleted_at")
            .where_not_null("verified_at")
            .where_not_null("email");

        assert_eq!(filter.len(), 3);

        // The filter should contain all null check conditions
    }

    #[test]
    fn test_filter_numeric_comparisons() {
        let filter = ModelFilter::new()
            .where_gt("age", 18)
            .where_gte("score", 60)
            .where_lt("errors", 10)
            .where_lte("warnings", 5);

        assert_eq!(filter.len(), 4);

        // The filter should contain all comparison conditions
    }

    #[test]
    fn test_filter_combination_preserves_all_conditions() {
        let filter1 = ModelFilter::new()
            .where_eq("tenant_id", 1)
            .where_eq("department", "engineering");

        let filter2 = ModelFilter::new()
            .where_in("role", vec!["developer", "lead"])
            .where_not_null("github_username");

        let filter3 = ModelFilter::new().where_gte("experience_years", 3);

        // Chain multiple filters together
        let combined = filter1.and(filter2).and(filter3);

        // Should have all 5 conditions
        assert_eq!(combined.len(), 5);

        // All conditions from all filters should be preserved
    }
}
