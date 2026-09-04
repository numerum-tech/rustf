# Session Access Guide: Proper Way to Access Session Values in Controllers

## Overview

This document analyzes the session-related functions in `context.rs` and provides guidance on the proper way to access session values in controllers.

## Session-Related Functions in Context

### 1. Session State Management

#### `session()` - Get Session Reference (Optional)
```rust
pub fn session(&self) -> Option<&Session>
```
- Returns `Option<&Session>` - safe for checking if session exists
- Returns `None` if no session is available
- **Use when**: You want to check for session existence without errors

#### `has_session()` - Check Session Existence
```rust
pub fn has_session(&self) -> bool
```
- Returns `bool` indicating if session exists
- **Use when**: Simple boolean check is sufficient

#### `require_session()` - Require Session (Error if Missing)
```rust
pub fn require_session(&self) -> Result<&Session>
```
- Returns `Result<&Session>` - errors if session doesn't exist
- **Use when**: Session is mandatory for the operation

#### `require_auth()` - Require Authenticated Session
```rust
pub fn require_auth(&self) -> Result<&Session>
```
- Returns `Result<&Session>` - errors if session doesn't exist or user is not authenticated
- Checks `session.is_authenticated()`
- **Use when**: Operation requires authenticated user

### 2. Session Data Access Methods

#### `session_set()` - Set Session Value
```rust
pub fn session_set<T: serde::Serialize>(&self, key: &str, value: T) -> Result<()>
```
- Stores any serializable value in session
- Requires session to exist (uses `require_session()` internally)
- Returns `Result<()>` - errors if session doesn't exist
- **Use when**: Storing user data, preferences, cart items, etc.

#### `session_get()` - Get Session Value
```rust
pub fn session_get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T>
```
- Retrieves typed value from session
- Returns `Option<T>` - `None` if key doesn't exist or session unavailable
- **Use when**: Reading session data with type safety

#### `session_remove()` - Remove Session Value
```rust
pub fn session_remove(&self, key: &str) -> Option<Value>
```
- Removes a specific key from session
- Returns `Option<Value>` - the removed value if it existed
- **Use when**: Cleaning up specific session data

#### `session_clear()` - Clear All Session Data
```rust
pub fn session_clear(&self)
```
- Removes all session data and flash messages
- Keeps session ID intact
- **Use when**: Logging out user but maintaining session tracking

#### `session_flush()` - Alias for `session_clear()`
```rust
pub fn session_flush(&self)
```
- Laravel-style compatibility method
- Same as `session_clear()`

#### `session_destroy()` - Destroy Session
```rust
pub fn session_destroy(&self)
```
- Currently same as `session_clear()` (clears data locally)
- **Note**: Complete destruction including storage removal handled by SessionStore

### 3. Authentication Helpers

#### `login()` - Login User
```rust
pub fn login(&self, user_id: i64) -> Result<()>
```
- Sets user ID in session
- Sets privilege level to 1
- Marks session for rotation (security)
- **Use when**: User successfully authenticates

#### `logout()` - Logout User
```rust
pub fn logout(&self) -> Result<()>
```
- Clears all session data
- **Use when**: User logs out

### 4. Flash Message Helpers

#### `flash()` - Set Flash Message
```rust
pub fn flash(&self, key: &str, value: impl serde::Serialize) -> Result<()>
```
- Stores flash message (consumed on next read)
- **Use when**: Storing temporary messages that survive redirects

#### `get_flash()` - Get Flash Message
```rust
pub fn get_flash(&self, key: &str) -> Option<Value>
```
- Retrieves and consumes flash message
- **Use when**: Displaying flash messages after redirect

#### `get_all_flash()` - Get All Flash Messages
```rust
pub fn get_all_flash(&self) -> HashMap<String, Value>
```
- Retrieves and consumes all flash messages
- **Use when**: Displaying all flash messages at once

#### Convenience Methods:
- `flash_success(message)` - Set success flash message
- `flash_error(message)` - Set error flash message
- `flash_info(message)` - Set info flash message
- `flash_warning(message)` - Set warning flash message
- `flash_clear()` - Clear all flash messages
- `flash_clear_key(key)` - Clear specific flash message

## Proper Usage Patterns in Controllers

### Pattern 1: Optional Session Access (Recommended for Public Endpoints)

```rust
async fn public_handler(ctx: &mut Context) -> Result<()> {
    // Check if session exists before accessing
    if let Some(user_id) = ctx.session_get::<i64>("user_id") {
        // User is logged in
        ctx.repository_set("is_logged_in", true);
        ctx.repository_set("user_id", user_id);
    } else {
        // User is not logged in
        ctx.repository_set("is_logged_in", false);
    }
    
    ctx.view("public_page", json!({}))
}
```

### Pattern 2: Required Session (Recommended for Protected Endpoints)

```rust
async fn protected_handler(ctx: &mut Context) -> Result<()> {
    // Require session - returns error if missing
    let session = ctx.require_session()?;
    
    // Access session data with type safety
    let user_id: i64 = ctx.session_get("user_id")
        .ok_or_else(|| Error::InvalidInput("User ID not found in session".to_string()))?;
    
    // Use user_id for business logic
    // ...
    
    ctx.json(json!({"user_id": user_id}))
}
```

### Pattern 3: Authenticated Session (Recommended for User-Specific Operations)

```rust
async fn user_profile(ctx: &mut Context) -> Result<()> {
    // Require authenticated session
    let _session = ctx.require_auth()?;
    
    // Get user data from session
    let user_id: i64 = ctx.session_get("user_id")
        .ok_or_else(|| Error::InvalidInput("User not authenticated".to_string()))?;
    
    // Fetch user profile from database
    // ...
    
    ctx.json(json!({"user_id": user_id}))
}
```

### Pattern 4: Login Flow

```rust
async fn login_handler(ctx: &mut Context) -> Result<()> {
    let email = ctx.body_str("email")?;
    let password = ctx.body_str("password")?;
    
    // Authenticate user (check database, verify password, etc.)
    let user = authenticate_user(&email, &password).await?;
    
    // Login user - sets user_id and marks session for rotation
    ctx.login(user.id)?;
    
    // Store additional user data in session
    ctx.session_set("username", &user.username)?;
    ctx.session_set("email", &user.email)?;
    ctx.session_set("role", &user.role)?;
    
    // Set flash message
    ctx.flash_success("Login successful")?;
    
    ctx.redirect("/dashboard")
}
```

### Pattern 5: Logout Flow

```rust
async fn logout_handler(ctx: &mut Context) -> Result<()> {
    // Clear all session data
    ctx.logout()?;
    
    // Or use session_clear() for same effect
    // ctx.session_clear();
    
    // Set flash message
    ctx.flash_info("You have been logged out")?;
    
    ctx.redirect("/login")
}
```

### Pattern 6: Session Data Updates

```rust
async fn update_preferences(ctx: &mut Context) -> Result<()> {
    // Require authenticated session
    ctx.require_auth()?;
    
    // Get current preferences
    let mut preferences: HashMap<String, Value> = ctx.session_get("preferences")
        .unwrap_or_else(|| HashMap::new());
    
    // Update preferences from request
    let theme = ctx.body_str("theme")?;
    preferences.insert("theme".to_string(), json!(theme));
    
    // Save back to session
    ctx.session_set("preferences", &preferences)?;
    
    ctx.json(json!({"success": true}))
}
```

### Pattern 7: Flash Messages with Redirects

```rust
async fn create_item(ctx: &mut Context) -> Result<()> {
    ctx.require_auth()?;
    
    // Process form data
    let name = ctx.body_str("name")?;
    
    // Create item in database
    match create_item_in_db(&name).await {
        Ok(item) => {
            // Success flash message
            ctx.flash_success(format!("Item '{}' created successfully", name))?;
            ctx.redirect("/items")
        }
        Err(e) => {
            // Error flash message
            ctx.flash_error(format!("Failed to create item: {}", e))?;
            ctx.redirect("/items/new")
        }
    }
}

async fn items_list(ctx: &mut Context) -> Result<()> {
    // Get flash messages (they are consumed after this call)
    let flash_messages = ctx.get_all_flash();
    
    // Pass to view
    ctx.view("items/list", json!({
        "flash": flash_messages
    }))
}
```

## Best Practices

### 1. Use Type-Safe Access
```rust
// ✅ Good - Type-safe with Option handling
let user_id: Option<i64> = ctx.session_get("user_id");

// ❌ Bad - Direct access without type safety
let session = ctx.session()?;
let user_id = session.data.get("user_id"); // Returns Value, not typed
```

### 2. Handle Missing Sessions Gracefully
```rust
// ✅ Good - Handles missing session
if let Some(user_id) = ctx.session_get::<i64>("user_id") {
    // Use user_id
}

// ✅ Also Good - Explicit error handling
let user_id: i64 = ctx.session_get("user_id")
    .ok_or_else(|| Error::InvalidInput("User not logged in".to_string()))?;
```

### 3. Use Appropriate Session Requirement Level
```rust
// ✅ For public endpoints - optional session
if ctx.has_session() {
    // Handle logged-in users
}

// ✅ For protected endpoints - require session
ctx.require_session()?;

// ✅ For authenticated operations - require auth
ctx.require_auth()?;
```

### 4. Store Serializable Data
```rust
// ✅ Good - Store simple types
ctx.session_set("user_id", 123)?;
ctx.session_set("username", "john")?;

// ✅ Good - Store complex types (must implement Serialize)
ctx.session_set("preferences", json!({
    "theme": "dark",
    "language": "en"
}))?;

// ✅ Good - Store custom structs (must derive Serialize)
#[derive(Serialize, Deserialize)]
struct UserPrefs {
    theme: String,
    language: String,
}
let prefs = UserPrefs { theme: "dark".to_string(), language: "en".to_string() };
ctx.session_set("preferences", &prefs)?;
```

### 5. Use Flash Messages for User Feedback
```rust
// ✅ Good - Flash messages survive redirects
ctx.flash_success("Operation completed")?;
ctx.redirect("/success");

// ❌ Bad - Repository data doesn't survive redirects
ctx.repository_set("message", "Operation completed");
ctx.redirect("/success"); // Message lost!
```

## Common Mistakes to Avoid

### Mistake 1: Not Handling Missing Sessions
```rust
// ❌ Bad - Will panic if session doesn't exist
let user_id: i64 = ctx.session_get("user_id").unwrap();

// ✅ Good - Handle Option properly
let user_id: Option<i64> = ctx.session_get("user_id");
```

### Mistake 2: Using Wrong Session Requirement
```rust
// ❌ Bad - Using require_auth() when session existence is enough
ctx.require_auth()?; // Fails if user not authenticated
let cart: Option<Value> = ctx.session_get("cart"); // Cart doesn't need auth

// ✅ Good - Use appropriate requirement level
ctx.require_session()?; // Only requires session existence
let cart: Option<Value> = ctx.session_get("cart");
```

### Mistake 3: Not Using Type Safety
```rust
// ❌ Bad - Loses type information
let session = ctx.session()?;
let user_id = session.data.get("user_id"); // Returns Value

// ✅ Good - Type-safe access
let user_id: Option<i64> = ctx.session_get("user_id");
```

### Mistake 4: Confusing Session and Repository
```rust
// ❌ Bad - Repository data doesn't persist across requests
ctx.repository_set("user_id", 123);
// Next request: user_id is gone!

// ✅ Good - Session data persists
ctx.session_set("user_id", 123)?;
// Next request: user_id is still there
```

## Summary

### Quick Reference

| Use Case | Method | Returns | Error Handling |
|----------|--------|---------|----------------|
| Check if session exists | `has_session()` | `bool` | None needed |
| Get session (optional) | `session()` | `Option<&Session>` | Handle `None` |
| Require session | `require_session()` | `Result<&Session>` | Use `?` operator |
| Require authenticated | `require_auth()` | `Result<&Session>` | Use `?` operator |
| Set session value | `session_set(key, value)` | `Result<()>` | Use `?` operator |
| Get session value | `session_get::<T>(key)` | `Option<T>` | Handle `None` |
| Remove session value | `session_remove(key)` | `Option<Value>` | Handle `None` |
| Clear all session data | `session_clear()` | `()` | None needed |
| Login user | `login(user_id)` | `Result<()>` | Use `?` operator |
| Logout user | `logout()` | `Result<()>` | Use `?` operator |
| Set flash message | `flash(key, value)` | `Result<()>` | Use `?` operator |
| Get flash message | `get_flash(key)` | `Option<Value>` | Handle `None` |

### Recommended Patterns

1. **Public endpoints**: Use `has_session()` or `session_get()` with `Option` handling
2. **Protected endpoints**: Use `require_session()` for session requirement
3. **Authenticated endpoints**: Use `require_auth()` for authentication requirement
4. **Type safety**: Always use `session_get::<T>()` for typed access
5. **User feedback**: Use flash messages for messages that survive redirects
6. **Data persistence**: Use `session_set()` for data that should persist across requests


