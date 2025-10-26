#[cfg(test)]
mod tests {
    use rustf::models::ModelFilter;

    #[test]
    fn test_filter_creation_and_conditions() {
        // Test basic filter creation
        let filter = ModelFilter::new();
        assert!(filter.is_empty());
        assert_eq!(filter.len(), 0);

        // Test adding conditions
        let filter = ModelFilter::new()
            .where_eq("status", "active")
            .where_gt("age", 18)
            .where_not_null("email");

        assert!(!filter.is_empty());
        assert_eq!(filter.len(), 3);

        // Test filter combination
        let filter1 = ModelFilter::new().where_eq("role", "admin");

        let filter2 = ModelFilter::new().where_eq("is_active", true);

        let combined = filter1.and(filter2);
        assert_eq!(combined.len(), 2);
    }

    #[test]
    fn test_filter_with_various_conditions() {
        let filter = ModelFilter::new()
            .where_eq("name", "John")
            .where_ne("status", "deleted")
            .where_gt("age", 21)
            .where_gte("score", 90)
            .where_lt("errors", 5)
            .where_lte("warnings", 10)
            .where_like("email", "%@example.com")
            .where_not_like("username", "test%")
            .where_in("role", vec!["admin", "moderator"])
            .where_not_in("status", vec!["banned", "suspended"])
            .where_null("deleted_at")
            .where_not_null("verified_at")
            .where_between("created_at", "2024-01-01", "2024-12-31");

        assert_eq!(filter.len(), 13);
    }
}
