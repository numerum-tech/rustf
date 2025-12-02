# Context API Reference

The `Context` struct provides access to request data, response methods, sessions, and more. All route handlers receive a `&mut Context` parameter.

## Overview

```rust
pub struct Context {
    pub req: Request,           // HTTP request
    pub res: Option<Response>,   // HTTP response
    // ... internal fields
}
```

## Request Data

### URL Parameters

```rust
// Get route parameter (e.g., /users/{id})
ctx.param("id") -> Option<&str>

// Typed parameter access
ctx.str_param("id") -> Result<String>
ctx.int_param("id") -> Result<i32>
ctx.str_param_or("id", "default") -> String
ctx.int_param_or("id", 0) -> i32
```

### Query Parameters

```rust
// Get query parameter (e.g., ?page=2)
ctx.query("page") -> Option<&str>

// Typed query access
ctx.str_query("page") -> Result<String>
ctx.int_query("page") -> Result<i32>
ctx.bool_query("active") -> Result<bool>
ctx.str_query_or("page", "1") -> String
ctx.int_query_or("limit", 10) -> i32
ctx.bool_query_or("active", false) -> bool
```

### Request Body

#### JSON Body

```rust
// Parse JSON body into struct
let data: MyStruct = ctx.body_json()?;

// Get full body as JSON Value
let body: Value = ctx.full_body()?;
```

#### Form Data

```rust
// Get form data as HashMap<String, String>
let form: HashMap<String, String> = ctx.body_form()?;

// Get form data with array support
let form: HashMap<String, FormValue> = ctx.body_form_data()?;

// Typed form parsing
#[derive(Deserialize)]
struct LoginForm {
    email: String,
    password: String,
}
let form: LoginForm = ctx.body_form_typed()?;

// Individual field access
let email = ctx.str_body("email")?;              // Required
let age = ctx.int_body("age")?;                  // Parse as integer
let active = ctx.bool_body("active")?;           // Parse as boolean
let name = ctx.str_body_or("name", "Anonymous"); // Optional with default
```

### Headers

```rust
// Get header value
ctx.header("Authorization") -> Option<&str>

// Add response header
ctx.add_header("X-Custom", "value");
```

### Client Information

```rust
ctx.ip() -> String                    // Client IP address
ctx.user_agent() -> Option<&str>      // User agent string
ctx.is_mobile() -> bool               // Mobile device detection
ctx.is_robot() -> bool                // Bot/crawler detection
ctx.is_secure() -> bool               // HTTPS request
ctx.is_xhr() -> bool                  // AJAX request
ctx.language() -> Option<&str>        // Preferred language
ctx.referrer() -> Option<&str>        // Referrer URL
ctx.url() -> &str                     // Request URL path
ctx.path() -> &str                    // URL path
ctx.host() -> Option<&str>            // Host header
ctx.hostname(path: Option<&str>) -> String  // Full hostname URL
ctx.extension() -> Option<&str>       // File extension
```

## Response Methods

### JSON Response

```rust
ctx.json(data: impl Serialize) -> Result<()>
```

### HTML Response

```rust
ctx.html(content: impl Into<String>) -> Result<()>
```

### Text Response

```rust
ctx.text(content: impl Into<String>) -> Result<()>
ctx.plain(text: impl Into<String>) -> Result<()>
```

### View/Template Response

```rust
ctx.view(template: &str, data: Value) -> Result<()>
```

### Redirect

```rust
ctx.redirect(path: &str) -> Result<()>
```

### HTTP Error Responses

```rust
ctx.throw400(message: Option<&str>) -> Result<()>  // Bad Request
ctx.throw401(message: Option<&str>) -> Result<()>  // Unauthorized
ctx.throw403(message: Option<&str>) -> Result<()>  // Forbidden
ctx.throw404(message: Option<&str>) -> Result<()>  // Not Found
ctx.throw409(message: Option<&str>) -> Result<()>  // Conflict
ctx.throw500(message: Option<&str>) -> Result<()>  // Internal Server Error
ctx.throw501(message: Option<&str>) -> Result<()>  // Not Implemented
ctx.view404() -> Result<()>                        // Custom 404 view
```

### Other Responses

```rust
ctx.empty() -> Result<()>                          // 204 No Content
ctx.success(data: Option<T>) -> Result<()>         // Success JSON response
ctx.status(status: StatusCode)                      // Set status code
```

### File Responses

```rust
// Download file
ctx.file_download(path: P, filename: Option<&str>) -> Result<()>

// Inline file
ctx.file_inline(path: P) -> Result<()>

// Binary data
ctx.binary(data: Vec<u8>, content_type: &str) -> Result<()>

// Stream response
ctx.stream(stream: impl Stream<Item = Result<Bytes>>, content_type: &str) -> Result<()>
```

## Session Management

### Session Access

```rust
ctx.session() -> Option<&Session>
ctx.has_session() -> bool
ctx.require_session() -> Result<&Session>
ctx.require_auth() -> Result<&Session>
```

### Session Data

```rust
// Set session value
ctx.session_set(key: &str, value: T) -> Result<()>

// Get session value
ctx.session_get(key: &str) -> Option<T>

// Remove session value
ctx.session_remove(key: &str) -> Option<Value>

// Clear all session data
ctx.session_clear()

// Flush session (save changes)
ctx.session_flush()

// Destroy session
ctx.session_destroy()
```

### Authentication

```rust
ctx.login(user_id: i64) -> Result<()>
ctx.logout() -> Result<()>
```

## Flash Messages

Flash messages are one-time messages stored in the session.

```rust
// Set flash message
ctx.flash(key: &str, value: impl Serialize) -> Result<()>

// Convenience methods
ctx.flash_success(message: impl Into<String>) -> Result<()>
ctx.flash_error(message: impl Into<String>) -> Result<()>
ctx.flash_info(message: impl Into<String>) -> Result<()>
ctx.flash_warning(message: impl Into<String>) -> Result<()>

// Get flash message
ctx.get_flash(key: &str) -> Option<Value>
ctx.get_all_flash() -> HashMap<String, Value>

// Clear flash messages
ctx.flash_clear() -> Result<()>
ctx.flash_clear_key(key: &str) -> Result<()>
```

## Repository Data

Repository data is handler-scoped data accessible in all views rendered within the handler.

```rust
// Set repository data
ctx.repository_set(key: &str, value: impl Into<Value>) -> &mut Self

// Get repository data
ctx.repository_get(key: &str) -> Option<&Value>

// Clear all repository data
ctx.repository_clear() -> &mut Self
```

## Layout Management

```rust
// Set layout for views
ctx.layout(name: &str) -> &mut Self

// Use empty layout
ctx.layout("")
```

## File Uploads

```rust
// Get all uploaded files
ctx.files() -> Result<&FileCollection>

// Get specific uploaded file
ctx.file(field_name: &str) -> Result<Option<&UploadedFile>>
```

## Middleware Data

Middleware can store data in the context for communication.

```rust
// Store data
ctx.set(key: &str, value: T) -> Result<()>

// Retrieve data
ctx.get(key: &str) -> Option<&T>

// Check if data exists
ctx.has_data(key: &str) -> bool
```

## Request Data Helper

```rust
// Get comprehensive request data
ctx.request_data() -> Result<RequestData>
```

## Response Management

```rust
// Set custom response
ctx.set_response(response: Response)

// Get response
ctx.get_response() -> Option<&Response>

// Take response
ctx.take_response() -> Option<Response>
```

## Examples

### Basic Handler

```rust
async fn get_user(ctx: &mut Context) -> Result<()> {
    let user_id = ctx.int_param("id")?;
    // ... fetch user ...
    ctx.json(json!({"user": user}))
}
```

### Form Handling

```rust
async fn create_user(ctx: &mut Context) -> Result<()> {
    let form: CreateUserForm = ctx.body_form_typed()?;
    // ... create user ...
    ctx.flash_success("User created!")?;
    ctx.redirect("/users")
}
```

### Session Usage

```rust
async fn dashboard(ctx: &mut Context) -> Result<()> {
    let user: User = ctx.session_get("user")
        .ok_or_else(|| ctx.throw401(Some("Login required")))?;
    
    ctx.repository_set("user", json!(user));
    ctx.view("/dashboard/index", json!({}))
}
```


