# Hello World Tutorial

This tutorial will walk you through creating your first RustF application step-by-step.

## What We'll Build

We'll create a simple application that:
- Displays a "Hello World" message
- Shows a JSON API endpoint
- Demonstrates basic routing

## Step 1: Create the Project

```bash
rustf-cli new project hello-rustf
cd hello-rustf
```

Or manually:

```bash
cargo new hello-rustf
cd hello-rustf
```

## Step 2: Add Dependencies

Edit `Cargo.toml`:

```toml
[dependencies]
rustf = { path = "../rustf" }  # Adjust path as needed
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
log = "0.4"
env_logger = "0.10"
```

## Step 3: Create the Main Application

Edit `src/main.rs`:

```rust
use rustf::prelude::*;

#[tokio::main]
async fn main() -> rustf::Result<()> {
    // Initialize logger
    env_logger::init();
    
    // Create the application with auto-discovery
    let app = RustF::new()
        .controllers(auto_controllers!())      // Auto-discover controllers
        .middleware_from(auto_middleware!());   // Auto-discover middleware
    
    println!("🚀 Server running at http://127.0.0.1:8000");
    println!("📖 Visit http://127.0.0.1:8000/ for Hello World");
    println!("📡 Visit http://127.0.0.1:8000/api/status for JSON API");
    
    // Start the server
    app.start().await
}
```

## Step 4: Create Your First Controller

Create `src/controllers/home.rs`:

```rust
use rustf::prelude::*;

// Every controller must have an install() function
pub fn install() -> Vec<Route> {
    routes![
        GET "/" => hello_world,
        GET "/api/status" => api_status,
    ]
}

// Handler function - must be async and take &mut Context
async fn hello_world(ctx: &mut Context) -> Result<()> {
    // Send HTML response
    ctx.html("<h1>Hello, RustF! 🚀</h1><p>Welcome to your first RustF application!</p>")
}

// JSON API endpoint
async fn api_status(ctx: &mut Context) -> Result<()> {
    // Create JSON data
    let data = json!({
        "status": "ok",
        "framework": "RustF",
        "version": "0.1.0",
        "message": "Hello from the API!"
    });
    
    // Send JSON response
    ctx.json(data)
}
```

## Step 5: Create Configuration

Create `config.toml`:

```toml
[server]
host = "127.0.0.1"
port = 8000
timeout = 30

[views]
directory = "views"
cache_enabled = false
default_layout = "layouts/default"

[session]
timeout = 3600
cookie_name = "hello_rustf_session"
secure = false
http_only = true
```

## Step 6: Run Your Application

```bash
cargo run
```

You should see:
```
🚀 Server running at http://127.0.0.1:8000
📖 Visit http://127.0.0.1:8000/ for Hello World
📡 Visit http://127.0.0.1:8000/api/status for JSON API
```

## Step 7: Test Your Application

### Test the HTML Endpoint

Visit `http://127.0.0.1:8000/` in your browser. You should see:
```html
Hello, RustF! 🚀
Welcome to your first RustF application!
```

### Test the JSON API

Visit `http://127.0.0.1:8000/api/status` or use curl:

```bash
curl http://127.0.0.1:8000/api/status
```

You should get:
```json
{
  "status": "ok",
  "framework": "RustF",
  "version": "0.1.0",
  "message": "Hello from the API!"
}
```

## Understanding the Code

### The `install()` Function

Every controller must have an `install()` function that returns `Vec<Route>`. This function:
- Defines all routes for this controller
- Uses the `routes![]` macro for clean syntax
- Is automatically discovered by `auto_controllers!()`

### Route Handlers

Route handlers must:
- Be `async` functions
- Take `&mut Context` as the only parameter
- Return `Result<()>`
- Use `ctx` methods to send responses

### Response Methods

The `Context` provides several response methods:

- `ctx.html(content)` - Send HTML response
- `ctx.json(data)` - Send JSON response
- `ctx.text(content)` - Send plain text
- `ctx.redirect(url)` - Redirect to another URL
- `ctx.status(code)` - Set HTTP status code

## Adding More Routes

Let's add a route with a parameter:

```rust
pub fn install() -> Vec<Route> {
    routes![
        GET "/" => hello_world,
        GET "/api/status" => api_status,
        GET "/hello/{name}" => personalized_hello,  // New route with parameter
    ]
}

async fn personalized_hello(ctx: &mut Context) -> Result<()> {
    // Get route parameter
    let name = ctx.param("name").unwrap_or("World");
    
    let message = format!("Hello, {}! Welcome to RustF!", name);
    ctx.html(&format!("<h1>{}</h1>", message))
}
```

Now visit `http://127.0.0.1:8000/hello/Alice` to see a personalized greeting!

## Next Steps

Congratulations! You've created your first RustF application. Now you can:

1. **[Learn about Project Structure](project-structure.md)** - Understand how RustF projects are organized
2. **[Explore Controllers](../guides/controllers.md)** - Learn more about routing and controllers
3. **[Add Views](../guides/views.md)** - Use templates instead of inline HTML
4. **[Build a REST API](../examples/rest-api.md)** - Create a complete API

## Common Questions

**Q: Why do I need `&mut Context`?**
A: Context is mutable because it stores request/response data and session information that may change during request processing.

**Q: What does `auto_controllers!()` do?**
A: It's a macro that automatically discovers all controllers in `src/controllers/` at compile time and registers their routes.

**Q: Can I use `&Context` instead?**
A: No, handlers require `&mut Context` because they need to modify the response and potentially update session data.

**Q: How do I handle errors?**
A: Return `Err(Error::...)` from your handler. The framework will handle it appropriately based on the error type.


