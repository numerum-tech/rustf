# Context API Reference

The `Context` struct gives a handler everything it needs about the
incoming request and lets it write the outgoing response. Every route
handler receives `&mut Context` and returns `rustf::Result<()>` — the
framework picks the response back up via `ctx.take_response()` after
the handler returns.

## Overview

```rust
pub struct Context {
    pub req: Request,             // HTTP request (headers, body, cookies, …)
    pub res: Option<Response>,    // HTTP response (mutated by ctx.json/view/etc.)
    // ... private fields: session, views, layout_name, repository, data, ...
}
```

> **Naming convention.** All typed accessors come in a **source-first**
> form (`param_str`, `query_int`, `body_bool`) and a deprecated
> **type-first** alias (`str_param`, `int_query`, `bool_body`). The
> source-first names are the canonical choice — they sort and grep
> better and match what you'll see in current code. The aliases still
> compile for backward compatibility.

## Request Data

### URL Parameters

```rust
// Untyped
ctx.param("id") -> Option<&str>

// Typed accessors (source-first, recommended)
ctx.param_str("id")            -> Result<String>
ctx.param_int("id")            -> Result<i32>
ctx.param_str_or("id", "default") -> String
ctx.param_int_or("id", 0)      -> i32
```

> `param_int` returns `i32`. If your model id type is `i64`, do
> `let id = ctx.param_int("id")? as i64;` — `where_eq` accepts
> `Into<SqlValue>` so the cast is safe across `i32`/`i64` columns.

### Query Parameters

```rust
// Untyped (e.g. ?page=2)
ctx.query("page") -> Option<&str>

// Typed (source-first, recommended)
ctx.query_str("page")             -> Result<String>
ctx.query_int("page")             -> Result<i32>
ctx.query_bool("active")          -> Result<bool>
ctx.query_str_or("page", "1")     -> String
ctx.query_int_or("limit", 10)     -> i32
ctx.query_bool_or("active", false) -> bool
```

### Request Body

#### JSON body

```rust
// Synchronous — no .await. Uses simd-json under the hood.
let data: MyStruct = ctx.body_json()?;

// Untyped — useful when shape is dynamic
let body: serde_json::Value = ctx.full_body()?;
```

#### Form data

```rust
// Returns FormData — a newtype wrapper around HashMap<String, String>
// that derefs to the underlying map. Use it as a map directly.
let form = ctx.body_form()?;            // FormData
let email = form.get("email").cloned(); // works via Deref

// Form data with array support (multi-value fields like checkboxes)
let form = ctx.body_form_data()?;       // &HashMap<String, FormValue>

// Strongly-typed form parsing via serde
#[derive(Deserialize)]
struct LoginForm { email: String, password: String }
let form: LoginForm = ctx.body_form_typed()?;

// Individual field accessors (source-first, recommended)
let email   = ctx.body_str("email")?;                // Required, returns String
let age     = ctx.body_int("age")?;                  // Parse as i32
let active  = ctx.body_bool("active")?;              // Parse as bool
let name    = ctx.body_str_or("name", "Anonymous");  // Optional with default
```

### Headers

```rust
ctx.header("Authorization") -> Option<&str>     // read request header
ctx.add_header("X-Custom", "value");            // append response header
```

### Cookies

Cookies live on `Request`, not directly on `Context` — but they're a
short hop away via `ctx.req`:

```rust
// Get one cookie value (parsed once, then cached for the whole request)
ctx.req.cookie("session") -> Option<String>

// All cookies as a borrowed map (same lazy cache)
ctx.req.cookies() -> &HashMap<String, String>
```

The Cookie header is parsed once per request and the result cached on
`Request`, so multiple middleware reading different cookies (session,
flash, CSRF) share one parse.

### Client Information

```rust
ctx.ip()                         -> String           // Client IP
ctx.user_agent()                 -> Option<&str>
ctx.is_mobile()                  -> bool
ctx.is_robot()                   -> bool
ctx.is_secure()                  -> bool             // HTTPS
ctx.is_xhr()                     -> bool             // AJAX detected
ctx.language()                   -> Option<&str>     // Accept-Language
ctx.referrer()                   -> Option<&str>
ctx.url()                        -> &str             // path + query
ctx.path()                       -> &str             // path only
ctx.host()                       -> Option<&str>
ctx.hostname(path: Option<&str>) -> String           // full URL with scheme
ctx.extension()                  -> Option<&str>     // file extension
```

## Response Methods

All response methods take `&mut self` and write to `ctx.res` in place.
They return `rustf::Result<()>` so you can `?`-propagate errors.

### JSON

```rust
ctx.json(data: impl Serialize) -> Result<()>
```

### HTML / Text

```rust
ctx.html(content: impl Into<String>)  -> Result<()>
ctx.text(content: impl Into<String>)  -> Result<()>
ctx.plain(text: impl Into<String>)    -> Result<()>
```

### View / Template

```rust
// Template name has NO file extension — the framework appends the
// configured one (default `.html`).
ctx.view(template: &str, data: Value) -> Result<()>
```

### Redirect

```rust
ctx.redirect(path: &str) -> Result<()>
```

### HTTP Error Responses

Each `throwNNN` sets `ctx.res` to an error response AND returns `Err`
so the `?` operator stops the handler chain naturally:

```rust
ctx.throw400(message: Option<&str>) -> Result<()>  // Bad Request
ctx.throw401(message: Option<&str>) -> Result<()>  // Unauthorized
ctx.throw403(message: Option<&str>) -> Result<()>  // Forbidden
ctx.throw404(message: Option<&str>) -> Result<()>  // Not Found
ctx.throw409(message: Option<&str>) -> Result<()>  // Conflict
ctx.throw500(message: Option<&str>) -> Result<()>  // Internal Server Error
ctx.throw501(message: Option<&str>) -> Result<()>  // Not Implemented
ctx.view404() -> Result<()>                        // Render the 404 view
```

### Other Responses

```rust
ctx.empty()                              -> Result<()>     // 204 No Content
ctx.success(data: Option<T>)             -> Result<()>     // {success: true, data: ...}
ctx.status(status: hyper::StatusCode)                      // mutate status (no Result)
```

### File Responses

```rust
ctx.file_download<P: AsRef<Path>>(path: P, filename: Option<&str>) -> Result<()>
ctx.file_inline<P: AsRef<Path>>(path: P) -> Result<()>

ctx.binary(
    data: Vec<u8>,
    content_type: &str,
    download_name: Option<&str>,
) -> Result<()>

ctx.stream(
    data: Vec<u8>,
    content_type: &str,
    download_name: Option<&str>,
) -> Result<()>
```

> The current `stream` buffers the full body in memory. True chunked
> streaming at the hyper layer is on the roadmap, not yet shipped.

## Session Management

### Session Access

```rust
ctx.session()         -> Option<&Session>
ctx.has_session()     -> bool
ctx.require_session() -> Result<&Session>   // Err if no session is attached
ctx.require_auth()    -> Result<&Session>   // Err if no authenticated session
```

### Session Data

```rust
// Set — value must be Serialize
ctx.session_set(key: &str, value: T) -> Result<()>

// Get — turbofish for the deserialised type
ctx.session_get::<T>(key: &str) -> Option<T>

// Remove returns the prior Value if there was one
ctx.session_remove(key: &str) -> Option<Value>

// Lifecycle
ctx.session_clear()
ctx.session_flush()                          // alias for clear
ctx.session_destroy()                        // full invalidation at end of request
```

### Authentication

```rust
ctx.login(user_id: i64) -> Result<()>
ctx.logout()            -> Result<()>
```

## Flash Messages

Flash messages are one-time messages stored in the session.

```rust
// Generic
ctx.flash(key: &str, value: impl Serialize) -> Result<()>

// Convenience (key is the level itself: "success", "error", ...)
ctx.flash_success(message: impl Into<String>) -> Result<()>
ctx.flash_error(message: impl Into<String>)   -> Result<()>
ctx.flash_info(message: impl Into<String>)    -> Result<()>
ctx.flash_warning(message: impl Into<String>) -> Result<()>

// Read (also clears)
ctx.get_flash(key: &str) -> Option<Value>
ctx.get_all_flash()      -> HashMap<String, Value>

// Manual clearing
ctx.flash_clear()                -> Result<()>
ctx.flash_clear_key(key: &str)   -> Result<()>
```

## Repository Data — visible in views

Repository data is request-scoped data that the renderer surfaces in
templates as `@{R.<key>}`. Use it for anything a view needs.

```rust
ctx.repository_set(key: &str, value: impl Into<Value>) -> &mut Self  // chainable
ctx.repository_get(key: &str) -> Option<&Value>
ctx.repository_clear() -> &mut Self
```

## Layout Management

```rust
ctx.layout(name: &str) -> &mut Self  // empty string = no layout
```

## File Uploads

```rust
ctx.files() -> Result<&FileCollection>
ctx.file(field_name: &str) -> Result<Option<&UploadedFile>>
```

## Middleware Data — NOT visible in views

For middleware-internal state (timing, auth claims, etc.), use the
`set` / `get` typed slot. These do NOT reach templates — that's what
`repository_*` is for.

```rust
ctx.set(key: &str, value: T) -> Result<()>
ctx.get::<T>(key: &str)      -> Option<&T>
ctx.has_data(key: &str)      -> bool
```

## Request Data Helper

```rust
ctx.request_data() -> Result<RequestData>   // structured aggregate
```

## Response Management

```rust
ctx.set_response(response: Response)
ctx.get_response()  -> Option<&Response>
ctx.take_response() -> Option<Response>     // framework calls this after the handler
```

## Examples

### Basic Handler

```rust
async fn get_user(ctx: &mut Context) -> rustf::Result<()> {
    let user_id = ctx.param_int("id")? as i64;
    // ... fetch user via your service module ...
    ctx.json(json!({"user": user_id}))
}
```

### Form Handling

```rust
async fn create_user(ctx: &mut Context) -> rustf::Result<()> {
    let form: CreateUserForm = ctx.body_form_typed()?;
    // ... call into your module ...
    ctx.flash_success("User created!")?;
    ctx.redirect("/users")
}
```

### Session Usage

```rust
async fn dashboard(ctx: &mut Context) -> rustf::Result<()> {
    let user_id: i64 = ctx
        .session_get::<i64>("user_id")
        .ok_or_else(|| {
            // throw* sets ctx.res AND returns Err — perfect for ?
            let _ = ctx.throw401(Some("Login required"));
            rustf::Error::authentication("Login required")
        })?;

    ctx.repository_set("user_id", user_id);
    ctx.view("dashboard/index", json!({}))
}
```

### Reading Cookies

```rust
async fn show_visitor(ctx: &mut Context) -> rustf::Result<()> {
    let last_visit = ctx.req.cookie("last_visit").unwrap_or_default();
    ctx.json(json!({ "last_visit": last_visit }))
}
```
