# RustF Controllers User Guide

**Complete documentation based on current framework implementation**

## Overview

RustF provides a clean, organized controller system inspired by Total.js. Controllers are Rust modules that group related route handlers together and expose them through an `install()` function. The framework supports both manual controller registration and automatic controller discovery for streamlined development.

### Key Features
- **Convention-based routing** - Clean `routes![]` macro syntax
- **Auto-discovery** - Automatically finds and registers controllers at compile time
- **Total.js-inspired API** - Familiar patterns for web developers
- **Type-safe handlers** - All handlers are statically checked at compile time
- **Flexible organization** - Group related functionality logically

## Core Components

### Route System

```rust
pub struct Route {
    pub method: String,    // HTTP method (GET, POST, PUT, DELETE)
    pub path: String,      // URL pattern with optional parameters ({id})
    pub handler: RouteHandler,  // Async handler function
}

// Type alias for route handlers - all handlers follow this signature
pub type RouteHandler = for<'a> fn(&'a mut Context) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
```

**Route Handler Requirements:**
- Must be `async` functions
- Take a single `&mut Context` parameter (mutable reference)
- Return `Result<()>` (response is set internally on Context)
- Are automatically wrapped by the `routes!` macro

### Supported HTTP Methods

The framework supports all standard HTTP methods:

```rust
// Available in routes! macro:
GET    "/path"     => handler_function
POST   "/path"     => handler_function  
PUT    "/path"     => handler_function
DELETE "/path"     => handler_function

// Manual route creation (rarely needed):
Route::get("/users", handler)           // GET request
Route::post("/users", handler)          // POST request
Route::put("/users/{id}", handler)       // PUT request
Route::delete("/users/{id}", handler)    // DELETE request
```

## Controller Pattern

### Basic Controller Structure

Every controller must follow this exact pattern:

```rust
use rustf::prelude::*;

// Required: Every controller must have an install() function
pub fn install() -> Vec<Route> {
    routes![
        GET "/" => index,
        POST "/submit" => submit,
        GET "/item/{id}" => show_item,
    ]
}

// Handler functions - must be async and return Result<()>
async fn index(ctx: &mut Context) -> Result<()> {
    let data = json!({"message": "Welcome!"});
    ctx.view("/home/index", data)
}

async fn submit(ctx: &mut Context) -> Result<()> {
    let form_data = ctx.body_form()?;
    // Process form data
    ctx.redirect("/success")
}

async fn show_item(ctx: &mut Context) -> Result<()> {
    let item_id = ctx.param("id").unwrap_or("0").to_string();
    let data = json!({"id": item_id});
    ctx.json(data)
}
```

### Controller File Organization

```
src/controllers/
├── home.rs          # Home page and static content
├── auth.rs          # Login, logout, registration
├── users.rs         # User management
├── api/
│   ├── users.rs     # API endpoints for users
│   └── posts.rs     # API endpoints for posts
└── admin.rs         # Admin functionality
```

### The routes! Macro

The `routes!` macro provides clean, declarative route definition:

```rust
routes![
    // Basic routes
    GET "/" => index,
    GET "/about" => about,
    
    // Routes with parameters
    GET "/users/{id}" => get_user,
    GET "/posts/{post_id}/comments/{comment_id}" => get_comment,
    
    // Different HTTP methods
    POST "/users" => create_user,
    PUT "/users/{id}" => update_user,
    DELETE "/users/{id}" => delete_user,
    
    // Complex paths
    GET "/api/v1/users/{id}/profile" => get_user_profile,
]
```

**Features:**
- Clean, readable syntax
- Automatic handler wrapping
- Compile-time validation
- Support for URL parameters with `{parameter}` syntax
- Trailing commas allowed

## Writing Route Handlers

### Handler Function Requirements

```rust
// Required signature for all handlers
async fn handler_name(ctx: &mut Context) -> Result<()>
```

**Handler Rules:**
- Must be `async` functions
- Must take exactly one `&mut Context` parameter (mutable reference)
- Must return `Result<()>` - the response is set internally on the Context
- Can have any name (referenced in routes! macro)
- Are automatically wrapped by the framework

### Response Handling Pattern

**Key Change:** Handlers now receive `&mut Context` and return `Result<()>` instead of `Result<Response>`. This architectural change ensures that session data and all middleware modifications persist throughout the entire request/response lifecycle.

#### How Responses Work

1. **Response Storage**: The Context struct now contains an `Option<Response>` field initialized with a default 200 OK response
2. **Setting Responses**: All response helper methods (`json()`, `view()`, `redirect()`, etc.) internally call `ctx.set_response()`
3. **Return Type**: Methods return `Result<()>` to indicate success/failure of setting the response
4. **Middleware Access**: Both inbound and outbound middleware can access and modify the response via `ctx.response`
5. **Response Helpers in Middleware**: Since Context initializes with a default response, middleware can use the same response helpers (`ctx.json()`, `ctx.throw403()`, etc.) as handlers

#### Custom Response Creation

If you need to create a custom response beyond the built-in helpers:

```rust
async fn custom_response_handler(ctx: &mut Context) -> Result<()> {
    // Create a custom response
    let response = Response::new(StatusCode::from_u16(418).unwrap())
        .with_header("X-Custom", "value")
        .with_body(b"I'm a teapot".to_vec());
    
    // Set it on the context
    ctx.set_response(response);
    Ok(())
}
```

#### Error Handling Pattern

Since handlers return `Result<()>`, error responses are handled the same way as success responses:

```rust
async fn validated_handler(ctx: &mut Context) -> Result<()> {
    let data = ctx.body_json::<MyData>()?;
    
    if !data.is_valid() {
        // Error response - still returns Result<()>
        return ctx.throw400(Some("Invalid data"));
    }
    
    // Success response - also returns Result<()>
    ctx.json(json!({"status": "success"}))
}
```

### Working with Context

The `Context` parameter provides access to all request/response functionality.

**Important Note on Middleware Context Preservation:**
The framework ensures that all context modifications made by middleware (such as setting layout, repository data, or session values) are properly preserved and passed to your controller handlers. The context is passed by mutable reference through the middleware chain and arrives at your handler with all modifications intact.

#### Request Data
```rust
// URL parameters (/users/{id} -> id)
let user_id = ctx.param("id").unwrap_or("0");

// Query parameters (?page=2 -> page)
let page = ctx.query("page").unwrap_or("1");

// Form data - Three approaches available:

// 1. Manual parsing (low-level, verbose but flexible)
let form_data = ctx.body_form()?;
let email = form_data.get("email").unwrap_or(&String::new());

// 2. Typed parsing (recommended - automatic deserialization)
#[derive(serde::Deserialize)]
struct LoginForm {
    email: String,
    password: String,
}
let form: LoginForm = ctx.body_form_typed()?;
let email = form.email;

// 3. Individual field helpers (for simple cases)
let email = ctx.str_body("email")?;              // Required field
let name = ctx.str_body_or("name", "Anonymous"); // Optional with default
let age = ctx.int_body("age")?;                  // Parse as integer
let active = ctx.bool_body_or("active", false);  // Parse as boolean

// JSON body
let json_data: MyStruct = ctx.body_json()?;

// Headers
let auth_header = ctx.header("Authorization");

// File uploads
let uploaded_file = ctx.file("avatar")?;
```

#### Response Modification
```rust
// Add custom headers to response
ctx.add_header("X-Custom-Header", "value");
ctx.add_header("Cache-Control", "no-cache");

// Set response status code
ctx.status(hyper::StatusCode::CREATED);  // 201 Created
ctx.status(hyper::StatusCode::ACCEPTED); // 202 Accepted

// These methods are particularly useful in middleware
// since Context now initializes with a default 200 OK response
```

#### Response Generation

**Important:** All response methods now set the response internally on the Context and return `Result<()>` instead of `Result<Response>`. This ensures that session data and middleware modifications are preserved throughout the request lifecycle.

```rust
// Template responses (data accessed via model.key or M.key in template)
ctx.view("/users/profile", json!({"user": user_data}))?  // Sets template response

// JSON responses
ctx.json(json!({"status": "success", "data": users}))?   // Sets JSON response

// Redirects
ctx.redirect("/login")?                                  // Sets redirect response

// HTTP errors - all return Result<()> after setting error response
ctx.throw404(Some("User not found"))?                   // Sets 404 error
ctx.throw400(Some("Invalid input"))?                    // Sets 400 error
ctx.throw500(None)?                                      // Sets 500 error

// Plain text
ctx.text("Hello, world!")?                              // Sets text response

// File responses
ctx.file_download("/path/to/file", Some("name.pdf"))?   // Sets file download
ctx.file_inline("/path/to/image.jpg")?                  // Sets inline file
```

#### Session Management
```rust
// Set session data
ctx.session_set("user_id", 123)?;
ctx.session_set("cart", json!({"items": []}));

// Get session data
let user_id: Option<i32> = ctx.session_get("user_id");
let cart: Option<Value> = ctx.session_get("cart");

// Remove session data
ctx.session_remove("temporary_data");
```

#### Repository Data (Handler-Scoped Data)

The repository system allows controllers to pass data to all views called within the handler function without explicitly including it in each view's data parameter. The repository lives for the duration of the handler function execution.

**Setting Repository Data**
```rust
async fn my_handler(ctx: &mut Context) -> Result<()> {
    // Set simple values
    ctx.repository_set("app_name", "RustF Application");
    ctx.repository_set("current_section", "dashboard");
    ctx.repository_set("user_level", 5);
    
    // Set arrays
    ctx.repository_set("nav_items", json!([
        {"title": "Home", "url": "/"},
        {"title": "About", "url": "/about"},
        {"title": "Contact", "url": "/contact"}
    ]));
    
    // Set complex objects
    ctx.repository_set("site_config", json!({
        "theme": "dark",
        "sidebar": "expanded",
        "notifications": true
    }));
    
    // Repository data is automatically available in the view
    // Note: View data is accessed via model.key or M.key in templates
    ctx.view("/my_view", json!({"title": "Page Title"}))
}
```

**Important Note About View Data Access**
```rust
// When you pass data to ctx.view(), it becomes the "model" in the template
ctx.view("/template", json!({
    "title": "My Page",
    "users": vec![...]
}))

// In the template, access this data with model. or M. prefix:
// @{model.title} or @{M.title}
// @{model.users} or @{M.users}
// NOT directly as @{title} or @{users}
```

**Working with Repository**
```rust
// Get data from repository
let section = ctx.repository_get("current_section");

// Clear all repository data
ctx.repository_clear();

// Chain multiple operations
ctx.repository_set("key1", "value1")
   .repository_set("key2", "value2")
   .repository_clear();  // Returns &mut Self for chaining
```

**Accessing in Templates**
```html
<!-- Use repository.key or R.key (shorthand) -->
<h1>@{repository.app_name}</h1>
<div class="section-@{R.current_section}">

<!-- Access nested data -->
Theme: @{repository.site_config.theme}
Sidebar: @{R.site_config.sidebar}

<!-- Use in conditionals -->
@{if R.site_config.notifications}
    <div class="notifications">Enabled</div>
@{fi}

<!-- Iterate over arrays -->
@{foreach item in repository.nav_items}
    <a href="@{item.url}">@{item.title}</a>
@{end}
```

**Complete Example**
```rust
use rustf::prelude::*;

pub fn install() -> Vec<Route> {
    routes![
        GET "/dashboard" => dashboard,
    ]
}

async fn dashboard(ctx: &mut Context) -> Result<()> {
    // Set shared repository data
    ctx.repository_set("user", json!({
        "name": "John Doe",
        "role": "admin",
        "avatar": "/images/john.jpg"
    }));
    
    ctx.repository_set("breadcrumbs", json!([
        {"label": "Home", "url": "/"},
        {"label": "Dashboard", "url": null}
    ]));
    
    ctx.repository_set("stats", json!({
        "total_users": 1234,
        "active_sessions": 42
    }));
    
    // View-specific data
    let data = json!({
        "title": "Dashboard",
        "recent_activity": ["Login", "Posted comment", "Updated profile"]
    });
    
    ctx.view("/dashboard/index", data)
}
```

**Multiple Views in One Handler**
```rust
async fn dashboard_with_sidebar(ctx: &mut Context) -> Result<()> {
    // Set repository data once - available to all views in this handler
    ctx.repository_set("user", get_current_user()?);
    ctx.repository_set("notifications", get_notifications()?);
    ctx.repository_set("theme", "dark");
    
    // Conditional rendering - all views have access to repository
    if is_mobile_device(&ctx) {
        // Mobile view also has access to repository data
        return ctx.view("/dashboard/mobile", json!({
            "stats": get_stats()?
        }));
    }
    
    // Desktop view also has access to the same repository data
    ctx.view("/dashboard/desktop", json!({
        "stats": get_stats()?,
        "charts": get_charts()?
    }))
}
```

**Use Cases for Repository**
- Data needed by all views rendered in the same handler
- User information and permissions for the current handler
- UI state (theme, layout) for views in this handler
- Temporary data that shouldn't be in the main view data
- Avoiding repetition when calling multiple views

**Note:** The repository lives only for the handler function's execution and is cleared when the handler returns. For application-wide data that persists across all requests and is accessible throughout the entire application, use the APP global repository system.

#### Flash Messages (one-time messages)
```rust
// Standard convenience methods
ctx.flash_success("User created successfully!");
ctx.flash_error("Invalid credentials");
ctx.flash_info("Please check your email");

// Generic flash setter for custom keys
ctx.flash("warning_msg", "This is a warning")?;
ctx.flash("user_level", 42)?;
ctx.flash("notification", json!({"text": "You have messages", "count": 5}))?;
ctx.flash("items", vec!["one", "two", "three"])?;

// Manual flash management
ctx.flash_clear();                    // Clear all flash messages
ctx.flash_clear_key("error_msg");     // Clear specific flash message

// Flash messages automatically appear in views via @{flash.success_msg}, @{flash.custom_key}, etc.
```

#### Client Information
```rust
// Client details
let ip = ctx.ip();                    // Client IP address
let user_agent = ctx.user_agent();    // Browser/client info
let is_mobile = ctx.is_mobile();      // Mobile device detection
let is_ajax = ctx.is_xhr();           // AJAX request detection
let language = ctx.language();        // Preferred language
```

## Controller Registration

### Manual Registration

For simple applications or when you need precise control:

```rust
use rustf::prelude::*;

mod controllers {
    pub mod home;
    pub mod auth;
    pub mod users;
}

#[tokio::main]
async fn main() -> Result<()> {
    let app = RustF::new()
        .controllers({
            let mut routes = Vec::new();
            routes.extend(controllers::home::install());
            routes.extend(controllers::auth::install());
            routes.extend(controllers::users::install());
            routes
        });
        
    app.start().await
}
```

### Auto-Discovery (Recommended)

For larger applications, use automatic controller discovery:

```rust
use rustf::prelude::*;

// The #[rustf::auto_discover] attribute automatically:
// 1. Scans src/controllers/*.rs files
// 2. Generates module declarations  
// 3. Creates controller registration code
#[rustf::auto_discover]
#[tokio::main]
async fn main() -> Result<()> {
    let app = RustF::new()
        .controllers(auto_controllers!())  // Registers all discovered controllers
        .models(auto_models!())           // Also discovers models
        .middleware_from(auto_middleware!()); // And middleware
        
    app.start().await
}
```

**Auto-discovery Process:**
1. **Compile-time scanning** - Framework scans `src/controllers/*.rs` at build time
2. **Module generation** - Generates `_controllers.rs` with module declarations
3. **Registration** - `auto_controllers!()` macro returns all discovered routes

**Requirements for Auto-Discovery:**
- Enable `auto-discovery` feature in `Cargo.toml`
- Each controller file must have `pub fn install() -> Vec<Route>`
- Controller files must be in `src/controllers/` directory
- Use `#[rustf::auto_discover]` attribute on main function

## Complete Controller Examples

### Simple Home Controller

```rust
// src/controllers/home.rs
use rustf::prelude::*;

pub fn install() -> Vec<Route> {
    routes![
        GET "/" => index,
        GET "/about" => about,
        GET "/contact" => contact,
    ]
}

async fn index(ctx: &mut Context) -> Result<()> {
    let data = json!({
        "title": "Welcome to RustF",
        "message": "Your application is running successfully!",
        "features": [
            "Auto-discovery for controllers",
            "Template engine with layouts", 
            "Session management",
            "Built-in security features"
        ]
    });
    
    ctx.view("/home/index", data)
}

async fn about(ctx: &mut Context) -> Result<()> {
    let data = json!({
        "title": "About",
        "description": "Built with RustF framework - an AI-friendly MVC framework for Rust",
        "version": "1.0.0"
    });
    
    ctx.view("/home/about", data)
}

async fn contact(ctx: &mut Context) -> Result<()> {
    ctx.view("/home/contact", json!({
        "title": "Contact Us",
        "email": "info@example.com"
    }))
}
```

### Authentication Controller with Repository

```rust
// src/controllers/auth.rs  
use rustf::prelude::*;
use serde::Deserialize;

pub fn install() -> Vec<Route> {
    routes![
        GET  "/auth/login"    => view_login,
        POST "/auth/login"    => do_login,
        GET  "/auth/logout"   => do_logout,
        GET  "/auth/register" => view_register,
        POST "/auth/register" => do_register,
    ]
}

// Struct for form validation (optional)
#[derive(Deserialize)]
struct LoginForm {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct RegisterForm {
    email: String,
    password: String,
    name: String,
}

async fn view_login(ctx: &mut Context) -> Result<()> {
    // Set repository data for this request
    ctx.repository_set("page_type", "auth")
       .repository_set("show_social_login", true)
       .repository_set("providers", json!(["google", "github"]));
    
    // Use empty layout for login page
    ctx.layout("")
       .view("/auth/login", json!({
           "title": "Login",
           "debug": cfg!(debug_assertions) // Show test credentials in debug
       }))
}

async fn do_login(ctx: &mut Context) -> Result<()> {
    // Parse form data into typed structure
    let form: LoginForm = ctx.body_form_typed()?;

    // Input validation
    if !form.email.contains('@') || form.password.is_empty() {
        ctx.flash_error("Please provide a valid email and password");
        return ctx.redirect("/auth/login");
    }

    // Authentication logic (use proper password hashing in production)
    if form.email == "admin@example.com" && form.password == "password" {
        ctx.session_set("user", json!({
            "id": 1,
            "email": form.email,
            "name": "Admin User",
            "role": "admin"
        }))?;

        ctx.flash_success("Login successful!");
        ctx.redirect("/dashboard")
    } else {
        ctx.flash_error("Invalid email or password");
        ctx.redirect("/auth/login")
    }
}

async fn do_logout(ctx: &mut Context) -> Result<()> {
    ctx.session_remove("user");
    ctx.flash_info("You have been logged out successfully");
    ctx.redirect("/auth/login")
}

async fn view_register(ctx: &mut Context) -> Result<()> {
    ctx.layout("")
       .view("/auth/register", json!({
           "title": "Create Account"
       }))
}

async fn do_register(ctx: &mut Context) -> Result<()> {
    // Parse form data into typed structure
    let form: RegisterForm = ctx.body_form_typed()?;

    // Validation
    if !form.email.contains('@') {
        ctx.flash_error("Please provide a valid email address");
        return ctx.redirect("/auth/register");
    }

    if form.password.len() < 8 {
        ctx.flash_error("Password must be at least 8 characters long");
        return ctx.redirect("/auth/register");
    }

    if form.name.trim().is_empty() {
        ctx.flash_error("Please provide your name");
        return ctx.redirect("/auth/register");
    }

    // Check if user already exists (simplified)
    // In real app: check database
    if form.email == "admin@example.com" {
        ctx.flash_error("User already exists");
        return ctx.redirect("/auth/register");
    }

    // Create user (in real app: hash password, save to database)
    ctx.flash_success(&format!("Account created successfully for {}! You can now log in.", form.name));
    ctx.redirect("/auth/login")
}
```

### Form Handling with Typed Parsing

RustF provides `body_form_typed<T>()` for automatic form deserialization into Rust structures, significantly reducing boilerplate code.

#### Basic Form Parsing

```rust
use rustf::prelude::*;
use serde::Deserialize;

// Define your form structure
#[derive(Deserialize)]
struct ContactForm {
    name: String,
    email: String,
    message: String,
}

pub fn install() -> Vec<Route> {
    routes![
        GET  "/contact" => view_contact,
        POST "/contact" => submit_contact,
    ]
}

async fn view_contact(ctx: &mut Context) -> Result<()> {
    ctx.view("/contact", json!({"title": "Contact Us"}))
}

async fn submit_contact(ctx: &mut Context) -> Result<()> {
    // Parse form directly into typed structure
    let form: ContactForm = ctx.body_form_typed()?;

    // Validate
    if !form.email.contains('@') {
        ctx.flash_error("Please provide a valid email address");
        return ctx.redirect("/contact");
    }

    if form.message.trim().is_empty() {
        ctx.flash_error("Message cannot be empty");
        return ctx.redirect("/contact");
    }

    // Process form data
    log::info!("Contact form from {}: {}", form.name, form.email);

    ctx.flash_success("Thank you! We'll get back to you soon.");
    ctx.redirect("/")
}
```

#### Optional Fields

Use `Option<T>` for optional form fields:

```rust
#[derive(Deserialize)]
struct ProfileForm {
    name: String,              // Required
    bio: Option<String>,       // Optional
    website: Option<String>,   // Optional
    age: Option<i32>,          // Optional number
    newsletter: Option<bool>,  // Optional checkbox
}

async fn update_profile(ctx: &mut Context) -> Result<()> {
    let form: ProfileForm = ctx.body_form_typed()?;

    // Required field is always present
    let name = form.name;

    // Optional fields can be None
    if let Some(bio) = form.bio {
        log::info!("User bio: {}", bio);
    }

    // Provide defaults for optional fields
    let website = form.website.unwrap_or_else(|| "Not provided".to_string());
    let age = form.age.unwrap_or(0);
    let newsletter = form.newsletter.unwrap_or(false);

    ctx.flash_success("Profile updated!");
    ctx.redirect("/profile")
}
```

#### Working with Arrays (Multiple Select / Checkboxes)

Handle multiple values using `Vec<T>`:

```rust
#[derive(Deserialize)]
struct PreferencesForm {
    username: String,
    interests: Vec<String>,      // Multiple checkboxes
    languages: Vec<String>,       // Multiple select
    notifications: Option<Vec<String>>, // Optional multiple
}

async fn save_preferences(ctx: &mut Context) -> Result<()> {
    let form: PreferencesForm = ctx.body_form_typed()?;

    // Handle multiple values
    log::info!("User interests: {:?}", form.interests);
    log::info!("Languages: {:?}", form.languages);

    // Optional arrays
    if let Some(notif) = form.notifications {
        log::info!("Notification preferences: {:?}", notif);
    }

    ctx.flash_success("Preferences saved!");
    ctx.redirect("/settings")
}
```

**HTML Form Example:**
```html
<form method="POST">
    <input name="username" value="john_doe" />

    <!-- Multiple checkboxes with same name -->
    <input type="checkbox" name="interests" value="sports" checked />
    <input type="checkbox" name="interests" value="music" checked />
    <input type="checkbox" name="interests" value="travel" />

    <!-- Multiple select -->
    <select name="languages" multiple>
        <option value="en" selected>English</option>
        <option value="fr" selected>French</option>
        <option value="es">Spanish</option>
    </select>
</form>
```

#### Post-Processing Transformations

Apply transformations after parsing:

```rust
#[derive(Deserialize)]
struct CountryForm {
    code: String,
    name: String,
    native_name: Option<String>,
    timezone: Option<String>,
}

async fn save_country(ctx: &mut Context) -> Result<()> {
    let mut form: CountryForm = ctx.body_form_typed()?;

    // Apply transformations
    form.code = form.code.trim().to_uppercase();
    form.name = form.name.trim().to_string();

    // Transform optional fields
    form.native_name = form.native_name
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    form.timezone = form.timezone
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // Validate transformed data
    if form.code.len() != 2 {
        ctx.flash_error("Country code must be exactly 2 characters");
        return ctx.redirect("/countries/new");
    }

    // Save to database...

    ctx.flash_success(&format!("Country {} created!", form.name));
    ctx.redirect("/countries")
}
```

#### Nested Structures

Handle complex nested forms:

```rust
#[derive(Deserialize)]
struct Address {
    street: String,
    city: String,
    country: String,
    postal_code: String,
}

#[derive(Deserialize)]
struct UserForm {
    name: String,
    email: String,
    address: Address,  // Nested structure
}

async fn create_user(ctx: &mut Context) -> Result<()> {
    let form: UserForm = ctx.body_form_typed()?;

    // Access nested data
    log::info!("User: {}", form.name);
    log::info!("Address: {}, {}", form.address.city, form.address.country);

    ctx.json(json!({
        "success": true,
        "user": form
    }))
}
```

**HTML Form Example:**
```html
<form method="POST">
    <input name="name" value="John Doe" />
    <input name="email" value="john@example.com" />

    <!-- Nested fields use dot notation -->
    <input name="address.street" value="123 Main St" />
    <input name="address.city" value="New York" />
    <input name="address.country" value="USA" />
    <input name="address.postal_code" value="10001" />
</form>
```

#### Error Handling

Handle parsing errors gracefully:

```rust
async fn safe_form_handler(ctx: &mut Context) -> Result<()> {
    // Parse form with error handling
    let form: ContactForm = match ctx.body_form_typed() {
        Ok(f) => f,
        Err(e) => {
            log::error!("Form parsing error: {}", e);
            ctx.flash_error("Invalid form data. Please check your input.");
            return ctx.redirect("/contact");
        }
    };

    // Process valid form...
    ctx.flash_success("Form submitted successfully!");
    ctx.redirect("/")
}
```

#### Comparison: Three Approaches

```rust
// ❌ Approach 1: Manual (verbose, error-prone)
async fn manual_approach(ctx: &mut Context) -> Result<()> {
    let form_data = ctx.body_form()?;
    let name = form_data.get("name").unwrap_or(&String::new()).clone();
    let email = form_data.get("email").unwrap_or(&String::new()).clone();
    let age = form_data.get("age").unwrap_or(&String::new()).parse::<i32>().unwrap_or(0);
    // ... lots of repetitive code
}

// ⚠️ Approach 2: Field helpers (good for simple forms)
async fn field_helpers_approach(ctx: &mut Context) -> Result<()> {
    let name = ctx.str_body("name")?;
    let email = ctx.str_body("email")?;
    let age = ctx.int_body("age")?;
    // Good for 2-3 fields, becomes verbose with many fields
}

// ✅ Approach 3: Typed parsing (recommended for complex forms)
#[derive(Deserialize)]
struct UserForm {
    name: String,
    email: String,
    age: i32,
}

async fn typed_approach(ctx: &mut Context) -> Result<()> {
    let form: UserForm = ctx.body_form_typed()?;
    // Clean, type-safe, and concise!
}
```

#### When to Use Each Method

**Use `body_form_typed<T>()`** when:
- ✅ Form has 4+ fields
- ✅ You need type safety
- ✅ You have nested data structures
- ✅ You want to reuse form structures
- ✅ You need to pass form data to other functions

**Use individual field helpers** (`str_body()`, etc.) when:
- ✅ Form has 1-3 simple fields
- ✅ You need immediate validation
- ✅ Quick prototyping

**Use manual `body_form()`** when:
- ✅ You need maximum flexibility
- ✅ Dynamic field names
- ✅ Custom parsing logic

### RESTful API Controller

```rust
// src/controllers/api.rs
use rustf::prelude::*;
use serde::{Serialize, Deserialize};

pub fn install() -> Vec<Route> {
    routes![
        // User management API
        GET    "/api/users"          => list_users,
        GET    "/api/users/{id}"      => get_user,
        POST   "/api/users"          => create_user,
        PUT    "/api/users/{id}"      => update_user,
        DELETE "/api/users/{id}"      => delete_user,
        
        // Additional endpoints
        GET    "/api/users/search"   => search_users,
        GET    "/api/health"         => health_check,
    ]
}

#[derive(Serialize, Deserialize)]
struct User {
    id: i32,
    name: String,
    email: String,
    created_at: String,
    is_active: bool,
}

#[derive(Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

#[derive(Deserialize)] 
struct UpdateUserRequest {
    name: Option<String>,
    email: Option<String>,
    is_active: Option<bool>,
}

async fn list_users(ctx: &mut Context) -> Result<()> {
    // Parse query parameters for pagination
    let page = ctx.query("page").unwrap_or("1").parse::<i32>().unwrap_or(1);
    let limit = ctx.query("limit").unwrap_or("10").parse::<i32>().unwrap_or(10);
    
    let users = vec![
        User {
            id: 1,
            name: "Alice Johnson".to_string(),
            email: "alice@example.com".to_string(),
            created_at: "2024-01-15T10:00:00Z".to_string(),
            is_active: true,
        },
        User {
            id: 2,
            name: "Bob Smith".to_string(),
            email: "bob@example.com".to_string(),
            created_at: "2024-02-20T14:30:00Z".to_string(),
            is_active: true,
        },
    ];
    
    ctx.json(json!({
        "users": users,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": users.len(),
            "total_pages": 1
        }
    }))
}

async fn get_user(ctx: &mut Context) -> Result<()> {
    let user_id = ctx.param("id").unwrap_or("0");
    
    match user_id.parse::<i32>() {
        Ok(id) if id > 0 => {
            let user = User {
                id,
                name: "Sample User".to_string(),
                email: "user@example.com".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                is_active: true,
            };
            
            ctx.json(json!({
                "success": true,
                "user": user
            }))
        }
        _ => {
            ctx.throw400(Some("Invalid user ID"))
        }
    }
}

async fn create_user(ctx: &mut Context) -> Result<()> {
    let request: CreateUserRequest = ctx.body_json()?;
    
    // Validation
    if request.name.trim().is_empty() {
        return ctx.throw400(Some("Name is required"));
    }
    
    if !request.email.contains('@') {
        return ctx.throw400(Some("Valid email is required"));
    }
    
    // Create user (in real app: save to database)
    let user = User {
        id: 3, // Would be generated by database
        name: request.name,
        email: request.email,
        created_at: chrono::Utc::now().to_rfc3339(),
        is_active: true,
    };
    
    ctx.json(json!({
        "success": true,
        "message": "User created successfully",
        "user": user
    }))
}

async fn update_user(ctx: &mut Context) -> Result<()> {
    let user_id = ctx.param("id").unwrap_or("0");
    let request: UpdateUserRequest = ctx.body_json()?;
    
    match user_id.parse::<i32>() {
        Ok(id) if id > 0 => {
            // In real app: update in database
            ctx.json(json!({
                "success": true,
                "message": "User updated successfully",
                "user_id": id,
                "updated_fields": request
            }))
        }
        _ => {
            ctx.throw400(Some("Invalid user ID"))
        }
    }
}

async fn delete_user(ctx: &mut Context) -> Result<()> {
    let user_id = ctx.param("id").unwrap_or("0");
    
    match user_id.parse::<i32>() {
        Ok(id) if id > 0 => {
            // In real app: delete from database
            ctx.json(json!({
                "success": true,
                "message": "User deleted successfully",
                "deleted_user_id": id
            }))
        }
        _ => {
            ctx.throw400(Some("Invalid user ID"))
        }
    }
}

async fn search_users(ctx: &mut Context) -> Result<()> {
    let query = ctx.query("q").unwrap_or("");
    
    if query.is_empty() {
        return ctx.throw400(Some("Search query is required"));
    }
    
    // In real app: search database
    let results = vec![
        User {
            id: 1,
            name: "Alice Johnson".to_string(),
            email: "alice@example.com".to_string(),
            created_at: "2024-01-15T10:00:00Z".to_string(),
            is_active: true,
        }
    ];
    
    ctx.json(json!({
        "success": true,
        "query": query,
        "results": results,
        "count": results.len()
    }))
}

async fn health_check(ctx: &mut Context) -> Result<()> {
    ctx.json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": "1.0.0"
    }))
}
```

## Advanced Features

### URL Parameters

Capture dynamic parts of URLs using the `{parameter}` syntax:

```rust
routes![
    GET "/users/{id}" => get_user,
    GET "/users/{user_id}/posts/{post_id}" => get_user_post,
    GET "/posts/{post_id}/comments/{comment_id}" => get_comment,
    GET "/files/{category}/{filename}" => download_file,
]

async fn get_user(ctx: &mut Context) -> Result<()> {
    let id = ctx.param("id").unwrap_or("0");
    
    // Validate and parse parameter
    match id.parse::<i32>() {
        Ok(user_id) if user_id > 0 => {
            // Use the validated user_id
            ctx.json(json!({"user_id": user_id, "name": "User Name"}))
        }
        _ => ctx.throw400(Some("Invalid user ID"))
    }
}

async fn get_user_post(ctx: &mut Context) -> Result<()> {
    let user_id = ctx.param("user_id").unwrap_or("0");
    let post_id = ctx.param("post_id").unwrap_or("0");
    
    ctx.json(json!({
        "user_id": user_id,
        "post_id": post_id,
        "title": "Sample Post"
    }))
}

async fn download_file(ctx: &mut Context) -> Result<()> {
    let category = ctx.param("category").unwrap_or("general");
    let filename = ctx.param("filename").unwrap_or("file.txt");
    
    // Security: validate file path (prevent directory traversal)
    if filename.contains("..") || filename.contains('/') {
        return ctx.throw403(Some("Invalid filename"));
    }
    
    let file_path = format!("uploads/{}/{}", category, filename);
    ctx.file_download(&file_path, Some(filename))
}
```

### Query Parameters

```rust
// URL: /search?q=rust&category=programming&page=2
async fn search(ctx: &mut Context) -> Result<()> {
    let query = ctx.query("q").unwrap_or("");
    let category = ctx.query("category").unwrap_or("all");
    let page = ctx.query("page").unwrap_or("1")
        .parse::<i32>().unwrap_or(1);
    
    if query.is_empty() {
        return ctx.throw400(Some("Search query is required"));
    }
    
    // Perform search with parameters
    ctx.json(json!({
        "query": query,
        "category": category,
        "page": page,
        "results": [/* search results */]
    }))
}
```

### File Handling

```rust
routes![
    GET  "/upload" => upload_form,
    POST "/upload" => handle_upload,
    GET  "/files/{filename}" => serve_file,
]

async fn upload_form(ctx: &mut Context) -> Result<()> {
    ctx.view("/upload", json!({"title": "File Upload"}))
}

async fn handle_upload(ctx: &mut Context) -> Result<()> {
    // Get uploaded files
    let files = ctx.files()?;
    
    if files.is_empty() {
        ctx.flash_error("No files were uploaded");
        return ctx.redirect("/upload");
    }
    
    // Process first uploaded file
    if let Some(file) = ctx.file("document")? {
        // Validate file type
        let allowed_types = ["pdf", "doc", "docx", "txt"];
        let file_ext = file.filename
            .as_ref()
            .and_then(|name| name.split('.').last())
            .unwrap_or("");
            
        if !allowed_types.contains(&file_ext) {
            ctx.flash_error("Only PDF, DOC, DOCX, and TXT files are allowed");
            return ctx.redirect("/upload");
        }
        
        // Save file (in real app: save to disk/cloud storage)
        let filename = format!("upload_{}.{}", U::guid(), file_ext);
        
        ctx.flash_success(&format!("File '{}' uploaded successfully as {}", 
            file.filename.as_ref().unwrap_or(&"unknown".to_string()), filename));
    }
    
    ctx.redirect("/upload")
}

async fn serve_file(ctx: &mut Context) -> Result<()> {
    let filename = ctx.param("filename").unwrap_or("missing");
    let file_path = format!("uploads/{}", filename);
    
    // Security check
    if filename.contains("..") {
        return ctx.throw403(Some("Access denied"));
    }
    
    ctx.file_download(&file_path, Some(filename))
}
```

### Middleware Integration

Controllers work seamlessly with middleware:

```rust
// Authentication middleware can protect routes
routes![
    GET "/admin/dashboard" => admin_dashboard,  // Protected by auth middleware
    GET "/admin/users"     => admin_users,     // Protected by auth middleware
    GET "/public/info"     => public_info,     // Not protected
]

async fn admin_dashboard(ctx: &mut Context) -> Result<()> {
    // This handler only runs if auth middleware allows it
    let user: Value = ctx.session_get("user").unwrap_or_default();
    
    ctx.view("/admin/dashboard", json!({
        "title": "Admin Dashboard",
        "user": user
    }))
}
```

## Error Handling Best Practices

### Structured Error Handling

```rust
async fn robust_handler(ctx: &mut Context) -> Result<()> {
    // Parse form data with proper error handling
    let form_data = match ctx.body_form() {
        Ok(data) => data,
        Err(e) => {
            log::error!("Failed to parse form data: {}", e);
            return ctx.throw400(Some("Invalid form data"));
        }
    };
    
    // Validate required fields
    let email = match form_data.get("email") {
        Some(email) if !email.is_empty() => email,
        _ => {
            ctx.flash_error("Email is required");
            return ctx.redirect("/form");
        }
    };
    
    // Business logic validation
    if !email.contains('@') {
        ctx.flash_error("Please provide a valid email address");
        return ctx.redirect("/form");
    }
    
    // Success path
    ctx.flash_success("Form processed successfully!");
    ctx.redirect("/success")
}
```

### API Error Responses

```rust
async fn api_handler(ctx: &mut Context) -> Result<()> {
    // Parse JSON with error handling
    let request_data: Value = match ctx.body_json() {
        Ok(data) => data,
        Err(_) => {
            return ctx.json(json!({
                "error": "Invalid JSON",
                "code": "INVALID_JSON",
                "status": 400
            }));
        }
    };
    
    // Validate required fields
    let name = match request_data["name"].as_str() {
        Some(name) if !name.trim().is_empty() => name.trim(),
        _ => {
            return ctx.json(json!({
                "error": "Name is required",
                "code": "MISSING_NAME", 
                "status": 400
            }));
        }
    };
    
    // Success response
    ctx.json(json!({
        "success": true,
        "message": "Data processed successfully",
        "data": {"name": name}
    }))
}
```

### Using HTTP Error Methods

```rust
async fn comprehensive_error_handler(ctx: &mut Context) -> Result<()> {
    let action = ctx.param("action").unwrap_or("");
    
    match action {
        "unauthorized" => ctx.throw401(Some("Please log in")),
        "forbidden" => ctx.throw403(Some("Access denied")), 
        "notfound" => ctx.throw404(Some("Resource not found")),
        "conflict" => ctx.throw409(Some("Resource already exists")),
        "server_error" => ctx.throw500(Some("Internal server error")),
        "not_implemented" => ctx.throw501(Some("Feature not implemented")),
        _ => ctx.throw400(Some("Invalid action"))
    }
}
```

## Framework Integration

### How Controllers Work in RustF

1. **Route Registration** - Controllers return `Vec<Route>` from their `install()` function
2. **Request Matching** - Framework matches incoming requests to routes using method and path
3. **Context Creation** - Framework creates a `Context` with request data, session, and config
4. **Middleware Chain** - Request passes through middleware before reaching controller
5. **Handler Execution** - Controller handler processes request and returns response
6. **Response Processing** - Framework sends response back to client

### Application Lifecycle

```rust
// 1. Application setup
let app = RustF::new()
    .controllers(auto_controllers!())  // Register all controllers
    .middleware("auth", AuthMiddleware::new());  // Add middleware

// 2. Server startup
app.start().await;  // Starts HTTP server

// 3. Request processing
// HTTP Request -> Middleware Chain -> Controller Handler -> HTTP Response
```

## Best Practices

### 1. Controller Organization

```rust
// ✅ Good: Group related functionality
// src/controllers/auth.rs - All authentication
// src/controllers/users.rs - All user management
// src/controllers/api/users.rs - API endpoints for users

// ❌ Bad: Mixed functionality in one controller
// src/controllers/everything.rs - Login, users, posts, etc.
```

### 2. Naming Conventions

```rust
// ✅ Good: Descriptive, consistent names
routes![
    GET  "/login"  => view_login,     // Shows form
    POST "/login"  => do_login,       // Processes form
    GET  "/users"  => list_users,     // Lists resources
    GET  "/users/{id}" => show_user,   // Shows single resource
    POST "/users"  => create_user,    // Creates resource
    PUT  "/users/{id}" => update_user, // Updates resource
    DELETE "/users/{id}" => delete_user, // Deletes resource
]

// ❌ Bad: Generic, unclear names
routes![
    GET "/login" => handler1,
    POST "/login" => handler2,
    GET "/users" => users,
]
```

### 3. Input Validation

```rust
// ✅ Good: Comprehensive validation
async fn create_user(ctx: &mut Context) -> Result<()> {
    let form_data = ctx.body_form()?;
    
    // Validate required fields
    let email = match form_data.get("email") {
        Some(email) if !email.trim().is_empty() => email.trim(),
        _ => {
            ctx.flash_error("Email is required");
            return ctx.redirect("/users/new");
        }
    };
    
    // Validate format
    if !email.contains('@') {
        ctx.flash_error("Please provide a valid email address");
        return ctx.redirect("/users/new");
    }
    
    // Continue with processing...
}

// ❌ Bad: No validation
async fn create_user(ctx: &mut Context) -> Result<()> {
    let form_data = ctx.body_form()?;
    let email = form_data.get("email").unwrap(); // Can panic!
    // Save without validation...
}
```

### 4. Error Handling

```rust
// ✅ Good: Proper error handling with user feedback
async fn process_payment(ctx: &mut Context) -> Result<()> {
    let amount_str = ctx.query("amount").unwrap_or("0");
    
    let amount = match amount_str.parse::<f64>() {
        Ok(amt) if amt > 0.0 => amt,
        _ => {
            ctx.flash_error("Invalid payment amount");
            return ctx.redirect("/payment");
        }
    };
    
    // Process payment with proper error handling
    match process_payment_logic(amount).await {
        Ok(receipt) => {
            ctx.flash_success("Payment processed successfully!");
            ctx.redirect(&format!("/receipt/{}", receipt.id))
        }
        Err(e) => {
            log::error!("Payment failed: {}", e);
            ctx.flash_error("Payment processing failed. Please try again.");
            ctx.redirect("/payment")
        }
    }
}
```

### 5. Response Patterns

```rust
// ✅ Good: Consistent response patterns
routes![
    GET  "/users"     => list_users,    // Returns view or JSON list
    POST "/users"     => create_user,   // Redirects on success, back on error
    GET  "/api/users" => api_list_users, // Always returns JSON
]

async fn list_users(ctx: &mut Context) -> Result<()> {
    let users = get_users().await?;
    ctx.view("/users/index", json!({"users": users}))
}

async fn create_user(ctx: &mut Context) -> Result<()> {
    // Validation...
    // Creation...
    
    ctx.flash_success("User created successfully!");
    ctx.redirect("/users")
}

async fn api_list_users(ctx: &mut Context) -> Result<()> {
    let users = get_users().await?;
    ctx.json(json!({
        "success": true,
        "users": users,
        "count": users.len()
    }))
}
```

### 6. Security Considerations

```rust
// ✅ Good: Security-aware controller
async fn download_file(ctx: &mut Context) -> Result<()> {
    let filename = ctx.param("filename").unwrap_or("");
    
    // Prevent directory traversal attacks
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return ctx.throw403(Some("Invalid filename"));
    }
    
    // Validate file exists and user has access
    let file_path = format!("uploads/{}", filename);
    if !std::path::Path::new(&file_path).exists() {
        return ctx.throw404(Some("File not found"));
    }
    
    // Check user permissions
    if let Some(user) = ctx.session_get::<Value>("user") {
        ctx.file_download(&file_path, Some(filename))
    } else {
        ctx.throw401(Some("Login required"))
    }
}
```

### 7. Testing Controllers

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rustf::test_helpers::*;
    
    #[tokio::test]
    async fn test_login_success() {
        let ctx = create_test_context()
            .with_form_data([
                ("email", "test@example.com"),
                ("password", "password")
            ]);
            
        let response = do_login(ctx).await.unwrap();
        assert_eq!(response.status_code(), 302); // Redirect
    }
    
    #[tokio::test]
    async fn test_login_invalid_email() {
        let ctx = create_test_context()
            .with_form_data([
                ("email", "invalid-email"),
                ("password", "password")
            ]);
            
        let response = do_login(ctx).await.unwrap();
        assert_eq!(response.status_code(), 302); // Redirect back to form
    }
}
```

## File Organization

### Recommended Project Structure

```
src/
├── controllers/
│   ├── home.rs          # Home page, about, contact
│   ├── auth.rs          # Authentication (login, register, logout)
│   ├── users.rs         # User management (CRUD operations)
│   ├── posts.rs         # Blog posts or content
│   ├── api/
│   │   ├── mod.rs        # API module declaration
│   │   ├── users.rs      # User API endpoints
│   │   ├── posts.rs      # Posts API endpoints
│   │   └── auth.rs       # Authentication API
│   ├── admin/
│   │   ├── mod.rs        # Admin module declaration
│   │   ├── dashboard.rs  # Admin dashboard
│   │   ├── users.rs      # Admin user management
│   │   └── settings.rs   # System settings
│   └── errors.rs         # Error pages (404, 500, etc.)
├── _controllers.rs       # Auto-generated (DO NOT EDIT)
├── models/              # Database models
├── modules/             # Business logic modules
├── middleware/          # Custom middleware
└── main.rs              # Application entry point
```

### Auto-Generated Files

- `_controllers.rs` - Generated by `#[rustf::auto_discover]` for IDE support
- **DO NOT EDIT** auto-generated files manually
- Regenerated on each build when controllers change

## Summary

RustF's controller system provides:

✅ **Clean Architecture** - Separate HTTP handling from business logic
✅ **Auto-Discovery** - Automatic controller registration at compile time  
✅ **Type Safety** - Compile-time validation of routes and handlers
✅ **Total.js Familiarity** - Familiar patterns for web developers
✅ **Flexible Organization** - Organize controllers by feature or API version
✅ **Rich Context API** - Comprehensive request/response handling
✅ **Error Handling** - Built-in HTTP error responses
✅ **Security Features** - Session management, input validation helpers
✅ **Testing Support** - Easy to unit test individual handlers

The controller system strikes a balance between simplicity and power, making it easy to build maintainable web applications while providing all the features needed for modern web development.

## Related Topics

- [Views & Templates](views.md) - Learn how to render templates in controllers
- [Middleware](middleware.md) - Add request/response processing to your routes
- [Sessions](sessions.md) - Manage user sessions in your controllers
- [Error Handling](error-handling.md) - Handle errors gracefully
- [Database Integration](database.md) - Access databases from controllers
- [API Reference: Context](../api-reference/context.md) - Complete Context API documentation
- [Examples: REST API](../examples/rest-api.md) - See controllers in action