# RustF Error System Documentation

## Overview

RustF provides a comprehensive, production-ready error handling system designed for building robust web applications. The error system emphasizes safety, observability, and developer experience while providing enterprise-grade features like retry logic, error chaining, and circuit breakers.

### Core Principles

- **Type Safety**: Strongly typed errors with exhaustive pattern matching
- **Production Safety**: Sanitized error messages, no sensitive data leakage
- **Observability**: Rich context and structured logging for debugging
- **Resilience**: Built-in retry logic and circuit breaker patterns
- **Developer Experience**: Clear error messages and intuitive APIs

## Error Types

### Main Error Enum

The `rustf::error::Error` enum provides comprehensive error variants for all common scenarios:

```rust
use rustf::error::Error;
```

### Core Error Variants

#### HTTP and Network Errors
- `Http(hyper::Error)` - HTTP protocol errors
- `Network(String)` - Network connectivity issues
- `Timeout(String)` - Request timeout errors
- `ExternalService { service, message }` - External API failures

#### Data Processing Errors
- `Json(serde_json::Error)` - JSON serialization/deserialization
- `Validation(String)` - Data validation failures
- `InvalidInput(String)` - User input errors
- `Template(String)` - Template rendering errors

#### Database Errors
- `DatabaseConnection(String)` - Connection pool issues
- `DatabaseQuery(String)` - SQL query errors
- `DatabaseTransaction(String)` - Transaction failures
- `DatabaseMigration(String)` - Migration errors
- `DatabasePool(String)` - Connection pool exhaustion

#### Authentication & Authorization
- `Authentication(String)` - Authentication failures (401)
- `Authorization(String)` - Permission denied (403)
- `RateLimit(String)` - Rate limiting (429)
- `Session(String)` - Session management errors

#### Application Errors
- `RouteNotFound(String)` - Route not found (404)
- `ModelNotFound(String)` - Model/resource not found (404)
- `Internal(String)` - Internal server errors (500)
- `Io(std::io::Error)` - File system I/O errors

#### Special Errors
- `WithContext { message, source }` - Error with context chain
- `Redis(redis::RedisError)` - Redis errors (feature-gated)
- `RedisPool(String)` - Redis connection pool errors

## Error Creation and Handling

### Creating Errors

RustF provides convenient constructors for all error types:

```rust
// Database errors
Error::database_connection("Failed to connect to PostgreSQL")
Error::database_query("Invalid SQL syntax near 'FORM'")
Error::database_transaction("Transaction rolled back")

// Authentication errors
Error::authentication("Invalid credentials")
Error::authorization("Insufficient permissions for this resource")
Error::rate_limit("API rate limit exceeded: 100 requests per minute")

// External service errors
Error::external_service("payment_gateway", "Service temporarily unavailable")
Error::timeout("Request timed out after 30 seconds")

// Validation errors
Error::validation("Email address is invalid")
Error::invalid_input("Age must be a positive number")
```

### Error Propagation

Use the `?` operator for clean error propagation:

```rust
async fn process_user_registration(data: UserData) -> Result<User> {
    // Validate input
    validate_email(&data.email)?;
    validate_password(&data.password)?;
    
    // Check for existing user
    if User::exists_by_email(&data.email).await? {
        return Err(Error::validation("Email already registered"));
    }
    
    // Create user in database
    let user = User::create(data).await?;
    
    // Send welcome email
    email_service::send_welcome(&user).await
        .map_err(|e| Error::external_service("email", e.to_string()))?;
    
    Ok(user)
}
```

### Error Properties

Each error has useful properties:

```rust
let error = Error::authentication("Invalid token");

// Get machine-readable error code
assert_eq!(error.error_code(), "E_AUTH");

// Get appropriate HTTP status code
assert_eq!(error.status_code(), 401);

// Check if error is retryable
assert!(!error.is_retryable());
```

## Error Context and Chaining

### Adding Context to Errors

Use the `ErrorContext` trait to add contextual information:

```rust
use rustf::error::ErrorContext;

async fn fetch_user_profile(id: u64) -> Result<Profile> {
    let user = fetch_user_from_db(id).await
        .context("Failed to fetch user from database")?;
    
    let profile = build_profile(user).await
        .context(format!("Failed to build profile for user {}", id))?;
    
    Ok(profile)
}

// With lazy evaluation
result.with_context(|| format!("Operation failed at {}", timestamp))
```

### Error Chains

Build and inspect error chains for better debugging:

```rust
use rustf::error::ErrorChain;

// When an error occurs with context
let error = database_operation()
    .context("Failed to update user")
    .context("Cannot complete profile update")
    .unwrap_err();

// Inspect the error chain
let chain = ErrorChain::new(&error);

// Get all messages in the chain
for message in chain.chain() {
    println!("- {}", message);
}

// Get the root cause
let root = chain.root_cause();
println!("Root cause: {}", root);

// Format for logging
log::error!("{}", chain.format_for_log());
```

### Option to Error Conversion

Convert `Option` to `Error` with context:

```rust
use rustf::error::OptionExt;

let value = some_option
    .context("Expected value to be present")?;

// With lazy evaluation
let config = config_value
    .with_context(|| format!("Missing config key: {}", key))?;
```

## Retry Logic

### Retry Policies

RustF provides configurable retry policies for handling transient failures:

```rust
use rustf::error::{RetryPolicy, with_retry};

// Exponential backoff (recommended for network operations)
let policy = RetryPolicy::exponential(3);  // 3 attempts with exponential backoff

// Fixed delay
let policy = RetryPolicy::fixed(5, Duration::from_secs(1));  // 5 attempts, 1 second apart

// Linear backoff
let policy = RetryPolicy::linear(4, Duration::from_millis(500));  // Linear increase

// Custom configuration
let policy = RetryPolicy {
    max_attempts: 3,
    initial_delay: Duration::from_millis(100),
    max_delay: Duration::from_secs(30),
    backoff_multiplier: 2.0,
    jitter: true,  // Add randomization to prevent thundering herd
};
```

### Using Retry Logic

```rust
// Simple retry with policy
let result = with_retry(policy, || async {
    fetch_from_external_api().await
}).await?;

// Using RetryBuilder for fluent configuration
use rustf::error::RetryBuilder;

let data = RetryBuilder::new()
    .max_attempts(3)
    .initial_delay(Duration::from_millis(100))
    .backoff_multiplier(2.0)
    .jitter(true)
    .execute(|| async {
        unstable_network_call().await
    })
    .await?;
```

### Retryable Errors

Only certain errors are automatically retried:

```rust
// These errors are retryable by default:
Error::Network(_)            // Network issues
Error::DatabaseConnection(_) // Connection failures
Error::Timeout(_)           // Timeouts
Error::ExternalService(..)  // External service failures
Error::DatabasePool(_)      // Pool exhaustion

// These are NOT retryable:
Error::Validation(_)        // Input validation
Error::Authentication(_)    // Auth failures
Error::Authorization(_)     // Permission denied
```

### Circuit Breaker Pattern

Prevent cascading failures with circuit breakers:

```rust
use rustf::error::retry::CircuitBreaker;

// Create a circuit breaker
let breaker = CircuitBreaker::new(
    5,                          // Open after 5 failures
    2,                          // Close after 2 successes
    Duration::from_secs(30),    // Timeout before half-open
    3,                          // Max calls in half-open state
);

// Use the circuit breaker
let result = breaker.execute(|| async {
    call_unreliable_service().await
}).await;

// Circuit states:
// - Closed: Normal operation
// - Open: Rejecting calls (after threshold failures)
// - Half-Open: Testing with limited calls
```

## Error Pages and Responses

### HTML Error Pages

RustF provides beautiful, customizable error pages:

```rust
use rustf::error::ErrorPages;

let error_pages = ErrorPages::new(view_engine, config);

// Render error page
let response = error_pages.render_error_page(
    404,                    // Status code
    Some(&error),          // Error object
    Some("req-123"),       // Request ID
)?;

// Custom error templates in views/errors/
// - views/errors/404.html
// - views/errors/500.html
// - views/errors/generic.html
```

### JSON Error Responses

For API endpoints, return structured JSON errors:

```rust
// Automatic based on Accept header
let response = error_pages.create_error_response(
    status_code,
    Some(&error),
    Some(request_id),
    request.header("Accept"),
)?;

// Force JSON response
let json_response = error_pages.create_json_error_response(
    400,
    Some(&error),
    Some("api-request-123"),
)?;

// Response format:
{
    "error": true,
    "status": 400,
    "code": "E_VALIDATION",
    "message": "User-friendly error message",
    "request_id": "api-request-123",
    "timestamp": "2024-01-20T10:30:00Z",
    "details": {  // Only in development mode
        "error_message": "Detailed error",
        "error_type": "Validation Error"
    }
}
```

### Development vs Production

Error responses adapt based on environment:

```rust
// Development mode (debug builds or RUSTF_ENV=development)
- Full error messages with context
- Stack traces included
- Detailed error information
- Source file locations

// Production mode
- Sanitized, user-friendly messages
- No stack traces or sensitive data
- Generic error messages for internal errors
- Request IDs for support correlation
```

## Error Logging

### Structured Logging

RustF provides comprehensive error logging:

```rust
use rustf::error::{ErrorLogger, LogLevel, LogConfig};

// Configure logging
let config = LogConfig {
    level: LogLevel::Info,
    output: LogOutput::Both("/var/log/rustf.log"),
    include_stack_trace: false,  // Only in development
    include_request_context: true,
    max_file_size: Some(10 * 1024 * 1024),  // 10MB
    max_files: Some(5),  // Keep 5 rotated files
};

// Initialize logger
let logger = ErrorLogger::new(config, app_config);

// Log errors with context
logger.log_error(
    LogLevel::Error,
    &error,
    Some(&request),
    Some("req-123"),
    Some(additional_data),
);
```

### Log Levels

```rust
LogLevel::Debug     // Detailed debugging information
LogLevel::Info      // Informational messages
LogLevel::Warn      // Warning conditions
LogLevel::Error     // Error conditions
LogLevel::Critical  // Critical failures requiring immediate attention
```

### Request Context in Logs

Automatically captured request information:

```json
{
    "timestamp": "2024-01-20T10:30:00Z",
    "level": "ERROR",
    "message": "Database connection failed",
    "error_type": "DatabaseConnection",
    "request_id": "req-123",
    "request_context": {
        "method": "POST",
        "uri": "/api/users",
        "client_ip": "192.168.1.100",
        "user_agent": "Mozilla/5.0..."
    },
    "stack_trace": "..."  // Only in development
}
```

### Sensitive Data Protection

Automatic sanitization of sensitive information:

```rust
// These are automatically redacted in logs:
- Passwords in URLs and headers
- API keys and tokens
- Authorization headers
- Cookie headers
- Credit card numbers
- Social security numbers
```

## Health Checks

Built-in health check endpoint:

```rust
use rustf::error::HealthCheck;

let health_check = HealthCheck::new(config);
let result = health_check.check_health().await;

// Health check response:
{
    "status": "healthy",
    "timestamp": "2024-01-20T10:30:00Z",
    "version": "1.0.0",
    "checks": {
        "database": {
            "status": "healthy",
            "message": "Connection successful"
        },
        "memory": {
            "status": "healthy",
            "message": "Memory usage within limits"
        }
    }
}
```

## Best Practices

### 1. Never Use `unwrap()` in Production

```rust
// ❌ Bad - Can panic
let value = some_option.unwrap();

// ✅ Good - Proper error handling
let value = some_option
    .ok_or_else(|| Error::invalid_input("Value is required"))?;
```

### 2. Add Context to Errors

```rust
// ❌ Bad - No context
database_query().await?;

// ✅ Good - With context
database_query().await
    .context("Failed to fetch user permissions")?;
```

### 3. Use Specific Error Types

```rust
// ❌ Bad - Generic error
Error::internal("Something went wrong")

// ✅ Good - Specific error
Error::database_query("Failed to execute SELECT query: connection timeout")
```

### 4. Handle Errors at the Right Level

```rust
// ❌ Bad - Swallowing errors
let _ = send_email().await;

// ✅ Good - Proper handling
if let Err(e) = send_email().await {
    log::warn!("Failed to send email: {}", e);
    // Continue - email is not critical
}
```

### 5. Test Error Conditions

```rust
#[test]
fn test_validation_error() {
    let result = validate_email("invalid");
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert_eq!(error.status_code(), 400);
    assert_eq!(error.error_code(), "E_VALIDATION");
}
```

## Examples

### Complete Request Handler with Error Handling

```rust
async fn create_order(ctx: &mut Context) -> Result<()> {
    // Parse and validate input
    let order_data: OrderData = ctx.json()
        .await
        .context("Failed to parse order data")?;
    
    order_data.validate()
        .map_err(|e| Error::validation(e))?;
    
    // Check authentication
    let user = ctx.get_user()
        .ok_or_else(|| Error::authentication("Login required"))?;
    
    // Check authorization
    if !user.can_create_orders() {
        return Err(Error::authorization("Insufficient permissions"));
    }
    
    // Retry external payment service
    let payment_result = RetryBuilder::new()
        .max_attempts(3)
        .exponential_backoff()
        .execute(|| async {
            payment_service::process(&order_data).await
                .map_err(|e| Error::external_service("payment", e.to_string()))
        })
        .await
        .context("Payment processing failed")?;
    
    // Create order in database
    let order = Order::create(order_data, payment_result)
        .await
        .context("Failed to create order")?;
    
    // Return success response
    ctx.json(json!({
        "success": true,
        "order_id": order.id,
        "status": order.status,
    }))
}
```

### Middleware Error Handling

```rust
pub async fn error_handling_middleware(
    ctx: &mut Context,
    next: Next<'_>,
) -> Result<()> {
    let request_id = uuid::Uuid::new_v4().to_string();
    ctx.set_header("X-Request-ID", &request_id);
    
    match next.run(ctx).await {
        Ok(()) => Ok(()),
        Err(error) => {
            // Log the error
            log_error(&error, Some(&ctx.request), Some(&request_id));
            
            // Create error response based on Accept header
            let accept = ctx.header("Accept");
            if accept.map_or(false, |h| h.contains("application/json")) {
                ctx.json(json!({
                    "error": true,
                    "code": error.error_code(),
                    "message": error.to_string(),
                    "request_id": request_id,
                }))
            } else {
                let error_pages = ctx.error_pages();
                let response = error_pages.render_error_page(
                    error.status_code(),
                    Some(&error),
                    Some(&request_id),
                )?;
                ctx.set_response(response);
                Ok(())
            }
        }
    }
}
```

### Database Operation with Retry

```rust
async fn get_user_with_retry(id: u64) -> Result<User> {
    RetryBuilder::new()
        .max_attempts(3)
        .initial_delay(Duration::from_millis(100))
        .execute(|| async {
            User::find(id)
                .await
                .map_err(|e| Error::database_query(format!("Failed to fetch user {}: {}", id, e)))?
                .ok_or_else(|| Error::model_not_found(format!("User {} not found", id)))
        })
        .await
        .context(format!("Failed to retrieve user {}", id))
}
```

## Configuration

### Environment Variables

```bash
# Logging configuration
RUSTF_LOG_LEVEL=info                    # debug, info, warn, error, critical
RUSTF_LOG_OUTPUT=both:/var/log/rustf.log  # console, file:<path>, both:<path>, none
RUSTF_LOG_STACK_TRACE=false             # Include stack traces
RUSTF_LOG_REQUEST_CONTEXT=true          # Include request context

# Error page configuration
RUSTF_ENV=production                    # development or production
RUSTF_ERROR_VERBOSE=false              # Verbose error messages in production
```

### Configuration File

```toml
[error]
# Error page templates directory
template_dir = "views/errors"

# Default error messages
default_404 = "The page you're looking for could not be found"
default_500 = "An internal error occurred. Please try again later"

[logging]
level = "info"
output = "file:/var/log/rustf/app.log"
max_file_size = 10485760  # 10MB
max_files = 5
include_stack_trace = false
include_request_context = true

[retry]
default_max_attempts = 3
default_initial_delay_ms = 100
default_max_delay_ms = 30000
default_backoff_multiplier = 2.0
```

## Performance Considerations

### Error Creation Cost

- Use static strings for fixed messages: `Error::validation("Invalid email")`
- Defer formatting until needed: `Error::internal(format!(...))` only when necessary
- Consider caching error messages for hot paths

### Retry Overhead

- Use circuit breakers to prevent cascading failures
- Configure reasonable timeouts and max attempts
- Add jitter to prevent thundering herd

### Logging Performance

- Use appropriate log levels (don't log everything as ERROR)
- Configure log rotation to prevent disk space issues
- Consider async logging for high-throughput applications

## Testing

### Testing Error Conditions

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rustf::error::{Error, ErrorContext};
    
    #[test]
    fn test_error_chain() {
        let error = database_operation()
            .context("Failed in service")
            .context("Failed in controller")
            .unwrap_err();
        
        let chain = ErrorChain::new(&error);
        assert_eq!(chain.chain().len(), 3);
    }
    
    #[tokio::test]
    async fn test_retry_logic() {
        let mut attempts = 0;
        
        let result = with_retry(RetryPolicy::fixed(3, Duration::from_millis(10)), || async {
            attempts += 1;
            if attempts < 3 {
                Err(Error::network("Temporary failure"))
            } else {
                Ok("Success")
            }
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(attempts, 3);
    }
}
```

## Migration Guide

### From Simple Errors to RustF Errors

```rust
// Before
fn process() -> Result<Data, Box<dyn std::error::Error>> {
    let data = fetch_data()?;
    Ok(data)
}

// After
fn process() -> rustf::error::Result<Data> {
    let data = fetch_data()
        .map_err(|e| Error::internal(e.to_string()))
        .context("Failed to fetch data")?;
    Ok(data)
}
```

### Adding Retry Logic to Existing Code

```rust
// Before
let result = unreliable_operation().await?;

// After
let result = RetryBuilder::new()
    .max_attempts(3)
    .execute(|| unreliable_operation())
    .await?;
```

## Conclusion

RustF's error system provides a comprehensive, production-ready solution for error handling in web applications. By following the patterns and best practices outlined in this guide, you can build robust, maintainable applications with excellent error handling and observability.

Key takeaways:
- Use specific error types for clarity
- Add context to errors for better debugging
- Implement retry logic for transient failures
- Configure appropriate logging and monitoring
- Never expose sensitive information in error messages
- Test error conditions thoroughly

For more information, see the [API documentation](https://docs.rs/rustf) or the [examples](https://github.com/rustf/examples) repository.