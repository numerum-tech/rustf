# RustF Definitions System Documentation

## Overview

The RustF Definitions System provides a powerful, convention-based framework for extending RustF with custom functionality. Inspired by modern web frameworks like Rails and Laravel, it enables developers to customize framework behavior through simple, discoverable patterns without modifying core code.

## Core Philosophy

The definitions system follows these principles:

1. **Convention Over Configuration** - Place files in expected locations, and they're automatically discovered
2. **Type Safety** - All extensions are type-checked at compile time
3. **Zero Boilerplate** - No registration code needed with auto-discovery
4. **Composable** - Mix and match different definition types as needed
5. **Framework Integration** - Definitions integrate seamlessly with RustF's core systems

## What Are Definitions?

Definitions are modular extensions that customize framework behavior:

- **Helpers** - Custom template functions for views
- **Validators** - Data validation logic for forms and APIs
- **Session Storage** - Custom session backend implementations

## Quick Start

### 1. Create a Definitions Module

Create a file in `src/definitions/` directory:

```rust
// src/definitions/app.rs
use rustf::definitions::{Definitions, Helper, Validator};
use rustf::error::Result;
use serde_json::Value;

/// Install function called by auto-discovery
/// This function MUST be named 'install' and have this exact signature
pub fn install(defs: &mut Definitions) {
    // Register a custom helper
    defs.register_helper("format_money", FormatMoneyHelper);
    
    // Register a custom validator
    defs.register_validator("email", EmailValidator);
}

// Helper implementation
struct FormatMoneyHelper;

impl Helper for FormatMoneyHelper {
    fn call(&self, args: &[Value], _context: Option<&Value>) -> Result<Value> {
        if let Some(Value::Number(n)) = args.first() {
            if let Some(amount) = n.as_f64() {
                return Ok(Value::String(format!("${:.2}", amount)));
            }
        }
        Ok(Value::Null)
    }
    
    fn name(&self) -> &str { "format_money" }
    fn description(&self) -> &str { "Formats numbers as currency" }
}

// Validator implementation
struct EmailValidator;

impl Validator for EmailValidator {
    fn validate(&self, value: &Value, _options: Option<&Value>) -> Result<()> {
        if let Some(email) = value.as_str() {
            if !email.contains('@') || !email.contains('.') {
                return Err(rustf::error::Error::validation("Invalid email format"));
            }
        }
        Ok(())
    }
    
    fn name(&self) -> &str { "email" }
    fn description(&self) -> &str { "Validates email addresses" }
}
```

### 2. Register Definitions in Your App

Use auto-discovery in `main.rs`:

```rust
use rustf::prelude::*;

#[rustf::auto_discover]
#[tokio::main]
async fn main() -> rustf::Result<()> {
    let app = RustF::new()
        .definitions_from(auto_definitions!())  // Auto-discovers all definitions
        .controllers(auto_controllers!());
    
    app.start().await
}
```

### 3. Use Your Definitions

In views (for helpers):
```html
<!-- Price: {{ price | format_money }} -->
<!-- Outputs: Price: $99.99 -->
```

In controllers (for validators):
```rust
async fn create_user(ctx: Context) -> Result<Response> {
    let email = ctx.param("email")?;
    
    // Get validators from global definitions
    let definitions = rustf::definitions::get().await;
    let defs = definitions.read().await;
    let validator = defs.validators.get("email")?;
    validator.validate(&json!(email), None)?;
    
    // Proceed with user creation...
    ctx.json(json!({"status": "user created"}))
}
```

## Component Types

### Template Helpers

Helpers are functions that transform data in views. They're perfect for formatting, calculations, and generating HTML snippets.

#### Creating a Helper

```rust
use rustf::definitions::Helper;
use rustf::error::Result;
use serde_json::Value;

struct DateFormatHelper;

impl Helper for DateFormatHelper {
    fn call(&self, args: &[Value], _context: Option<&Value>) -> Result<Value> {
        // args[0] = date string/timestamp
        // args[1] = format string (optional)
        
        if let Some(date_value) = args.first() {
            let format = args.get(1)
                .and_then(|v| v.as_str())
                .unwrap_or("%Y-%m-%d");
            
            // Format the date...
            let formatted = format_date(date_value, format)?;
            return Ok(Value::String(formatted));
        }
        
        Ok(Value::Null)
    }
    
    fn name(&self) -> &str { "date_format" }
    
    fn description(&self) -> &str { 
        "Formats dates according to strftime patterns" 
    }
}
```

#### Using Helpers in Views

```html
<!-- TotalJS syntax -->
@{date_format(user.created_at, "%B %d, %Y")}

<!-- Tera syntax -->
{{ user.created_at | date_format("%B %d, %Y") }}

<!-- With default format -->
{{ timestamp | date_format }}
```

#### Built-in Helper Registry

RustF provides a `HelperRegistry` with several built-in helpers:

```rust
pub struct HelperRegistry {
    helpers: HashMap<String, Box<dyn Helper>>,
}

impl HelperRegistry {
    pub fn new() -> Self {
        let mut registry = Self { helpers: HashMap::new() };
        
        // Register built-in helpers
        registry.register("format_currency", FormatCurrencyHelper);
        registry.register("truncate", TruncateHelper);
        registry.register("pluralize", PluralizeHelper);
        registry.register("time_ago", TimeAgoHelper);
        registry.register("format_date", FormatDateHelper);
        registry.register("url_encode", UrlEncodeHelper);
        registry.register("url_decode", UrlDecodeHelper);
        registry.register("json", JsonHelper);
        registry.register("default", DefaultHelper);
        
        registry
    }
}
```

### Validators

Validators ensure data integrity before processing. They're essential for form validation, API input validation, and business rule enforcement.

#### Creating a Validator

```rust
use rustf::definitions::Validator;
use rustf::error::{Result, Error};
use serde_json::Value;

struct PasswordStrengthValidator;

impl Validator for PasswordStrengthValidator {
    fn validate(&self, value: &Value, options: Option<&Value>) -> Result<()> {
        let min_length = options
            .and_then(|o| o.get("min_length"))
            .and_then(|v| v.as_u64())
            .unwrap_or(8) as usize;
        
        if let Some(password) = value.as_str() {
            if password.len() < min_length {
                return Err(Error::validation(
                    format!("Password must be at least {} characters", min_length)
                ));
            }
            
            let has_upper = password.chars().any(|c| c.is_uppercase());
            let has_lower = password.chars().any(|c| c.is_lowercase());
            let has_digit = password.chars().any(|c| c.is_numeric());
            let has_special = password.chars().any(|c| !c.is_alphanumeric());
            
            if !(has_upper && has_lower && has_digit && has_special) {
                return Err(Error::validation(
                    "Password must contain uppercase, lowercase, digit, and special character"
                ));
            }
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str { "password_strength" }
    
    fn description(&self) -> &str { 
        "Validates password meets security requirements" 
    }
}
```

#### Using Validators

```rust
async fn update_password(ctx: Context) -> Result<Response> {
    let new_password = ctx.param("new_password")?;
    
    // Get validator from global definitions
    let definitions = rustf::definitions::get().await;
    let defs = definitions.read().await;
    let validator = defs.validators.get("password_strength")?;
    
    // Validate with options
    let options = json!({
        "min_length": 12,
        "require_special": true
    });
    
    validator.validate(&json!(new_password), Some(&options))?;
    
    // Password is valid, proceed with update
    update_user_password(&new_password).await?;
    
    ctx.json(json!({"status": "password updated"}))
}
```

### Custom Session Storage

The definitions system enables custom session storage backends through a factory pattern. This is the modern, recommended approach for implementing database or custom storage backends.

#### Creating Custom Session Storage

Create `src/definitions/session_storage.rs`:

```rust
use rustf::definitions::Definitions;
use rustf::session::{SessionStorage, SessionData, StorageStats};
use rustf::config::SessionConfig;
use rustf::error::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;

/// Install function called by auto-discovery
/// This registers our custom session storage factory with the definitions system
pub fn install(defs: &mut Definitions) {
    log::info!("Installing custom session storage from definitions");
    defs.set_session_storage_factory(create_session_storage);
}

/// Factory function that creates our custom session storage
/// This is called by the framework when initializing sessions
fn create_session_storage(config: &SessionConfig) -> Result<Arc<dyn SessionStorage>> {
    log::info!("Creating custom PostgreSQL session storage");
    
    // You can access configuration here
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/myapp".to_string());
    
    // Create and return your custom storage
    let storage = PostgresSessionStorage::new(&database_url)?;
    Ok(Arc::new(storage))
}

/// PostgreSQL session storage implementation
pub struct PostgresSessionStorage {
    pool: PgPool,
}

impl PostgresSessionStorage {
    pub fn new(database_url: &str) -> Result<Self> {
        // Note: For simplicity, using block_on here
        // In production, consider initializing the pool elsewhere
        let pool = futures::executor::block_on(
            PgPool::connect(database_url)
        ).map_err(|e| rustf::error::Error::internal(
            format!("Failed to connect to database: {}", e)
        ))?;
        
        // Create table if needed
        futures::executor::block_on(async {
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS sessions (
                    id VARCHAR(64) PRIMARY KEY,
                    data JSONB NOT NULL,
                    expires_at TIMESTAMP NOT NULL,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )"
            ).execute(&pool).await
        }).map_err(|e| rustf::error::Error::internal(
            format!("Failed to create sessions table: {}", e)
        ))?;
        
        Ok(Self { pool })
    }
}

#[async_trait]
impl SessionStorage for PostgresSessionStorage {
    async fn get(&self, session_id: &str) -> Result<Option<SessionData>> {
        let row = sqlx::query!(
            "SELECT data FROM sessions 
             WHERE id = $1 AND expires_at > NOW()",
            session_id
        )
        .fetch_optional(&self.pool)
        .await?;
        
        match row {
            Some(row) => {
                let mut data: SessionData = serde_json::from_value(row.data)?;
                data.touch(); // Update last accessed
                
                // Update timestamp in database
                sqlx::query!(
                    "UPDATE sessions SET updated_at = NOW() WHERE id = $1",
                    session_id
                )
                .execute(&self.pool)
                .await?;
                
                Ok(Some(data))
            }
            None => Ok(None)
        }
    }
    
    async fn set(
        &self, 
        session_id: &str, 
        data: &SessionData, 
        ttl: Duration
    ) -> Result<()> {
        let expires_at = chrono::Utc::now() + 
            chrono::Duration::seconds(ttl.as_secs() as i64);
        let json_data = serde_json::to_value(data)?;
        
        sqlx::query!(
            "INSERT INTO sessions (id, data, expires_at, updated_at)
             VALUES ($1, $2, $3, NOW())
             ON CONFLICT (id) DO UPDATE
             SET data = $2, expires_at = $3, updated_at = NOW()",
            session_id,
            json_data,
            expires_at
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    async fn delete(&self, session_id: &str) -> Result<()> {
        sqlx::query!(
            "DELETE FROM sessions WHERE id = $1",
            session_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    
    async fn exists(&self, session_id: &str) -> Result<bool> {
        let exists = sqlx::query!(
            "SELECT 1 as exists FROM sessions 
             WHERE id = $1 AND expires_at > NOW()",
            session_id
        )
        .fetch_optional(&self.pool)
        .await?
        .is_some();
        
        Ok(exists)
    }
    
    async fn cleanup_expired(&self) -> Result<usize> {
        let result = sqlx::query!(
            "DELETE FROM sessions WHERE expires_at <= NOW()"
        )
        .execute(&self.pool)
        .await?;
        
        Ok(result.rows_affected() as usize)
    }
    
    fn backend_name(&self) -> &'static str {
        "postgresql"
    }
    
    async fn stats(&self) -> Result<StorageStats> {
        let total: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM sessions"
        )
        .fetch_one(&self.pool)
        .await?;
        
        let active: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM sessions WHERE expires_at > NOW()"
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(StorageStats {
            total_sessions: total as usize,
            active_sessions: active as usize,
            expired_sessions: (total - active) as usize,
            backend_metrics: HashMap::new(),
        })
    }
}
```

#### Integration with Auto-Discovery

The session storage factory is automatically discovered and used when you include it in your definitions:

```rust
// main.rs
#[rustf::auto_discover]
#[tokio::main]
async fn main() -> rustf::Result<()> {
    let app = RustF::new()
        .definitions_from(auto_definitions!())  // Discovers session_storage.rs
        .controllers(auto_controllers!());
    
    // Your custom session storage is now active!
    app.start().await
}
```

## Auto-Discovery System

### How Auto-Discovery Works

The `auto_definitions!()` macro scans your `src/definitions/` directory and automatically:

1. Finds all `.rs` files with an `install` function
2. Generates the necessary module declarations
3. Calls each `install` function to register definitions
4. Makes them available throughout your application

**IMPORTANT**: Every file in `src/definitions/` must export a public `install` function with this exact signature:

```rust
pub fn install(defs: &mut Definitions) {
    // Register your definitions here
}
```

This function is where you register all your helpers, validators, or storage factories. The auto-discovery system will call this function during application initialization.

### File Convention

Place definition files in `src/definitions/`:

```
src/
  definitions/
    app.rs           # General helpers and validators
    helpers.rs       # Template helpers only
    validators.rs    # Validators only
    session_storage.rs  # Custom session storage factory
```

Each file should export an `install` function:

```rust
pub fn install(defs: &mut Definitions) {
    // Register your definitions here
}
```

### Manual Registration

If you prefer explicit control, register definitions manually:

```rust
// main.rs
use my_app::definitions;

#[tokio::main]
async fn main() -> rustf::Result<()> {
    let app = RustF::new()
        .definitions_from(definitions::app::install)
        .definitions_from(definitions::auth::install)
        .controllers(auto_controllers!());
    
    app.start().await
}
```

## Advanced Topics

### Accessing Definitions in Code

#### In Controllers

```rust
async fn my_controller(ctx: Context) -> Result<Response> {
    // Get definitions from global registry
    let definitions = rustf::definitions::get().await;
    let defs = definitions.read().await;
    
    // Access helpers
    let helper = defs.helpers.get("format_money")?;
    let formatted = helper.call(&[json!(99.99)], None)?;
    
    // Access validators  
    let validator = defs.validators.get("email")?;
    validator.validate(&json!("user@example.com"), None)?;
    
    ctx.json(json!({"formatted": formatted}))
}
```

#### In Middleware

```rust
use rustf::middleware::{InboundMiddleware, InboundAction};
use rustf::context::Context;
use rustf::error::Result;

pub struct ValidationMiddleware;

impl InboundMiddleware for ValidationMiddleware {
    fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
        // Get email from request
        let email = ctx.param("email")?;
        
        // Get validator from global definitions
        // Note: Using block_on since middleware is synchronous
        let definitions = futures::executor::block_on(
            rustf::definitions::get()
        );
        let defs = futures::executor::block_on(definitions.read());
        let validator = defs.validators.get("email")?;
        validator.validate(&json!(email), None)?;
        
        Ok(InboundAction::Continue)
    }
}
```

### Testing Definitions

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rustf::definitions::Definitions;
    
    #[test]
    fn test_format_money_helper() {
        let mut defs = Definitions::new();
        super::install(&mut defs);
        
        let helper = defs.helpers.get("format_money").unwrap();
        let result = helper.call(&[json!(42.50)], None).unwrap();
        
        assert_eq!(result, json!("$42.50"));
    }
    
    #[test]
    fn test_email_validator() {
        let mut defs = Definitions::new();
        super::install(&mut defs);
        
        let validator = defs.validators.get("email").unwrap();
        
        // Valid email
        assert!(validator.validate(&json!("user@example.com"), None).is_ok());
        
        // Invalid email
        assert!(validator.validate(&json!("invalid"), None).is_err());
    }
}
```

### Async Helpers and Validators

For async operations, wrap them appropriately:

```rust
struct AsyncDataHelper;

impl Helper for AsyncDataHelper {
    fn call(&self, args: &[Value], _context: Option<&Value>) -> Result<Value> {
        // For async operations, you might need to use block_on
        // or restructure to handle async differently
        let data = futures::executor::block_on(async {
            fetch_async_data().await
        })?;
        
        Ok(json!(data))
    }
    
    fn name(&self) -> &str { "async_data" }
    fn description(&self) -> &str { "Fetches async data" }
}
```

## Best Practices

### 1. Keep Definitions Focused

Each definition should have a single, clear purpose:

```rust
// Good: Specific, reusable
struct FormatCurrencyHelper;
struct ValidateEmailValidator;

// Less ideal: Too generic
struct GeneralHelper;
struct DoEverythingValidator;
```

### 2. Use Descriptive Names

Names should clearly indicate function:

```rust
// Good
"format_date"
"validate_phone"
"sanitize_html"

// Less clear
"process"
"check"
"helper1"
```

### 3. Handle Errors Gracefully

Always provide helpful error messages:

```rust
impl Validator for PhoneValidator {
    fn validate(&self, value: &Value, _options: Option<&Value>) -> Result<()> {
        if let Some(phone) = value.as_str() {
            if !is_valid_phone(phone) {
                return Err(Error::validation(
                    format!("'{}' is not a valid phone number. Expected format: +1-555-555-5555", phone)
                ));
            }
        } else {
            return Err(Error::validation(
                "Phone number must be a string"
            ));
        }
        Ok(())
    }
}
```

### 4. Document Your Definitions

Always implement the `description()` method:

```rust
impl Helper for MyHelper {
    fn name(&self) -> &str { "my_helper" }
    
    fn description(&self) -> &str {
        "Formats user data for display. \
         Usage: {{ user | my_helper }} or {{ my_helper(user, 'option') }}"
    }
}
```

### 5. Make Definitions Testable

Design definitions to be easily testable:

```rust
// Separate business logic from the definition
fn format_phone_number(phone: &str) -> String {
    // Formatting logic here
}

struct FormatPhoneHelper;

impl Helper for FormatPhoneHelper {
    fn call(&self, args: &[Value], _context: Option<&Value>) -> Result<Value> {
        if let Some(Value::String(phone)) = args.first() {
            Ok(Value::String(format_phone_number(phone)))
        } else {
            Ok(Value::Null)
        }
    }
    // ...
}

#[test]
fn test_phone_formatting() {
    assert_eq!(format_phone_number("5555555555"), "+1 (555) 555-5555");
}
```

## Integration with RustF Systems

### View System Integration

Helpers are automatically available in both TotalJS and Tera templates:

```rust
// Registered in definitions
defs.register_helper("user_avatar", UserAvatarHelper);

// Available in TotalJS views
// @{user_avatar(user.email, 200)}

// Available in Tera templates
// {{ user.email | user_avatar(200) }}
```

### Validation System Integration

Validators work with RustF's error handling:

```rust
async fn api_endpoint(ctx: Context) -> Result<Response> {
    let data = ctx.json_body()?;
    
    // Get validators from global definitions
    let definitions = rustf::definitions::get().await;
    let defs = definitions.read().await;
    
    // Validate multiple fields
    defs.validators.get("email")?.validate(&data["email"], None)?;
    defs.validators.get("password")?.validate(&data["password"], None)?;
    defs.validators.get("age")?.validate(&data["age"], Some(&json!({"min": 18})))?;
    
    // All validations passed
    process_data(data).await?;
    
    ctx.json(json!({"status": "success"}))
}
```

### Session System Integration

Custom session storage integrates seamlessly with RustF's session middleware:

```rust
// Your custom storage is automatically used when defined
// src/definitions/session_storage.rs exists and has an install() function

// In controllers, sessions work normally
async fn login(ctx: Context) -> Result<Response> {
    ctx.session_set("user_id", user.id)?;
    ctx.session_set("username", user.username)?;
    
    // Your custom storage handles persistence
    ctx.json(json!({"status": "logged in"}))
}
```

## Troubleshooting

### Definitions Not Found

If your definitions aren't being discovered:

1. Ensure files are in `src/definitions/`
2. Check that files have an `install` function
3. Verify you're using `auto_definitions!()` or manual registration
4. Make sure the function signature matches: `pub fn install(defs: &mut Definitions)`

### Helper Not Available in Views

1. Confirm the helper is registered in `install()`
2. Check the helper name matches exactly (case-sensitive)
3. Verify the view engine is configured correctly
4. Check helper's `name()` method returns the expected string

### Validator Errors Not Showing

1. Ensure you're returning proper `Error::validation()` errors
2. Check that error messages are descriptive
3. Verify the validator is registered with the correct name
4. Test the validator independently to ensure it works

### Session Storage Not Being Used

1. Verify `src/definitions/session_storage.rs` exists
2. Check the file has an `install(defs: &mut Definitions)` function
3. Ensure `install()` calls `defs.set_session_storage_factory(your_factory_fn)`
4. Verify your factory function signature matches: `fn(&SessionConfig) -> Result<Arc<dyn SessionStorage>>`
5. Confirm auto-discovery is enabled with `auto_definitions!()` in main.rs
6. Check that session middleware is enabled in configuration
7. Look for log messages: "Using custom session storage from definitions"

**Important**: The `SessionConfig` parameter is from `rustf::config::SessionConfig`, not `rustf::session::manager::SessionConfig`

## Examples Repository

For complete working examples of all definition types, see the RustF examples repository:

- **Basic Definitions**: Simple helpers and validators
- **Advanced Helpers**: Complex formatting and HTML generation
- **Custom Validators**: Business rule validation
- **Database Session Storage**: PostgreSQL, MySQL implementations
- **Redis Session Storage**: High-performance session storage
- **Multi-tenant Definitions**: Per-tenant customization

## Summary

The RustF Definitions System provides a powerful, type-safe way to extend the framework with custom functionality. By following conventions and using auto-discovery, you can build modular, reusable extensions that integrate seamlessly with RustF's core systems.

Key takeaways:

1. **Use conventions** - Place files in `src/definitions/` for auto-discovery
2. **Implement traits** - `Helper`, `Validator`, and `SessionStorage` provide the contracts
3. **Leverage auto-discovery** - Let the framework handle registration
4. **Keep it simple** - Each definition should do one thing well
5. **Test thoroughly** - Definitions are easy to unit test

With definitions, you can build powerful, customized applications while keeping your code organized, maintainable, and reusable across projects.