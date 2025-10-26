#[cfg(test)]
mod tests {
    use rustf::error::{Error, ErrorChain, ErrorContext, RetryPolicy};

    #[test]
    fn test_new_database_errors() {
        let err = Error::database_connection("Connection failed");
        assert_eq!(err.error_code(), "E_DB_CONNECTION");
        assert_eq!(err.status_code(), 500);
        assert!(err.is_retryable());

        let err = Error::database_query("Invalid SQL");
        assert_eq!(err.error_code(), "E_DB_QUERY");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_error_context_chaining() {
        let result: Result<(), Error> = Err(Error::database_connection("Connection refused"));
        let with_context = result.context("Failed to initialize database");

        assert!(with_context.is_err());
        let error = with_context.unwrap_err();

        let chain = ErrorChain::new(&error);
        assert_eq!(chain.chain().len(), 2);
    }

    #[test]
    fn test_auth_errors() {
        let err = Error::authentication("Invalid credentials");
        assert_eq!(err.error_code(), "E_AUTH");
        assert_eq!(err.status_code(), 401);

        let err = Error::authorization("Insufficient permissions");
        assert_eq!(err.error_code(), "E_AUTHZ");
        assert_eq!(err.status_code(), 403);

        let err = Error::rate_limit("Too many requests");
        assert_eq!(err.error_code(), "E_RATE_LIMIT");
        assert_eq!(err.status_code(), 429);
    }

    #[tokio::test]
    async fn test_retry_policy() {
        let policy = RetryPolicy::exponential(3);
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.backoff_multiplier, 2.0);

        let policy = RetryPolicy::fixed(5, std::time::Duration::from_secs(1));
        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.backoff_multiplier, 1.0);
    }

    #[test]
    fn test_external_service_errors() {
        let err = Error::external_service("payment_gateway", "Service unavailable");
        assert_eq!(err.error_code(), "E_EXTERNAL_SERVICE");
        assert!(err.is_retryable());

        let err = Error::timeout("Request timed out");
        assert_eq!(err.error_code(), "E_TIMEOUT");
        assert_eq!(err.status_code(), 408);
        assert!(err.is_retryable());
    }
}
