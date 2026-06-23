# RustF CSRF Protection Guide

🔒 **Comprehensive Cross-Site Request Forgery Protection**

This guide covers RustF's automatic CSRF protection system, designed for both security and developer productivity with zero-configuration defaults and extensive customization options.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Automatic Protection](#automatic-protection)
- [Configuration](#configuration)
- [Route Exemptions](#route-exemptions)
- [Token Management](#token-management)
- [Context Integration](#context-integration)
- [Error Handling](#error-handling)
- [AJAX Integration](#ajax-integration)
- [Form Integration](#form-integration)
- [Advanced Usage](#advanced-usage)
- [Troubleshooting](#troubleshooting)
- [Best Practices](#best-practices)

## Overview

Cross-Site Request Forgery (CSRF) attacks trick users into performing unintended actions by exploiting their authenticated sessions. RustF provides comprehensive CSRF protection through:

### Key Features

✅ **One-Time Use Tokens** - Tokens are consumed after successful verification (prevents replay attacks)  
✅ **Token Expiration** - Tokens automatically expire after 1 hour  
✅ **Multiple Concurrent Tokens** - Support different token IDs for different forms  
✅ **Automatic HTTP Method Detection** - Only unsafe methods (POST, PUT, PATCH, DELETE) require validation  
✅ **Session Integration** - Tokens stored and validated via the session system  
✅ **Route Exemption Patterns** - Flexible wildcard and exact matching  
✅ **Multiple Token Sources** - Headers and form fields  
✅ **Smart Error Handling** - Context-aware responses (JSON for APIs, HTML for web)  
✅ **Zero Configuration** - Works out-of-the-box with sensible defaults  
✅ **Context Methods** - Convenient `verify_csrf()` and `generate_csrf()` methods  

### Protection Scope

**Protected Methods (Require CSRF token):**
- `POST` - Create operations
- `PUT` - Full resource updates  
- `PATCH` - Partial resource updates
- `DELETE` - Resource deletion

**Safe Methods (Bypass CSRF):**
- `GET` - Read operations
- `HEAD` - Header-only requests
- `OPTIONS` - CORS preflight requests

## Quick Start

### Basic Setup

Add CSRF protection with zero configuration:

Routes live in controllers (each controller exposes `pub fn install() -> Vec<Route>`),
and middleware is registered on the app builder with `.middleware_from(...)`:

```rust
// src/controllers/users.rs
use rustf::prelude::*;

pub fn install() -> Vec<Route> {
    routes![
        GET    "/users"      => list_users,    // Not protected (GET)
        POST   "/users"      => create_user,   // Protected
        PUT    "/users/{id}" => update_user,   // Protected
        DELETE "/users/{id}" => delete_user,   // Protected
    ]
}

async fn create_user(ctx: &mut Context) -> rustf::Result<()> {
    // CSRF already validated by the middleware
    let form_data = ctx.body_form()?;

    let user = create_user_record(&form_data).await?;
    ctx.json(json!({"message": "User created", "id": user.id}))
}
```

```rust
// src/main.rs
use rustf::prelude::*;
use rustf::security::CsrfMiddleware;

#[tokio::main]
async fn main() -> rustf::Result<()> {
    RustF::new()
        .controllers(auto_controllers!())
        // Enable CSRF protection (default configuration)
        .middleware_from(|registry| {
            registry.register_inbound("csrf", CsrfMiddleware::new());
        })
        .serve(Some("127.0.0.1:8080"))
        .await
}
```

### Default Behavior

With zero configuration, the CSRF middleware:
- Protects POST, PUT, PATCH, DELETE requests
- Exempts no routes by default
- Stores tokens in the session
- Returns appropriate error responses

## Automatic Protection

### HTTP Method-Based Protection

CSRF protection is applied automatically based on HTTP methods:

```rust
// In a controller's install():
routes![
    // These routes are automatically protected:
    POST   "/users"      => create_user,     // ✅ CSRF Required
    PUT    "/users/{id}" => update_user,      // ✅ CSRF Required
    PATCH  "/users/{id}" => partial_update,   // ✅ CSRF Required
    DELETE "/users/{id}" => delete_user,      // ✅ CSRF Required

    // These routes bypass CSRF protection:
    GET    "/users"      => list_users,       // ⏭️ CSRF Bypassed
    HEAD   "/users/{id}" => check_user,       // ⏭️ CSRF Bypassed
    OPTIONS "/users"     => cors_preflight,   // ⏭️ CSRF Bypassed
]
```

### Session Integration

CSRF tokens are automatically managed through the session system:

```rust
// Token lifecycle (handled automatically):
// 1. Generate token: stored in session["_csrf_token"]
// 2. Validate token: compare submitted vs stored
// 3. Token persistence: lives with the session
// 4. Token regeneration: on session regeneration
```

## Configuration

### Custom Configuration

Create a custom CSRF configuration for advanced scenarios:

```rust
use rustf::security::{CsrfMiddleware, CsrfConfig};

let csrf_config = CsrfConfig::new()
    // Route exemptions
    .exempt("/webhook/github")              // Specific webhook
    .exempt("/webhook/*")                   // All webhook routes  
    .exempt("/api/public/*")               // Public API endpoints
    .exempt("/uploads/process")             // File processing endpoint
    
    // Error handling
    .error_message("Security validation failed. Please refresh and try again.")
    .redirect_on_failure("/login")          // Redirect instead of error page
    
    // HTTP method customization
    .protect_method("CUSTOM")               // Add custom method protection
    .exempt_method("DELETE");               // Remove DELETE protection

let csrf_middleware = CsrfMiddleware::with_config(csrf_config);

// Register on the app builder:
RustF::new()
    .controllers(auto_controllers!())
    .middleware_from(move |registry| {
        registry.register_inbound("csrf", csrf_middleware.clone());
    });
```

### Configuration Builder Methods

| Method | Description | Example |
|--------|-------------|---------|
| `exempt(route)` | Add route exemption pattern | `.exempt("/api/*")` |
| `error_message(msg)` | Custom error message | `.error_message("Token expired")` |
| `redirect_on_failure(url)` | Redirect URL on failure | `.redirect_on_failure("/login")` |
| `disabled()` | Disable CSRF globally | `.disabled()` |
| `protect_method(method)` | Add protected HTTP method | `.protect_method("CUSTOM")` |
| `exempt_method(method)` | Remove method protection | `.exempt_method("PUT")` |

## Route Exemptions

### Pattern Matching

CSRF route exemptions support exact matches and a single trailing `/*`
prefix wildcard. A `*` anywhere other than the end (e.g. `/admin/*/public`)
is **not** supported and never matches.

```rust
let csrf_config = CsrfConfig::new()
    // Exact matches
    .exempt("/webhook")                     // Only /webhook
    .exempt("/public/upload")               // Only /public/upload

    // Trailing /* prefix wildcards
    .exempt("/api/*")                       // /api/users, /api/v1/posts, etc.
    .exempt("/webhook/*");                  // /webhook/github, /webhook/stripe, etc.
```

### Pattern Examples

| Pattern | Matches | Doesn't Match |
|---------|---------|---------------|
| `/api/*` | `/api/users`, `/api/v1/data` | `/api`, `/public/api` |
| `/webhook/github` | `/webhook/github` | `/webhook/github/push` |
| `/public/upload` | `/public/upload` | `/public/uploads` |
| `/admin/*/public` | *(nothing — mid-path `*` is unsupported)* | everything |

### Default Exemptions

`CsrfConfig::new()` (and `default()`) starts with an empty exemption list.
`.exempt(...)` appends routes to that list. So this:

```rust
let csrf_config = CsrfConfig::new()
    .exempt("/external/webhooks/*")
    .exempt("/integrations/*");
// exempt_routes is now ["/external/webhooks/*", "/integrations/*"]
```

To replace the full exemption list from configuration, set
`middleware.csrf.exempt_routes` in `config.toml`.

### Common Exemption Patterns

```rust
// Webhooks and integrations
.exempt("/webhook/*")
.exempt("/integration/*") 
.exempt("/callback/*")

// Public APIs  
.exempt("/api/public/*")
.exempt("/api/v1/status")
.exempt("/api/health")

// File uploads from external sources
.exempt("/upload/external/*")
.exempt("/cdn/callback")

// Payment gateways
.exempt("/payment/stripe/webhook")
.exempt("/payment/paypal/ipn")
```

## Token Management

### Automatic Token Generation

CSRF tokens are generated and managed automatically:

```rust
// Token generation happens automatically when:
// 1. First CSRF-protected request is made
// 2. ctx.generate_csrf() is called explicitly

async fn show_form(ctx: &mut Context) -> rustf::Result<()> {
    // Automatically generates and stores token in session.
    // generate_csrf takes Option<&str>; pass None for the default token id.
    ctx.generate_csrf(None)?;
    
    // Token is accessible in templates via @{csrf_token} and @{csrf}
    ctx.view("user/form", json!({
        "user": load_user_data().await?
    })).await
}
```

### Token Properties

- **Format**: Base64-encoded random bytes (32 bytes = ~44 characters)
- **Storage**: Session key `_csrf_token` (or custom ID) with JSON structure: `{"token": "...", "valid_to": timestamp}`
- **Lifetime**: 1 hour from generation (configurable)
- **Usage**: One-time use - consumed after successful verification
- **Security**: Cryptographically secure random generation

### Token Lifecycle

Tokens follow a secure lifecycle:

```rust
// 1. Generation - Token created with expiration
let token = ctx.generate_csrf(None)?;  // Default token
let custom = ctx.generate_csrf(Some("upload_csrf"))?;  // Custom ID

// 2. Verification - Token consumed (one-time use)
if ctx.verify_csrf(None)? {  // Default token
    // Token is valid and now removed from session
}

// 3. Expiration - Tokens expire after 1 hour
// Expired tokens are automatically removed on verification attempt

// 4. Session destruction - All tokens cleared
if let Some(session) = ctx.session() {
    session.destroy();  // All tokens removed
}
```

## Context Integration

### Context Methods

RustF provides convenient methods on the Context for CSRF operations:

```rust
async fn form_controller(ctx: &mut Context) -> rustf::Result<()> {
    // Generate CSRF token (stored in session); None = default token id
    let csrf_token = ctx.generate_csrf(None)?;
    
    // Manual verification (usually not needed due to middleware)
    if !ctx.verify_csrf(None)? {
        return ctx.throw403(Some("CSRF validation failed"));
    }
    
    // Token is accessible in templates
    ctx.view("form", json!({
        "user_data": load_user().await?
    })).await
}
```

### Method Details

#### `ctx.generate_csrf(token_id: Option<&str>)`

Generates a new CSRF token with optional custom ID and stores it in the session:

```rust
async fn api_csrf_token(ctx: &mut Context) -> rustf::Result<()> {
    // Default token
    let token = ctx.generate_csrf(None)?;
    
    // Custom token for specific form
    let upload_token = ctx.generate_csrf(Some("upload_csrf"))?;
    
    ctx.json(json!({
        "csrf_token": token,
        "upload_token": upload_token,
        "expires_in": "1 hour",
        "usage": "one-time"
    }))
}
```

#### `ctx.verify_csrf(token_id: Option<&str>)`

Manually verify and consume CSRF token (rarely needed due to middleware):

```rust
async fn manual_verification(ctx: &mut Context) -> rustf::Result<()> {
    // Verify default token (consumed after successful verification)
    match ctx.verify_csrf(None)? {
        true => {
            // Token was valid and is now consumed
            ctx.json(json!({"message": "Token valid and consumed"}))
        }
        false => {
            // Token is invalid, expired, or missing
            ctx.throw403(Some("CSRF token validation failed"))
        }
    }
}

// Verify custom token
async fn verify_upload(ctx: &mut Context) -> rustf::Result<()> {
    if ctx.verify_csrf(Some("upload_csrf"))? {
        // Upload token valid and consumed
        process_upload(&ctx).await
    } else {
        ctx.throw403(Some("Invalid upload token"))
    }
}
```

#### CSRF Token in Templates

CSRF tokens support multiple forms with different token IDs:

```rust
async fn show_forms(ctx: &mut Context) -> rustf::Result<()> {
    // Generate multiple tokens for different forms
    ctx.generate_csrf(None)?;  // Default token
    ctx.generate_csrf(Some("upload_csrf"))?;  // Upload form
    ctx.generate_csrf(Some("api_csrf"))?;  // API calls
    
    // Tokens accessible in templates:
    // - @{csrf} - hidden input with default token
    // - @{csrf("upload_csrf")} - hidden input with custom token
    // - @{csrf_token} - default token value
    // - @{csrf_token.upload_csrf} - custom token value
    
    ctx.view("forms/multi", json!({
        "user": load_user_data().await?
    })).await
}
```

**Template Usage:**
```html
<!-- Default token -->
<form method="POST">
    @{csrf}  <!-- <input type="hidden" name="_csrf_token" value="..."> -->
</form>

<!-- Custom token -->
<form method="POST" action="/upload">
    @{csrf("upload_csrf")}  <!-- <input type="hidden" name="upload_csrf" value="..."> -->
</form>

<!-- Token values for JavaScript -->
<script>
    const defaultToken = "@{csrf_token}";
    const uploadToken = "@{csrf_token.upload_csrf}";
    const apiToken = "@{csrf_token.api_csrf}";
</script>
```

## Error Handling

### Automatic Error Responses

CSRF failures are handled intelligently based on the request type:

#### API Requests (JSON Response)

For requests expecting JSON (detected by headers):

```json
{
    "error": "csrf_token_invalid",
    "message": "CSRF token validation failed. Please try again."
}
```

HTTP Status: `403 Forbidden`

#### Web Requests (HTML Response)

For regular web requests:
- Returns HTTP `403 Forbidden` with error page
- OR redirects to configured URL with flash message

### Custom Error Handling

Configure custom error responses:

```rust
let csrf_config = CsrfConfig::new()
    // Custom error message
    .error_message("Security token expired. Please refresh the page.")
    
    // Redirect instead of error page
    .redirect_on_failure("/login");

let csrf_config = csrf_config; // built above

RustF::new()
    .controllers(auto_controllers!())
    .middleware_from(move |registry| {
        registry.register_inbound("csrf", CsrfMiddleware::with_config(csrf_config.clone()));
    });

// In your login template (Total.js), show the flash message:
// @{if flash.error}
//     <div class="alert alert-danger">@{flash.error}</div>
// @{fi}
```

### Error Detection

The middleware detects request type using:

1. `Accept` header containing `application/json`
2. `Content-Type` header containing `application/json`
3. Request path starting with `/api/` (if not exempted)

## AJAX Integration

### Frontend CSRF Token Fetching

Create an endpoint to provide CSRF tokens for AJAX requests:

```rust
// CSRF token endpoint
async fn csrf_token_api(ctx: &mut Context) -> rustf::Result<()> {
    let token = ctx.generate_csrf(None)?;
    
    ctx.json(json!({
        "csrf_token": token,
        "field_name": "_csrf_token",
        "header_name": "X-CSRF-Token"
    }))
}

// Register the endpoint in a controller's install() (GET bypasses CSRF):
// routes![ GET "/api/csrf-token" => csrf_token_api ]
```

### JavaScript Integration

```javascript
// Fetch CSRF token
async function getCsrfToken() {
    const response = await fetch('/api/csrf-token');
    if (!response.ok) throw new Error('Failed to fetch CSRF token');
    const data = await response.json();
    return data.csrf_token;
}

// Method 1: HTTP Header (Recommended)
const csrfToken = await getCsrfToken();

const response = await fetch('/api/users', {
    method: 'POST',
    headers: {
        'Content-Type': 'application/json',
        'X-CSRF-Token': csrfToken  // Header method
    },
    body: JSON.stringify({
        name: 'John Doe',
        email: 'john@example.com'
    })
});

// Method 2: Form Data
const formData = new FormData();
formData.append('name', 'John Doe');
formData.append('email', 'john@example.com');
formData.append('_csrf_token', csrfToken);  // Form field method

await fetch('/api/users', {
    method: 'POST',
    body: formData
});
```

### Global AJAX Setup

Set up CSRF token globally for all AJAX requests:

```javascript
class CsrfManager {
    constructor() {
        this.token = null;
        this.tokenPromise = null;
    }
    
    async getToken() {
        if (!this.tokenPromise) {
            this.tokenPromise = this.fetchToken();
        }
        return this.tokenPromise;
    }
    
    async fetchToken() {
        const response = await fetch('/api/csrf-token');
        const data = await response.json();
        this.token = data.csrf_token;
        return this.token;
    }
    
    async apiCall(url, options = {}) {
        const token = await this.getToken();
        
        const headers = {
            ...options.headers,
            'X-CSRF-Token': token
        };
        
        return fetch(url, { ...options, headers });
    }
}

// Global instance
const csrf = new CsrfManager();

// Usage
await csrf.apiCall('/api/users', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(userData)
});
```

## Form Integration

### HTML Forms

Include CSRF tokens in HTML forms using the context helper:

```rust
async fn show_user_form(ctx: &mut Context) -> rustf::Result<()> {
    // Ensure CSRF token exists
    ctx.generate_csrf(None)?;
    
    ctx.view("users/form", json!({
        "user": load_user_data().await?,
        "action": "/users/create"
    })).await
}
```

Template (Total.js syntax):
```html
<!-- Default token -->
<form action="@{action}" method="POST">
    @{csrf}  <!-- Auto-generated hidden input -->
    
    <div class="form-group">
        <label>Name:</label>
        <input type="text" name="name" value="@{user.name}" required>
    </div>
    
    <div class="form-group">
        <label>Email:</label>
        <input type="email" name="email" value="@{user.email}" required>
    </div>
    
    <button type="submit">Save User</button>
</form>

<!-- Multiple forms with different tokens -->
<form id="userForm" method="POST" action="/users">
    @{csrf}  <!-- Default token -->
    <!-- form fields -->
</form>

<form id="uploadForm" method="POST" action="/upload">
    @{csrf("upload_csrf")}  <!-- Custom upload token -->
    <input type="file" name="file">
</form>
```

### Manual Token Inclusion

If you need to include tokens manually in data:

```rust
async fn manual_form(ctx: &mut Context) -> rustf::Result<()> {
    let csrf_token = ctx.generate_csrf(None)?;
    
    ctx.view("forms/manual", json!({
        "csrf_token": csrf_token,
        "form_data": load_form_data().await?
    })).await
}
```

Template:
```html
<!-- Default token manual inclusion -->
<form action="/users" method="POST">
    <input type="hidden" name="_csrf_token" value="@{csrf_token}">
    <!-- form fields... -->
</form>

<!-- Custom token manual inclusion -->
<form action="/api/action" method="POST">
    <input type="hidden" name="api_csrf" value="@{csrf_token.api_csrf}">
    <!-- form fields... -->
</form>

<!-- Mixed approach -->
<div>
    <form method="POST" action="/form1">
        @{csrf}  <!-- Auto-generated -->
    </form>
    
    <form method="POST" action="/form2">
        <input type="hidden" name="form2_csrf" value="@{csrf_token.form2_csrf}">
    </form>
</div>
```

### Form Processing

Form processing requires no changes - CSRF is handled by middleware:

```rust
async fn process_user_form(ctx: &mut Context) -> rustf::Result<()> {
    // CSRF already validated by middleware
    let form_data = ctx.body_form()?;
    
    // Validate form data
    let user_data = validate_user_form(&form_data)?;
    
    // Save user
    let user = create_user(&user_data).await?;
    
    // Success response
    ctx.flash_success("User created successfully!")?;
    ctx.redirect("/users")
}
```

## Advanced Usage

### Conditional CSRF Protection

Apply CSRF protection conditionally:

```rust
use rustf::security::CsrfConfig;

// Environment-based configuration
let csrf_config = match env::var("ENVIRONMENT").as_deref() {
    Ok("development") => {
        // Relaxed CSRF in development
        CsrfConfig::new()
            .exempt("/debug/*")
            .exempt("/test/*")
    }
    Ok("production") => {
        // Strict CSRF in production  
        CsrfConfig::new()
            .error_message("Security validation failed")
            .redirect_on_failure("/login")
    }
    _ => CsrfConfig::new()  // Default
};
```

### Custom HTTP Methods

Protect custom HTTP methods:

```rust
let csrf_config = CsrfConfig::new()
    .protect_method("CUSTOM")       // Add protection
    .protect_method("MERGE")        // Custom REST method
    .exempt_method("DELETE");       // Remove DELETE protection

RustF::new()
    .controllers(auto_controllers!())
    .middleware_from(move |registry| {
        registry.register_inbound("csrf", CsrfMiddleware::with_config(csrf_config.clone()));
    });
```

### Differentiating CSRF Across Routes

RustF registers a single CSRF middleware globally; there is no `RouteGroup` type
or per-group middleware. To apply different behavior to different paths, configure
exemptions (and protected methods) on one `CsrfConfig` and let path patterns
decide. Routes themselves are declared in controllers via `install()`:

```rust
// One CSRF config: public API paths are exempt, everything else is protected.
let csrf_config = CsrfConfig::new()
    .exempt("/api/public/*")              // public endpoints bypass CSRF
    .error_message("CSRF validation failed");

RustF::new()
    .controllers(auto_controllers!())
    .middleware_from(move |registry| {
        registry.register_inbound("csrf", CsrfMiddleware::with_config(csrf_config.clone()));
    });
```

```rust
// src/controllers/api.rs
pub fn install() -> Vec<Route> {
    routes![
        // Exempt by the /api/public/* pattern above:
        POST "/api/public/contact"    => contact_handler,
        POST "/api/public/newsletter" => newsletter_handler,
        // Protected (not exempt):
        POST "/api/private/users"     => create_user,
        PUT  "/api/private/users/{id}" => update_user,
    ]
}
```

### Exempting Webhook Routes

Webhooks usually come from external systems that cannot send a CSRF token, so
exempt their paths in the config:

```rust
let csrf_config = CsrfConfig::new()
    .exempt("/webhook/*")          // all webhook routes
    .exempt("/integration/*")      // all integration callbacks
    .exempt("/external/callback"); // a single exact route

RustF::new()
    .controllers(auto_controllers!())
    .middleware_from(move |registry| {
        registry.register_inbound("csrf", CsrfMiddleware::with_config(csrf_config.clone()));
    });
```

## Troubleshooting

### Common Issues

#### 1. 403 CSRF Token Errors

**Problem:** Getting 403 errors on valid requests

**Solutions:**
```rust
// Check if route is properly exempted
let csrf_config = CsrfConfig::new()
    .exempt("/your/route/*");  // Add exemption

// Verify token submission method
// For default token:
// - Header: X-CSRF-Token
// - Form field: _csrf_token or _token

// For custom token (e.g., "upload_csrf"):
// - Header: X-CSRF-Token
// - Form field: upload_csrf

// Check token expiration (tokens expire after 1 hour)
// Generate a fresh token if needed

// Remember: Tokens are one-time use
// After successful verification, generate a new token for the next request
```

#### 2. AJAX Requests Failing

**Problem:** AJAX requests returning 403 CSRF errors

**Solutions:**
```javascript
// Method 1: Include CSRF header
fetch('/api/endpoint', {
    method: 'POST',
    headers: {
        'X-CSRF-Token': await getCsrfToken()  // Most reliable
    },
    body: formData
});

// Method 2: Check API route exemptions
// Add explicit exemptions only for routes that truly cannot carry CSRF tokens
```

#### 3. Webhooks Failing

**Problem:** External webhooks returning 403 errors

**Solutions:**
```rust
// Add webhook exemptions
let csrf_config = CsrfConfig::new()
    .exempt("/webhook/*")           // All webhook routes
    .exempt("/integration/*")       // Integration callbacks
    .exempt("/callback/*");         // External callbacks
```

#### 4. Token Generation Issues

**Problem:** CSRF tokens not being generated or stored

**Solutions:**
```rust
// Check session configuration
// CSRF relies on sessions - ensure sessions are enabled

// Manual token generation
let token = ctx.generate_csrf(None)?;  // Default token
let custom = ctx.generate_csrf(Some("custom_id"))?;  // Custom token

// Verify session storage
// Check that session storage backend is working
// Tokens are stored as: {"token": "...", "valid_to": timestamp}
```

#### 5. Token Reuse Issues

**Problem:** Token validation fails on second attempt

**Solution:** Tokens are **one-time use** - they are consumed after successful verification

```rust
// Generate new token for each form submission
async fn show_form_after_submit(ctx: &mut Context) -> rustf::Result<()> {
    // Generate fresh token for the next submission
    ctx.generate_csrf(None)?;
    ctx.view("form", data).await
}

// For AJAX: Fetch new token after each successful request
```

### Debugging

Enable debug logging to troubleshoot CSRF issues:

```rust
// Add logging middleware to see request flow
RustF::new()
    .middleware_from(|registry| {
        registry.register_dual("logging", LoggingMiddleware::new());
    });

// Check CSRF middleware execution
// Look for log entries showing CSRF validation
```

Check CSRF token presence and expiration:

```rust
async fn debug_csrf(ctx: &mut Context) -> rustf::Result<()> {
    // Check default token (session() returns Option<&Session>)
    let token_data: Option<serde_json::Value> =
        ctx.session().and_then(|s| s.get("_csrf_token"));
    let token_info = if let Some(data) = token_data {
        json!({
            "token": data.get("token"),
            "expires_at": data.get("valid_to"),
            "expired": {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH).unwrap()
                    .as_secs();
                data.get("valid_to")
                    .and_then(|v| v.as_u64())
                    .map(|exp| now > exp)
                    .unwrap_or(true)
            }
        })
    } else {
        json!(null)
    };
    
    ctx.json(json!({
        "token_info": token_info,
        "headers": ctx.req.headers.clone(),
        "form_data": ctx.body_form().ok()
    }))
}
```

## Best Practices

### 1. Default Configuration

Start with default CSRF configuration and customize as needed:

```rust
// Good: Start simple
RustF::new()
    .middleware_from(|registry| {
        registry.register_inbound("csrf", CsrfMiddleware::new());
    });

// Then customize for specific needs
let csrf_config = CsrfConfig::new()
    .exempt("/specific/webhook")
    .error_message("Custom error message");
```

### 2. Route Organization

Organize routes to minimize exemptions:

```rust
// Good: Group exempt routes under common prefixes (in a controller's install()).
// These become exempt only if you configure the matching prefix:
routes![
    POST "/api/webhook/stripe"   => stripe_handler,
    POST "/api/webhook/github"   => github_handler,
    POST "/api/integration/slack" => slack_handler,
]

// Better than scattered exemptions in the config:
let csrf_config = CsrfConfig::new()
    .exempt("/webhook/stripe")      // Scattered
    .exempt("/integration/github")  // Scattered  
    .exempt("/callback/slack");     // Scattered
```

### 3. Token Management

Use context methods for token management:

```rust
// Good: Generate tokens before rendering
async fn show_form(ctx: &mut Context) -> rustf::Result<()> {
    ctx.generate_csrf(None)?;  // Default token
    ctx.generate_csrf(Some("api_csrf"))?;  // API token
    ctx.view("form", data).await  // Tokens accessible in templates
}

// Good: Regenerate after consumption
async fn handle_form(ctx: &mut Context) -> rustf::Result<()> {
    if !ctx.verify_csrf(None)? {
        return ctx.throw403(Some("Invalid token"));
    }
    // Token consumed, generate new one for next request
    ctx.generate_csrf(None)?;
    ctx.redirect("/form")
}

// Avoid: Reusing tokens
// Tokens are one-time use - always generate fresh tokens
```

### 4. Error Handling

Provide user-friendly error messages:

```rust
let csrf_config = CsrfConfig::new()
    .error_message("Your session has expired. Please refresh the page and try again.")
    .redirect_on_failure("/login");  // Better UX than error page
```

### 5. Testing

Test CSRF protection in your application. RustF does not ship a built-in HTTP
test client, so the snippet below is **illustrative pseudo-code** — adapt it to
whatever HTTP client/test harness you use to exercise your running app:

```rust,ignore
// Illustrative only: `app.post(...).send()` and `test_app_with_csrf()` are not
// RustF APIs. Drive your app through a real HTTP client (e.g. reqwest) instead.
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_csrf_protection() {
        let app = test_app_with_csrf();
        
        // Test protected route without token (should fail)
        let response = app.post("/users").send().await;
        assert_eq!(response.status(), 403);
        
        // Test with valid token (should succeed)
        let csrf_token = get_csrf_token(&app).await;
        let response = app.post("/users")
            .header("X-CSRF-Token", csrf_token)
            .json(json!({"name": "Test User"}))
            .send().await;
        assert_eq!(response.status(), 200);
    }
    
    #[tokio::test]
    async fn test_exempt_routes() {
        let app = test_app_with_csrf();
        
        // Test exempt route (should succeed without token)
        let response = app.post("/api/webhook/test").send().await;
        assert_eq!(response.status(), 200);
    }
}
```

### 6. Performance Considerations

- CSRF middleware has minimal performance impact
- Session reads/writes only occur on protected requests
- Pattern matching is optimized for common cases
- Consider using `should_run()` for conditional execution

### 7. Security Considerations

- Always use HTTPS in production to protect tokens in transit
- Tokens are automatically one-time use (prevents replay attacks)
- Tokens expire after 1 hour (limits attack window)
- Support for multiple concurrent tokens (different security contexts)
- Monitor for unusual CSRF failure patterns (potential attacks)
- Regularly review and update route exemptions

---

## Summary

RustF's CSRF protection provides:

✅ **One-Time Use Tokens** - Consumed after verification (prevents replay attacks)  
✅ **Token Expiration** - Automatic 1-hour expiration  
✅ **Multiple Concurrent Tokens** - Different tokens for different forms  
✅ **Zero Configuration** - Works out-of-the-box with sensible defaults  
✅ **Automatic Protection** - HTTP method-based validation  
✅ **Flexible Configuration** - Extensive customization options  
✅ **Session Integration** - Seamless token storage and validation  
✅ **Multiple Token Sources** - Headers and forms  
✅ **Smart Error Handling** - Context-aware error responses  
✅ **Developer Friendly** - Convenient context methods and helpers  

## Related Topics

- [Middleware](middleware.md) - Request/response processing and security middleware
- [Configuration](configuration.md) - Security configuration options
- [Sessions](sessions.md) - Secure session management
- [Error Handling](error-handling.md) - Secure error responses
---

## Additional Security Features

See the [Security Configuration](../guides/configuration.md#security) section for more security options.
