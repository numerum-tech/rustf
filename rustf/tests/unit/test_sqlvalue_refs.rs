#[cfg(test)]
mod tests {
    use rustf::models::{SqlValue, QueryBuilder, DatabaseBackend};

    #[test]
    fn test_from_string_ref() {
        let email = String::from("test@example.com");
        let email_ref = &email;
        
        // This should compile now
        let value: SqlValue = email_ref.into();
        match value {
            SqlValue::String(s) => assert_eq!(s, "test@example.com"),
            _ => panic!("Expected SqlValue::String"),
        }
    }
    
    #[test]
    fn test_from_i32_ref() {
        let age = 25i32;
        let age_ref = &age;
        
        let value: SqlValue = age_ref.into();
        match value {
            SqlValue::Int(i) => assert_eq!(i, 25),
            _ => panic!("Expected SqlValue::Int"),
        }
    }
    
    #[test]
    fn test_from_bool_ref() {
        let is_active = true;
        let active_ref = &is_active;
        
        let value: SqlValue = active_ref.into();
        match value {
            SqlValue::Bool(b) => assert_eq!(b, true),
            _ => panic!("Expected SqlValue::Bool"),
        }
    }
    
    #[test]
    fn test_where_eq_with_string_ref() {
        let email = String::from("test@example.com");
        let email_ref = &email;
        
        // This should compile and work
        let query = QueryBuilder::new(DatabaseBackend::Postgres)
            .from("users")
            .where_eq("email", email_ref);
        
        let result = query.build();
        assert!(result.is_ok());
        
        let (sql, params) = result.unwrap();
        assert!(sql.contains("WHERE"));
        assert_eq!(params.len(), 1);
    }
}