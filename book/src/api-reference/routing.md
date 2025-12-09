# Routing API Reference

RustF provides a clean routing system with the `routes!` macro and `Route` struct.

## Route Definition

### The `routes!` Macro

```rust
routes![
    GET "/path" => handler_function,
    POST "/path" => handler_function,
    PUT "/path/{id}" => handler_function,
    DELETE "/path/{id}" => handler_function,
]
```

### Route Struct

```rust
pub struct Route {
    pub method: String,        // HTTP method
    pub path: String,          // URL pattern
    pub handler: RouteHandler,  // Handler function
}
```

### RouteHandler Type

```rust
pub type RouteHandler = for<'a> fn(&'a mut Context) 
    -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
```

## Manual Route Creation

```rust
use rustf::routing::Route;

// Create routes manually
Route::get("/users", handler)
Route::post("/users", handler)
Route::put("/users/{id}", handler)
Route::delete("/users/{id}", handler)
```

## URL Parameters

Routes support URL parameters using `{parameter}` syntax:

```rust
routes![
    GET "/users/{id}" => get_user,
    GET "/posts/{post_id}/comments/{comment_id}" => get_comment,
]
```

Access parameters in handlers:

```rust
async fn get_user(ctx: &mut Context) -> Result<()> {
    let user_id = ctx.param("id")?;
    // ...
}
```

## Route Registration

### Auto-Discovery (Recommended)

```rust
let app = RustF::new()
    .controllers(auto_controllers!());
```

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

Every controller must have an `install()` function:

```rust
pub fn install() -> Vec<Route> {
    routes![
        GET "/" => index,
        POST "/submit" => submit,
    ]
}
```

## HTTP Methods

Supported methods:
- `GET` - Retrieve resource
- `POST` - Create resource
- `PUT` - Update resource
- `DELETE` - Delete resource

## Route Matching

Routes are matched in order of registration. First match wins.

## Examples

### Basic Routes

```rust
pub fn install() -> Vec<Route> {
    routes![
        GET "/" => index,
        GET "/about" => about,
    ]
}
```

### RESTful Routes

```rust
pub fn install() -> Vec<Route> {
    routes![
        GET    "/api/users" => list_users,
        GET    "/api/users/{id}" => get_user,
        POST   "/api/users" => create_user,
        PUT    "/api/users/{id}" => update_user,
        DELETE "/api/users/{id}" => delete_user,
    ]
}
```

### Nested Routes

```rust
// src/controllers/api/users.rs
pub fn install() -> Vec<Route> {
    routes![
        GET "/api/users" => list,
        GET "/api/users/{id}" => show,
    ]
}
```





