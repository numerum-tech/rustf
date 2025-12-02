# Authentication Implementation

This guide shows how to implement authentication in a RustF application using sessions.

## Overview

We'll implement:
- User login
- User registration
- Session management
- Protected routes
- Logout

## Setup

### 1. Create Auth Controller

Create `src/controllers/auth.rs`:

```rust
use rustf::prelude::*;
use serde::Deserialize;

pub fn install() -> Vec<Route> {
    routes![
        GET  "/auth/login"    => view_login,
        POST "/auth/login"    => do_login,
        GET  "/auth/register" => view_register,
        POST "/auth/register" => do_register,
        POST "/auth/logout"   => do_logout,
    ]
}
```

### 2. Define Data Structures

```rust
#[derive(Deserialize)]
struct LoginForm {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct RegisterForm {
    name: String,
    email: String,
    password: String,
    password_confirm: String,
}
```

## Implementation

### Login View

```rust
async fn view_login(ctx: &mut Context) -> Result<()> {
    // Check if already logged in
    if ctx.has_session() {
        if let Some(session) = ctx.session() {
            if session.is_authenticated() {
                return ctx.redirect("/dashboard");
            }
        }
    }
    
    ctx.layout("")  // No layout for login page
       .view("/auth/login", json!({
           "title": "Login"
       }))
}
```

### Login Handler

```rust
async fn do_login(ctx: &mut Context) -> Result<()> {
    let form: LoginForm = ctx.body_form_typed()?;
    
    // Validation
    if form.email.trim().is_empty() {
        ctx.flash_error("Email is required")?;
        return ctx.redirect("/auth/login");
    }
    
    if form.password.is_empty() {
        ctx.flash_error("Password is required")?;
        return ctx.redirect("/auth/login");
    }
    
    // In a real app, verify credentials against database
    // This is a simplified example
    if form.email == "admin@example.com" && form.password == "password123" {
        // Get or create session
        let session = ctx.require_session()?;
        
        // Store user data in session
        ctx.session_set("user", json!({
            "id": 1,
            "email": form.email,
            "name": "Admin User",
            "role": "admin"
        }))?;
        
        // Mark as authenticated
        ctx.login(1)?;
        
        ctx.flash_success("Login successful!")?;
        ctx.redirect("/dashboard")
    } else {
        ctx.flash_error("Invalid email or password")?;
        ctx.redirect("/auth/login")
    }
}
```

### Registration View

```rust
async fn view_register(ctx: &mut Context) -> Result<()> {
    ctx.layout("")
       .view("/auth/register", json!({
           "title": "Create Account"
       }))
}
```

### Registration Handler

```rust
async fn do_register(ctx: &mut Context) -> Result<()> {
    let form: RegisterForm = ctx.body_form_typed()?;
    
    // Validation
    if form.name.trim().is_empty() {
        ctx.flash_error("Name is required")?;
        return ctx.redirect("/auth/register");
    }
    
    if form.email.trim().is_empty() || !form.email.contains('@') {
        ctx.flash_error("Valid email is required")?;
        return ctx.redirect("/auth/register");
    }
    
    if form.password.len() < 8 {
        ctx.flash_error("Password must be at least 8 characters")?;
        return ctx.redirect("/auth/register");
    }
    
    if form.password != form.password_confirm {
        ctx.flash_error("Passwords do not match")?;
        return ctx.redirect("/auth/register");
    }
    
    // In a real app, create user in database
    // Hash password before storing
    
    ctx.flash_success("Account created successfully! Please login.")?;
    ctx.redirect("/auth/login")
}
```

### Logout Handler

```rust
async fn do_logout(ctx: &mut Context) -> Result<()> {
    ctx.logout()?;
    ctx.flash_info("You have been logged out")?;
    ctx.redirect("/auth/login")
}
```

## Protected Routes

### Create Auth Middleware

Create `src/middleware/auth.rs`:

```rust
use rustf::prelude::*;

pub struct AuthMiddleware;

impl InboundMiddleware for AuthMiddleware {
    fn handle(&self, ctx: &mut Context) -> MiddlewareResult {
        // Check if route requires authentication
        let path = ctx.path();
        let protected_paths = ["/dashboard", "/profile", "/settings"];
        
        if protected_paths.iter().any(|p| path.starts_with(p)) {
            if let Some(session) = ctx.session() {
                if session.is_authenticated() {
                    return MiddlewareResult::Continue;
                }
            }
            
            // Not authenticated, redirect to login
            ctx.flash_error("Please login to access this page")?;
            if let Err(e) = ctx.redirect("/auth/login") {
                return MiddlewareResult::Error(e);
            }
            return MiddlewareResult::Stop;
        }
        
        MiddlewareResult::Continue
    }
}

pub fn install(registry: &mut MiddlewareRegistry) {
    registry.register("auth", AuthMiddleware);
}
```

### Register Middleware

In `src/main.rs`:

```rust
let app = RustF::new()
    .controllers(auto_controllers!())
    .middleware_from(auto_middleware!());
```

## Accessing User Data

### In Controllers

```rust
async fn dashboard(ctx: &mut Context) -> Result<()> {
    // Get user from session
    let user: Option<serde_json::Value> = ctx.session_get("user");
    
    if let Some(user_data) = user {
        ctx.repository_set("user", user_data.clone());
        ctx.view("/dashboard/index", json!({
            "title": "Dashboard"
        }))
    } else {
        ctx.throw401(Some("Not authenticated"))
    }
}
```

### In Templates

```html
<!-- views/dashboard/index.html -->
@{if repository.user}
    <h1>Welcome, @{repository.user.name}!</h1>
    <p>Email: @{repository.user.email}</p>
    <p>Role: @{repository.user.role}</p>
@{fi}
```

## Security Best Practices

1. **Password Hashing**: Always hash passwords before storing
   ```rust
   use bcrypt;
   let hashed = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
   ```

2. **Session Security**: Use secure, HTTP-only cookies in production
   ```toml
   [session]
   secure = true
   http_only = true
   ```

3. **CSRF Protection**: Enable CSRF protection - See [Security Guide](../guides/security.md)

4. **Rate Limiting**: Add rate limiting to login endpoints

5. **Input Validation**: Always validate and sanitize user input

## Next Steps

- Add password reset functionality
- Add email verification
- Add two-factor authentication
- Add role-based access control (RBAC)
- Integrate with OAuth providers


