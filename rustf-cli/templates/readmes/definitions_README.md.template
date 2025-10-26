# Definitions Directory

This directory contains definition modules that customize and extend framework behavior.

## What are Definitions?

Definitions allow you to:
- **Replace framework components** with custom implementations (providers)
- **Add template helpers** for use in views
- **Register validators** for data validation
- **Intercept and modify** framework behavior at specific points

## Directory Structure

Each `.rs` file in this directory should export an `install` function that registers its definitions:

```rust
pub fn install(defs: &mut Definitions) {
    // Register your definitions here
}
```

## Types of Definitions

### 1. Providers
Replace framework implementations with custom ones:
- Session storage backends (Redis, Database, etc.)
- Cache providers (Memcached, Redis, etc.)
- File storage (S3, Azure Blob, etc.)
- Email services (SendGrid, SMTP, etc.)

```rust
defs.register_provider(CustomSessionStorageProvider::new());
```

### 2. Template Helpers
Functions available in view templates:

```rust
defs.register_helper("format_price", |args, _ctx| {
    // Format price for display
});
```

### 3. Validators
Reusable validation logic:

```rust
defs.register_validator("phone_number", PhoneValidator);
```

### 4. Interceptors
Modify data at framework execution points:

```rust
defs.register_interceptor("before_model_save", TimestampInterceptor);
```

## Interception Points

The framework provides these standard interception points:

### Model Operations
- `before_model_save` - Before saving a model
- `after_model_save` - After saving a model
- `before_model_delete` - Before deleting a model
- `after_model_delete` - After deleting a model
- `after_model_load` - After loading a model

### Request/Response
- `before_request` - Before processing a request
- `after_request` - After processing a request
- `before_response` - Before sending a response
- `after_response` - After sending a response

### Views
- `before_view_render` - Before rendering a view
- `after_view_render` - After rendering a view
- `view_compile` - During view compilation

### Validation
- `before_validation` - Before running validation
- `after_validation` - After validation completes

### Session/Cache
- `before_session_save` - Before saving session data
- `after_session_load` - After loading session data
- `before_cache_set` - Before setting cache value
- `after_cache_get` - After getting cache value

## Examples

### Custom Session Storage

```rust
use rustf::definitions::*;
use rustf::session::{SessionStorage, Session};

struct CustomSessionStorage {
    // Your storage implementation
}

#[async_trait]
impl SessionStorage for CustomSessionStorage {
    async fn load(&self, id: &str) -> rustf::Result<Option<Session>> {
        // Load session from your backend
    }
    
    async fn save(&self, session: &Session) -> rustf::Result<()> {
        // Save session to your backend
    }
    
    async fn delete(&self, id: &str) -> rustf::Result<()> {
        // Delete session from your backend
    }
}

pub fn install(defs: &mut Definitions) {
    defs.register_provider(CustomSessionStorageProvider {
        storage: Arc::new(CustomSessionStorage { /* ... */ })
    });
}
```

### Custom Template Helper

```rust
pub fn install(defs: &mut Definitions) {
    // Format currency values
    defs.register_helper_fn("money", |args, _ctx| {
        if let Some(Value::Number(n)) = args.first() {
            let amount = n.as_f64().unwrap_or(0.0);
            Ok(Value::String(format!("${:.2}", amount)))
        } else {
            Ok(Value::String("$0.00".to_string()))
        }
    });
}
```

### Model Timestamp Interceptor

```rust
use chrono::Utc;

pub fn install(defs: &mut Definitions) {
    defs.register_json_interceptor("before_model_save", |mut data| {
        if let Value::Object(ref mut map) = data {
            map.insert("updated_at".to_string(), 
                      Value::String(Utc::now().to_rfc3339()));
        }
        Ok(data)
    });
}
```

## Best Practices

1. **Keep definitions modular** - One concern per file
2. **Use descriptive names** - Make it clear what each definition does
3. **Document your definitions** - Add comments explaining purpose and usage
4. **Test your definitions** - Ensure they work correctly before deployment
5. **Handle errors gracefully** - Don't panic in definition code

## Auto-Discovery

Definitions are automatically discovered and loaded when using:

```rust
app.definitions_from(auto_definitions!())
```

This macro scans this directory and calls the `install` function from each module.