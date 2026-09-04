# Routing API Reference

RustF provides a clean routing system built around the `routes!` macro
and `Route` struct, dispatched by a high-performance trie router.

## Route Definition

### The `routes!` Macro

Two arms — pick the one you need:

```rust
// Bare arm — the common case
routes![
    GET    "/path"      => handler_function,
    POST   "/path"      => handler_function,
    PUT    "/path/{id}" => handler_function,
    DELETE "/path/{id}" => handler_function,
    XHR    "/api/data"  => handler_function,   // XHR-only (AJAX)
]
```

```rust
// With an optional controller-level `before` hook
routes![
    before: before,
    GET  "/path"      => handler_function,
    POST "/path"      => handler_function,
]
```

The `before:` clause attaches a pre-handler hook to **every** route in
this `install()`. See [Controller-level `before` Hook](../guides/controllers.md#controller-level-before-hook)
in the controllers guide for the full pattern.

### Route Struct

```rust
pub struct Route {
    pub method: String,           // HTTP method ("GET", "POST", ..., "XHR")
    pub path: String,             // URL pattern, e.g. "/users/{id}"
    pub handler: RouteHandler,    // Handler function pointer
    pub xhr_only: bool,           // Set when constructed via Route::xhr / `XHR` arm
    pub before: Option<BeforeFn>, // Optional controller-level pre-handler hook
}
```

### Handler Types

```rust
// Handler — runs after `before` (if any) and writes the response in place.
pub type RouteHandler = for<'a> fn(&'a mut Context)
    -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

// Optional pre-handler hook — controller-level setup or guard.
pub type BeforeFn = for<'a> fn(&'a mut Context)
    -> Pin<Box<dyn Future<Output = Result<BeforeAction>> + Send + 'a>>;

pub enum BeforeAction { Continue, Stop }
```

`RouteHandler` is a function pointer type, not a trait object — non-
capturing closures coerce to it via the `routes!` macro.

## Manual Route Creation

You can build routes without the macro using `Route::*` constructors:

```rust
use rustf::routing::Route;

Route::get("/users",      handler);
Route::post("/users",     handler);
Route::put("/users/{id}", handler);
Route::delete("/users/{id}", handler);
Route::xhr("/api/data",   handler);

// Generic constructor (any method including custom):
Route::new("PATCH", "/users/{id}", handler);
```

All constructors set `before: None`. Use the `routes![before: ..., ...]`
macro arm to wire a hook (or set the field manually if you really need
to).

## URL Parameters

Routes use `{parameter}` syntax (not `:parameter`):

```rust
routes![
    GET "/users/{id}"                              => get_user,
    GET "/posts/{post_id}/comments/{comment_id}"   => get_comment,
]
```

Access parameters in handlers via the typed accessors on `Context`:

```rust
async fn get_user(ctx: &mut Context) -> rustf::Result<()> {
    // String parameter
    let id = ctx.param_str("id")?;            // -> String

    // Integer parameter (returns i32; cast to i64 if your model uses i64)
    // let id = ctx.param_as::<i64>("id")?;
    Ok(())
}
```

The typed accessors return `Err` when the parameter is missing or
unparseable. For a genuinely optional parameter use `param_str_or` /
`param_int_or` / `param_as_or` with a default, or read `ctx.req.params`
directly.

## Route Registration

### Auto-Discovery (Recommended)

```rust
let app = RustF::new()
    .controllers(auto_controllers!());
```

`auto_controllers!()` scans `src/controllers/*.rs` at compile time and
calls each module's `install()` function.

### Manual Registration

```rust
let app = RustF::new()
    .controllers({
        let mut routes = Vec::new();
        routes.extend(controllers::home::install());
        routes.extend(controllers::users::install());
        routes
    });
```

## Controller Pattern

Every controller exports an `install()` function returning `Vec<Route>`:

```rust
use rustf::prelude::*;

pub fn install() -> Vec<Route> {
    routes![
        GET  "/"       => index,
        POST "/submit" => submit,
    ]
}

async fn index(ctx: &mut Context) -> rustf::Result<()> { /* ... */ }
async fn submit(ctx: &mut Context) -> rustf::Result<()> { /* ... */ }
```

## HTTP Methods

Supported in the `routes!` macro and as `Route::*` constructors:

| Macro keyword | `Route::*`         | Notes |
|---------------|--------------------|-------|
| `GET`         | `Route::get`       | Retrieve a resource |
| `POST`        | `Route::post`      | Create a resource (form/JSON body) |
| `PUT`         | `Route::put`       | Update a resource |
| `DELETE`      | `Route::delete`    | Delete a resource |
| `XHR`         | `Route::xhr`       | AJAX-only — registered for both GET and POST internally; rejects non-XHR requests with 403 |

Other methods (PATCH, HEAD, OPTIONS, …) are reachable via
`Route::new("PATCH", path, handler)`.

## Route Matching

The router is a trie (`rustf/src/routing/trie.rs`) — matching is
**O(log n)** in the number of path segments, not O(n) over registered
routes. When several routes could match a given path, the trie picks
the **most specific** match in this order:

1. **Exact (static) segment** — e.g. `/users/special` always wins over
   `/users/{id}`.
2. **Parameter segment** — `{id}` matches anything in that position.
3. **Wildcard** (`*`) — least specific, matches the rest of the path.

Method matching is exact on the verb. A `GET` request to a route
registered only for `POST` returns 404, not 405.

## Examples

### Basic Routes

```rust
pub fn install() -> Vec<Route> {
    routes![
        GET "/"      => index,
        GET "/about" => about,
    ]
}
```

### RESTful Routes

```rust
pub fn install() -> Vec<Route> {
    routes![
        GET    "/api/users"      => list_users,
        GET    "/api/users/{id}" => get_user,
        POST   "/api/users"      => create_user,
        PUT    "/api/users/{id}" => update_user,
        DELETE "/api/users/{id}" => delete_user,
    ]
}
```

### With a `before` Hook

```rust
pub fn install() -> Vec<Route> {
    async fn before(ctx: &mut Context) -> rustf::Result<BeforeAction> {
        if !ctx.has_session() {
            ctx.redirect("/login")?;
            return Ok(BeforeAction::Stop);
        }
        Ok(BeforeAction::Continue)
    }

    routes![
        before: before,
        GET    "/admin"          => dashboard,
        GET    "/admin/users"    => list_users,
        DELETE "/admin/users/{id}" => delete_user,
    ]
}
```
