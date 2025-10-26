# RustF Module System User Guide

**Complete documentation based on actual framework implementation**

## Overview

RustF provides a comprehensive module system for organizing business logic, services, and reusable components. The system combines standard Rust module patterns with an optional `SharedModule` trait and a Total.js-style global MODULE accessor for singleton services. Modules follow clean architecture principles, separating business logic from HTTP request handling.

### Key Concepts
- **Standard Rust Modules** - Uses regular Rust module system with `use` statements for direct instantiation
- **Global MODULE System** - Total.js-style singleton access via `MODULE::get_typed::<T>()` for shared services
- **Auto-Discovery** - Automatic module scanning with `auto_modules!()` macro at compile time
- **SharedModule Trait** - Optional trait for lifecycle management and type-safe registration
- **Business Logic Separation** - Controllers handle HTTP, modules handle business logic
- **Flexible Access Patterns** - Choose between direct instantiation or singleton access based on needs

## Module Organization

### Directory Structure

```
src/
├── modules/
│   ├── user_service.rs      # User business logic
│   ├── email_service.rs     # Email operations
│   ├── payment_service.rs   # Payment processing
│   └── validation_utils.rs  # Utility functions
├── controllers/             # HTTP handlers
├── models/                  # Data models
└── _modules.rs             # Auto-generated (DO NOT EDIT)
```

### Module Types

The framework recognizes four types of modules:

1. **Services** - Stateful business logic with side effects (database operations, external API calls)
2. **Utilities** - Stateless pure functions for data transformation and validation
3. **Helpers** - Template and view helper functions
4. **Traits** - Custom trait definitions and interfaces

## Global MODULE System (New)

RustF provides a Total.js-style global MODULE system for singleton access to shared services:

### Overview

The MODULE system enables global access to registered modules without passing them through Context or using dependency injection. This is ideal for stateful services that should have a single instance across the entire application.

### Basic Usage

```rust
// Access modules globally anywhere in your application
let email = MODULE::get_typed::<EmailService>()?;
let cache = MODULE::get_typed::<CacheService>()?;

// Check if a module is registered
if MODULE::exists::<EmailService>() {
    let email = MODULE::get_typed::<EmailService>()?;
    email.send_notification("user@example.com").await?;
}

// Name-based access for dynamic scenarios
if let Some(service) = MODULE::get("EmailService") {
    // Use the service dynamically
}

// List all registered modules (useful for debugging)
let modules = MODULE::list();
for (name, module_type) in modules {
    println!("Module: {} ({})", name, module_type);
}
```

### When to Use MODULE Registration

**Good candidates for MODULE (singleton pattern):**
- **Database Connection Pools** - Expensive to create, should be shared
- **Cache Services** - In-memory caches, Redis connections
- **Email/SMS Services** - Configured once, used everywhere
- **WebSocket Managers** - Track all active connections
- **Rate Limiters** - Shared request counters
- **Background Job Queues** - Single task scheduler
- **Metrics Collectors** - Application-wide statistics

**Use direct instantiation for:**
- **Stateless Utilities** - Pure functions, no shared state
- **Request-Specific Services** - Need new instance per request
- **Simple Validators** - Lightweight, no configuration
- **View Helpers** - Template formatting functions

### Registering Modules for Global Access

To enable MODULE access, register your modules during app initialization:

```rust
// src/main.rs
use rustf::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let app = RustF::new()
        .controllers(auto_controllers!())
        
        // Option 1: Auto-discovery (requires install() function in each module)
        .modules_from(auto_modules!())
        
        // Option 2: Manual registration for specific modules
        .modules_from(|registry| {
            use crate::modules::{
                email_service::EmailService,
                cache_service::CacheService,
                database_pool::DatabasePool,
            };
            
            // These become singletons accessible via MODULE
            registry.register(EmailService::new());
            registry.register(CacheService::new());
            registry.register(DatabasePool::new());
        })
        
        .start().await
}
```

### Usage Patterns Comparison

#### Pattern 1: Direct Instantiation (Simple, Stateless)

```rust
// Best for stateless utilities and simple services
use crate::modules::validation_utils::ValidationUtils;

async fn handler(ctx: &mut Context) -> Result<()> {
    // Create new instance when needed
    let validator = ValidationUtils::new();
    
    if !validator.is_valid_email("user@example.com") {
        return ctx.throw400(Some("Invalid email"));
    }
    
    ctx.json(json!({"status": "valid"}))
}
```

#### Pattern 2: Global MODULE Access (Singleton, Stateful)

```rust
// Best for shared resources and configured services
async fn handler(ctx: &mut Context) -> Result<()> {
    // Access singleton instance
    let email_service = MODULE::get_typed::<EmailService>()?;
    let cache = MODULE::get_typed::<CacheService>()?;
    
    // Check cache first
    if let Some(cached) = cache.get("user:123").await {
        return ctx.json(cached);
    }
    
    // Send email using shared service
    email_service.send_welcome("user@example.com").await?;
    
    ctx.json(json!({"status": "sent"}))
}
```

#### Pattern 3: Hybrid Approach (Best of Both)

```rust
use crate::modules::{user_service, validation_utils};

async fn handler(ctx: &mut Context) -> Result<()> {
    // Stateless utility - direct instantiation
    let validator = validation_utils::ValidationUtils::new();
    
    // Stateful service - could use MODULE if registered
    let email = if MODULE::exists::<EmailService>() {
        // Use singleton if available
        MODULE::get_typed::<EmailService>()?
    } else {
        // Fall back to direct instantiation
        &email_service::EmailService::new()
    };
    
    // Business logic...
    email.send_notification("user@example.com").await?;
    
    ctx.json(json!({"status": "processed"}))
}
```

## Creating Modules

### Basic Module Structure

Every module is a standard Rust module with an optional `install()` function for auto-discovery:

```rust
// src/modules/user_service.rs
use rustf::prelude::*;

#[derive(Debug)]
pub struct UserService {
    name: String,
}

impl UserService {
    pub fn new() -> Self {
        Self {
            name: "UserService".to_string(),
        }
    }
    
    /// Register a new user with business logic validation
    pub async fn register_user(&self, email: &str, password: &str, profile: Value) -> Result<Value> {
        // Input validation
        if email.is_empty() || !email.contains('@') {
            return Err(Error::validation("Invalid email format"));
        }
        
        if password.len() < 8 {
            return Err(Error::validation("Password must be at least 8 characters"));
        }
        
        // Business logic implementation
        let user = json!({
            "user_id": 1,
            "email": email,
            "created_at": chrono::Utc::now(),
            "is_verified": false
        });
        
        Ok(user)
    }
}

// Required for auto-discovery
pub fn install() -> UserService {
    UserService::new()
}
```

### SharedModule Implementation (Optional)

For advanced lifecycle management, implement the `SharedModule` trait:

```rust
use rustf::prelude::*;
use async_trait::async_trait;

#[derive(Debug)]
pub struct EmailService {
    templates: HashMap<String, String>,
}

impl EmailService {
    pub fn new() -> Self {
        let mut templates = HashMap::new();
        templates.insert("welcome".to_string(), 
            "Welcome {{name}}! Please verify: {{link}}".to_string());
        
        Self { templates }
    }
    
    pub async fn send_verification_email(&self, email: &str, name: &str, token: &str) -> Result<Value> {
        let template = self.templates.get("welcome").unwrap();
        let rendered = template
            .replace("{{name}}", name)
            .replace("{{link}}", &format!("https://app.com/verify?token={}", token));
            
        // In production: integrate with email provider
        log::info!("Sending verification email to {}: {}", email, rendered);
        
        Ok(json!({
            "email": email,
            "status": "sent",
            "message_id": uuid::Uuid::new_v4()
        }))
    }
}

#[async_trait]
impl SharedModule for EmailService {
    fn name(&self) -> &'static str {
        "EmailService"
    }
    
    fn module_type(&self) -> SharedModuleType {
        SharedModuleType::Service
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    async fn initialize(&self) -> Result<()> {
        log::info!("EmailService initialized with {} templates", self.templates.len());
        Ok(())
    }
}

pub fn install() -> EmailService {
    EmailService::new()
}
```

### Utility Modules

For stateless utilities, use the convenience macro:

```rust
// src/modules/validation_utils.rs
use rustf::prelude::*;

pub struct ValidationUtils;

impl ValidationUtils {
    /// Validate email format
    pub fn is_valid_email(email: &str) -> bool {
        email.contains('@') && email.len() > 5
    }
    
    /// Validate password strength
    pub fn is_strong_password(password: &str) -> bool {
        password.len() >= 8 
            && password.chars().any(|c| c.is_uppercase())
            && password.chars().any(|c| c.is_lowercase())
            && password.chars().any(|c| c.is_numeric())
    }
    
    /// Sanitize user input
    pub fn sanitize_input(input: &str) -> String {
        input.trim()
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace("&", "&amp;")
    }
}

// Use convenience macro for simple implementation
impl_shared_util!(ValidationUtils);

pub fn install() -> ValidationUtils {
    ValidationUtils
}
```

## Module Discovery and Registration

### Auto-Discovery with `auto_modules!()`

The framework automatically discovers modules at compile time and optionally enables global MODULE access:

```rust
// src/main.rs
use rustf::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let app = RustF::new()
        .controllers(auto_controllers!())
        .models(auto_models!())
        .modules_from(auto_modules!())  // Enables global MODULE access
        .start().await
}
```

**What happens during registration:**
1. `auto_modules!()` scans `src/modules/` directory at compile time
2. Each module's `install()` function is called to create an instance
3. Modules implementing `SharedModule` are registered in SharedRegistry
4. `MODULE::init()` is called during app startup, enabling global access
5. Registered modules become singletons accessible via `MODULE::get_typed::<T>()`

### Manual Module Access

**Recommended Pattern**: Use standard Rust module system for accessing business logic:

```rust
// src/controllers/auth.rs
use rustf::prelude::*;
use crate::modules::{user_service, email_service};

pub fn install() -> Vec<Route> {
    routes![
        POST "/register" => register_user,
        POST "/login" => login_user,
    ]
}

async fn register_user(ctx: Context) -> Result<Response> {
    let form_data = ctx.body_form()?;
    let email = form_data.get("email").unwrap_or(&String::new());
    let password = form_data.get("password").unwrap_or(&String::new());
    
    // Access modules using standard Rust module system
    let user_svc = user_service::UserService::new();
    let email_svc = email_service::EmailService::new();
    
    // Business logic handled by modules
    match user_svc.register_user(email, password, json!({})).await {
        Ok(user) => {
            // Send verification email
            let _ = email_svc.send_verification_email(
                email, 
                "New User", 
                "verification_token_123"
            ).await;
            
            ctx.flash_success("Registration successful! Check your email.");
            ctx.redirect("/login")
        }
        Err(e) => {
            ctx.flash_error(&format!("Registration failed: {}", e));
            ctx.redirect("/register")
        }
    }
}
```

### SharedRegistry Pattern (Automatic)

The SharedRegistry is integrated with the RustF app and handles initialization automatically:

```rust
// src/main.rs
use rustf::prelude::*;

#[rustf::auto_discover]
#[tokio::main]
async fn main() -> Result<()> {
    let app = RustF::new()
        .controllers(auto_controllers!())
        .models(auto_models!())
        .modules_from(auto_modules!())  // Registers and initializes modules
        .start().await  // Calls initialize_all() automatically
}
```

**What happens automatically:**
1. `auto_modules!()` scans `src/modules/` and calls each module's `install()` function
2. Modules implementing `SharedModule` trait are registered with the SharedRegistry
3. `app.start()` calls `initialize_all()` on all registered modules
4. Modules are available throughout the application lifecycle

## Using Modules in Controllers

### Pattern 1: Direct Instantiation (Simple)

Controllers create module instances directly for clear, simple code:

```rust
// src/controllers/users.rs
use rustf::prelude::*;
use crate::modules::{user_service, validation_utils};

async fn create_user(ctx: Context) -> Result<Response> {
    let form_data = ctx.body_form()?;
    let email = form_data.get("email").unwrap_or(&String::new());
    let password = form_data.get("password").unwrap_or(&String::new());
    
    // Use validation utilities
    if !validation_utils::ValidationUtils::is_valid_email(email) {
        ctx.flash_error("Invalid email format");
        return ctx.redirect("/users/new");
    }
    
    if !validation_utils::ValidationUtils::is_strong_password(password) {
        ctx.flash_error("Password must be at least 8 characters with mixed case and numbers");
        return ctx.redirect("/users/new");
    }
    
    // Use business service
    let user_svc = user_service::UserService::new();
    match user_svc.register_user(email, password, json!({})).await {
        Ok(user) => {
            ctx.flash_success("User created successfully!");
            ctx.redirect("/users")
        }
        Err(e) => {
            ctx.flash_error(&format!("Failed to create user: {}", e));
            ctx.redirect("/users/new")
        }
    }
}
```

### Pattern 2: Global MODULE Access (Singleton)

Use MODULE for shared services that should have one instance:

```rust
// src/controllers/api.rs
use rustf::prelude::*;

async fn process_order(ctx: &mut Context) -> Result<()> {
    // Access singleton services via MODULE
    let cache = MODULE::get_typed::<CacheService>()?;
    let email = MODULE::get_typed::<EmailService>()?;
    let payment = MODULE::get_typed::<PaymentService>()?;
    
    let order_data: Value = ctx.body_json()?;
    let order_id = order_data["id"].as_str().unwrap_or("");
    
    // Check cache for duplicate order
    if cache.exists(&format!("order:{}", order_id)).await {
        return ctx.throw400(Some("Duplicate order"));
    }
    
    // Process payment using singleton service
    let payment_result = payment.process(
        order_data["amount"].as_u64().unwrap_or(0),
        order_data["token"].as_str().unwrap_or("")
    ).await?;
    
    // Send confirmation email
    email.send_order_confirmation(
        order_data["email"].as_str().unwrap_or(""),
        order_id
    ).await?;
    
    // Cache the order
    cache.set(&format!("order:{}", order_id), &order_data, 3600).await?;
    
    ctx.json(json!({
        "status": "processed",
        "payment_id": payment_result.id
    }))
}
```

### Pattern 3: Dependency Injection Pattern

For applications requiring shared state, pass dependencies explicitly:

```rust
// src/controllers/api.rs
use rustf::prelude::*;
use crate::modules::user_service::UserService;

// Service container approach
pub struct ApiServices {
    user_service: UserService,
    // other services...
}

impl ApiServices {
    pub fn new() -> Self {
        Self {
            user_service: UserService::new(),
        }
    }
}

async fn api_create_user(ctx: Context) -> Result<Response> {
    let services = ApiServices::new();
    let request_data: Value = ctx.body_json()?;
    
    let email = request_data["email"].as_str().unwrap_or("");
    let password = request_data["password"].as_str().unwrap_or("");
    
    match services.user_service.register_user(email, password, json!({})).await {
        Ok(user) => ctx.json(json!({
            "success": true,
            "user": user
        })),
        Err(e) => ctx.json(json!({
            "success": false,
            "error": e.to_string()
        }))
    }
}
```

## Module Patterns and Best Practices

### 1. Service Pattern (Stateful Business Logic)

```rust
// src/modules/order_service.rs
use rustf::prelude::*;

#[derive(Debug)]
pub struct OrderService {
    config: OrderConfig,
}

#[derive(Debug)]
pub struct OrderConfig {
    pub max_items: usize,
    pub tax_rate: f64,
}

impl OrderService {
    pub fn new() -> Self {
        Self {
            config: OrderConfig {
                max_items: 100,
                tax_rate: 0.08,
            },
        }
    }
    
    pub async fn create_order(&self, user_id: i64, items: Vec<OrderItem>) -> Result<Order> {
        // Business validation
        if items.len() > self.config.max_items {
            return Err(Error::validation("Too many items in order"));
        }
        
        // Calculate totals
        let subtotal: f64 = items.iter().map(|item| item.price * item.quantity as f64).sum();
        let tax = subtotal * self.config.tax_rate;
        let total = subtotal + tax;
        
        // Create order (in real app: save to database)
        let order = Order {
            id: 1,
            user_id,
            items,
            subtotal,
            tax,
            total,
            status: OrderStatus::Pending,
            created_at: chrono::Utc::now(),
        };
        
        log::info!("Created order {} for user {} (total: ${:.2})", order.id, user_id, total);
        Ok(order)
    }
    
    pub async fn calculate_shipping(&self, order: &Order, address: &Address) -> Result<f64> {
        // Shipping calculation logic
        let base_rate = 5.99;
        let weight_factor = order.items.len() as f64 * 0.5;
        Ok(base_rate + weight_factor)
    }
}

#[derive(Debug, Serialize)]
pub struct Order {
    pub id: i64,
    pub user_id: i64,
    pub items: Vec<OrderItem>,
    pub subtotal: f64,
    pub tax: f64,
    pub total: f64,
    pub status: OrderStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub fn install() -> OrderService {
    OrderService::new()
}
```

### 2. Utility Pattern (Stateless Functions)

```rust
// src/modules/text_utils.rs
use rustf::prelude::*;

pub struct TextUtils;

impl TextUtils {
    /// Convert text to URL-friendly slug
    pub fn slugify(text: &str) -> String {
        text.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>()
            .join("-")
    }
    
    /// Truncate text with ellipsis
    pub fn truncate(text: &str, max_length: usize) -> String {
        if text.len() <= max_length {
            text.to_string()
        } else {
            format!("{}...", &text[..max_length - 3])
        }
    }
    
    /// Extract mentions from text (@username)
    pub fn extract_mentions(text: &str) -> Vec<String> {
        let re = regex::Regex::new(r"@(\w+)").unwrap();
        re.captures_iter(text)
            .map(|cap| cap[1].to_string())
            .collect()
    }
    
    /// Generate random string
    pub fn random_string(length: usize) -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut rng = rand::thread_rng();
        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
}

impl_shared_util!(TextUtils);

pub fn install() -> TextUtils {
    TextUtils
}
```

### 3. Helper Pattern (View Helpers)

```rust
// src/modules/view_helpers.rs
use rustf::prelude::*;

pub struct ViewHelpers;

impl ViewHelpers {
    /// Format currency for display
    pub fn format_currency(amount: f64, currency: &str) -> String {
        match currency {
            "USD" => format!("${:.2}", amount),
            "EUR" => format!("€{:.2}", amount),
            "GBP" => format!("£{:.2}", amount),
            _ => format!("{:.2} {}", amount, currency),
        }
    }
    
    /// Format relative time (e.g., "2 hours ago")
    pub fn time_ago(datetime: &chrono::DateTime<chrono::Utc>) -> String {
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(*datetime);
        
        if duration.num_days() > 7 {
            datetime.format("%Y-%m-%d").to_string()
        } else if duration.num_days() > 0 {
            format!("{} days ago", duration.num_days())
        } else if duration.num_hours() > 0 {
            format!("{} hours ago", duration.num_hours())
        } else if duration.num_minutes() > 0 {
            format!("{} minutes ago", duration.num_minutes())
        } else {
            "Just now".to_string()
        }
    }
    
    /// Generate avatar URL from email
    pub fn gravatar_url(email: &str, size: u32) -> String {
        let hash = md5::compute(email.trim().to_lowercase().as_bytes());
        format!("https://www.gravatar.com/avatar/{:x}?s={}&d=identicon", hash, size)
    }
    
    /// Pluralize words based on count
    pub fn pluralize(word: &str, count: usize) -> String {
        if count == 1 {
            word.to_string()
        } else if word.ends_with('y') {
            format!("{}ies", &word[..word.len()-1])
        } else if word.ends_with(&['s', 'x', 'z']) || word.ends_with("ch") || word.ends_with("sh") {
            format!("{}es", word)
        } else {
            format!("{}s", word)
        }
    }
}

impl_shared_helper!(ViewHelpers);

pub fn install() -> ViewHelpers {
    ViewHelpers
}
```

## Advanced Features

### Module Lifecycle Management

For modules requiring initialization and cleanup:

```rust
// src/modules/cache_service.rs
use rustf::prelude::*;

#[derive(Debug)]
pub struct CacheService {
    redis_client: Option<redis::Client>,
    memory_cache: HashMap<String, CacheEntry>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    value: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

impl CacheService {
    pub fn new() -> Self {
        Self {
            redis_client: None,
            memory_cache: HashMap::new(),
        }
    }
    
    pub async fn get(&self, key: &str) -> Option<String> {
        // Try memory cache first
        if let Some(entry) = self.memory_cache.get(key) {
            if entry.expires_at > chrono::Utc::now() {
                return Some(entry.value.clone());
            }
        }
        
        // Fall back to Redis if available
        None // Simplified for example
    }
    
    pub async fn set(&mut self, key: &str, value: &str, ttl_seconds: u64) -> Result<()> {
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(ttl_seconds as i64);
        self.memory_cache.insert(key.to_string(), CacheEntry {
            value: value.to_string(),
            expires_at,
        });
        Ok(())
    }
}

#[async_trait]
impl SharedModule for CacheService {
    fn name(&self) -> &'static str {
        "CacheService"
    }
    
    fn module_type(&self) -> SharedModuleType {
        SharedModuleType::Service
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    async fn initialize(&self) -> Result<()> {
        log::info!("CacheService initializing...");
        
        // Initialize Redis connection
        if let Ok(redis_url) = std::env::var("REDIS_URL") {
            match redis::Client::open(redis_url) {
                Ok(_client) => log::info!("Redis cache client initialized"),
                Err(e) => log::warn!("Failed to initialize Redis: {}", e),
            }
        }
        
        log::info!("CacheService initialized successfully");
        Ok(())
    }
    
    async fn shutdown(&self) -> Result<()> {
        log::info!("CacheService shutting down...");
        // Cleanup operations would go here
        Ok(())
    }
}

pub fn install() -> CacheService {
    CacheService::new()
}
```

### Cross-Module Dependencies

Modules can depend on each other using standard Rust patterns:

```rust
// src/modules/notification_service.rs
use rustf::prelude::*;
use super::email_service::EmailService;

#[derive(Debug)]
pub struct NotificationService {
    email_service: EmailService,
}

impl NotificationService {
    pub fn new() -> Self {
        Self {
            email_service: EmailService::new(),
        }
    }
    
    pub async fn send_welcome_notification(&self, user_email: &str, user_name: &str) -> Result<()> {
        // Send email notification
        self.email_service.send_verification_email(user_email, user_name, "welcome_token").await?;
        
        // Could also send push notification, SMS, etc.
        log::info!("Welcome notification sent to {}", user_email);
        Ok(())
    }
    
    pub async fn send_order_confirmation(&self, user_email: &str, order_id: i64) -> Result<()> {
        // Complex notification logic using email service
        let subject = format!("Order #{} confirmed", order_id);
        // Implementation details...
        Ok(())
    }
}

pub fn install() -> NotificationService {
    NotificationService::new()
}
```

## Testing Modules

### Unit Testing Services

```rust
// src/modules/user_service.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_register_user_valid() {
        let service = UserService::new();
        let result = service.register_user(
            "test@example.com", 
            "password123", 
            json!({})
        ).await;
        
        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user["email"], "test@example.com");
        assert_eq!(user["is_verified"], false);
    }
    
    #[tokio::test]
    async fn test_register_user_invalid_email() {
        let service = UserService::new();
        let result = service.register_user(
            "invalid-email", 
            "password123", 
            json!({})
        ).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid email format"));
    }
    
    #[tokio::test]
    async fn test_register_user_weak_password() {
        let service = UserService::new();
        let result = service.register_user(
            "test@example.com", 
            "123", 
            json!({})
        ).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least 8 characters"));
    }
}
```

### Integration Testing

```rust
// tests/integration/module_tests.rs
use rustf::prelude::*;
use your_app::modules::{user_service, email_service};

#[tokio::test]
async fn test_user_registration_flow() {
    // Setup
    let user_svc = user_service::UserService::new();
    let email_svc = email_service::EmailService::new();
    
    // Test user registration
    let user_result = user_svc.register_user(
        "integration@test.com",
        "TestPassword123",
        json!({"first_name": "Test", "last_name": "User"})
    ).await;
    
    assert!(user_result.is_ok());
    let user = user_result.unwrap();
    
    // Test email notification
    let email_result = email_svc.send_verification_email(
        "integration@test.com",
        "Test",
        "test_token_123"
    ).await;
    
    assert!(email_result.is_ok());
    let email_response = email_result.unwrap();
    assert_eq!(email_response["email"]["status"], "sent");
}
```

## Configuration

### Environment-Based Module Configuration

```rust
// src/modules/payment_service.rs
use rustf::prelude::*;

#[derive(Debug)]
pub struct PaymentService {
    stripe_key: Option<String>,
    sandbox_mode: bool,
}

impl PaymentService {
    pub fn new() -> Self {
        let stripe_key = std::env::var("STRIPE_SECRET_KEY").ok();
        let sandbox_mode = std::env::var("PAYMENT_SANDBOX")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);
            
        Self {
            stripe_key,
            sandbox_mode,
        }
    }
    
    pub async fn process_payment(&self, amount: u64, token: &str) -> Result<PaymentResult> {
        if self.sandbox_mode {
            log::info!("SANDBOX: Processing payment of ${} with token {}", amount, token);
            return Ok(PaymentResult {
                id: "sandbox_payment_123".to_string(),
                status: PaymentStatus::Succeeded,
                amount,
            });
        }
        
        // Real payment processing logic
        match &self.stripe_key {
            Some(key) => {
                // Use Stripe API
                log::info!("Processing real payment with Stripe");
                // Implementation...
                Ok(PaymentResult {
                    id: "real_payment_456".to_string(),
                    status: PaymentStatus::Succeeded,
                    amount,
                })
            }
            None => {
                Err(Error::configuration("STRIPE_SECRET_KEY not configured"))
            }
        }
    }
}

pub fn install() -> PaymentService {
    PaymentService::new()
}
```

## Module Documentation Patterns

### Self-Documenting Modules

```rust
// src/modules/analytics_service.rs
//! Analytics Service Module
//! 
//! Provides comprehensive analytics tracking and reporting functionality.
//! 
//! ## Features
//! - Event tracking with custom properties
//! - User behavior analytics
//! - Performance metrics collection
//! - A/B testing support
//! 
//! ## Usage
//! ```rust
//! use crate::modules::analytics_service::AnalyticsService;
//! 
//! let analytics = AnalyticsService::new();
//! analytics.track_event("user_signup", json!({"source": "web"})).await?;
//! ```

use rustf::prelude::*;

/// Analytics service for tracking user events and system metrics
/// 
/// The AnalyticsService provides methods for tracking various types of events:
/// - User actions (clicks, page views, form submissions)
/// - System events (errors, performance metrics)  
/// - Custom business events (purchases, subscriptions)
#[derive(Debug)]
pub struct AnalyticsService {
    /// Whether analytics is enabled (can be disabled for testing)
    enabled: bool,
    /// Buffer for batching events before sending
    event_buffer: Vec<AnalyticsEvent>,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsEvent {
    pub event_name: String,
    pub properties: Value,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl AnalyticsService {
    /// Create new analytics service
    /// 
    /// # Example
    /// ```rust
    /// let analytics = AnalyticsService::new();
    /// ```
    pub fn new() -> Self {
        let enabled = std::env::var("ANALYTICS_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);
            
        Self {
            enabled,
            event_buffer: Vec::new(),
        }
    }
    
    /// Track a user event with optional properties
    /// 
    /// # Arguments
    /// * `event_name` - Name of the event (e.g., "page_view", "button_click")
    /// * `properties` - Additional event data as JSON
    /// 
    /// # Example
    /// ```rust
    /// analytics.track_event("user_signup", json!({
    ///     "source": "web",
    ///     "plan": "premium"
    /// })).await?;
    /// ```
    pub async fn track_event(&mut self, event_name: &str, properties: Value) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        
        let event = AnalyticsEvent {
            event_name: event_name.to_string(),
            properties,
            user_id: None, // Would be set from context in real implementation
            session_id: None,
            timestamp: chrono::Utc::now(),
        };
        
        self.event_buffer.push(event);
        
        // Flush buffer if it gets too large
        if self.event_buffer.len() >= 100 {
            self.flush_events().await?;
        }
        
        Ok(())
    }
    
    /// Flush buffered events to analytics provider
    async fn flush_events(&mut self) -> Result<()> {
        if self.event_buffer.is_empty() {
            return Ok(());
        }
        
        log::info!("Flushing {} analytics events", self.event_buffer.len());
        
        // In real implementation: send to analytics provider
        // Google Analytics, Mixpanel, Segment, etc.
        
        self.event_buffer.clear();
        Ok(())
    }
}

pub fn install() -> AnalyticsService {
    AnalyticsService::new()
}
```

## Error Handling in Modules

### Consistent Error Patterns

```rust
// src/modules/file_service.rs
use rustf::prelude::*;
use std::path::PathBuf;

#[derive(Debug)]
pub struct FileService {
    upload_path: PathBuf,
    max_file_size: usize,
}

impl FileService {
    pub fn new() -> Self {
        let upload_path = std::env::var("UPLOAD_PATH")
            .unwrap_or_else(|_| "uploads".to_string())
            .into();
            
        let max_file_size = std::env::var("MAX_FILE_SIZE")
            .unwrap_or_else(|_| "10485760".to_string()) // 10MB default
            .parse()
            .unwrap_or(10485760);
            
        Self {
            upload_path,
            max_file_size,
        }
    }
    
    pub async fn save_upload(&self, file_data: &[u8], filename: &str) -> Result<FileUploadResult> {
        // Validation
        if file_data.is_empty() {
            return Err(Error::validation("File cannot be empty"));
        }
        
        if file_data.len() > self.max_file_size {
            return Err(Error::validation(&format!(
                "File size {} exceeds maximum allowed size {}", 
                file_data.len(), 
                self.max_file_size
            )));
        }
        
        // Security: validate filename
        if filename.contains("..") || filename.contains("/") || filename.contains("\\") {
            return Err(Error::security("Invalid filename"));
        }
        
        // Generate unique filename
        let unique_filename = format!("{}_{}", 
            uuid::Uuid::new_v4(), 
            filename
        );
        
        let file_path = self.upload_path.join(&unique_filename);
        
        // Ensure upload directory exists
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::filesystem(&format!("Failed to create upload directory: {}", e)))?;
        }
        
        // Save file
        std::fs::write(&file_path, file_data)
            .map_err(|e| Error::filesystem(&format!("Failed to save file: {}", e)))?;
        
        log::info!("File saved: {} ({} bytes)", unique_filename, file_data.len());
        
        Ok(FileUploadResult {
            filename: unique_filename,
            path: file_path.to_string_lossy().to_string(),
            size: file_data.len(),
            mime_type: self.detect_mime_type(filename),
        })
    }
    
    fn detect_mime_type(&self, filename: &str) -> String {
        match filename.split('.').last() {
            Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),
            Some("png") => "image/png".to_string(),
            Some("gif") => "image/gif".to_string(),
            Some("pdf") => "application/pdf".to_string(),
            Some("txt") => "text/plain".to_string(),
            _ => "application/octet-stream".to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FileUploadResult {
    pub filename: String,
    pub path: String,
    pub size: usize,
    pub mime_type: String,
}

pub fn install() -> FileService {
    FileService::new()
}
```

## Best Practices Summary

### 1. Module Design Principles

- **Single Responsibility** - Each module handles one domain of business logic
- **Clear Interfaces** - Public methods are well-documented and consistent
- **Error Handling** - Use Result<T> for operations that can fail
- **Configuration** - Use environment variables for runtime configuration
- **Logging** - Include appropriate logging for debugging and monitoring

### 2. Code Organization

```rust
// ✅ Good: Clear module structure
// src/modules/
//   ├── user_service.rs        # User domain logic
//   ├── order_service.rs       # Order processing
//   ├── payment_service.rs     # Payment handling
//   ├── email_service.rs       # Email operations
//   └── validation_utils.rs    # Shared utilities

// ❌ Bad: Mixed concerns
// src/modules/
//   └── everything_service.rs  # Handles users, orders, payments, etc.
```

### 3. Dependencies and Coupling

```rust
// ✅ Good: Explicit dependencies
use crate::modules::email_service::EmailService;

pub struct UserService {
    email_service: EmailService,
}

// ❌ Bad: Hidden global state
pub struct UserService;

impl UserService {
    pub async fn register_user(&self, email: &str) -> Result<User> {
        // Hidden dependency on global email service
        GLOBAL_EMAIL_SERVICE.send_welcome(email).await?;
    }
}
```

### 4. Testing Strategy

```rust
// ✅ Good: Testable design
impl PaymentService {
    pub fn new_for_testing() -> Self {
        Self {
            stripe_key: None,
            sandbox_mode: true, // Always sandbox for tests
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_payment_processing() {
        let service = PaymentService::new_for_testing();
        // Test implementation
    }
}
```

## Migration and Compatibility

### From Older Module Systems

If migrating from other Rust web frameworks:

```rust
// Actix Web style -> RustF style
// Old:
// web::Data<EmailService>

// New:
use crate::modules::email_service::EmailService;
let email_service = EmailService::new();
```

### Framework Agnostic Modules

Keep business logic independent of RustF specifics:

```rust
// ✅ Good: Framework-independent core logic
impl UserService {
    pub async fn validate_and_create_user(&self, email: &str, password: &str) -> Result<User> {
        // Core business logic - no RustF dependencies
        // Can be used in CLI tools, tests, other frameworks
    }
}

// ❌ Bad: Tightly coupled to RustF
impl UserService {
    pub async fn create_user(&self, ctx: &Context) -> Result<Response> {
        // Mixing business logic with HTTP concerns
    }
}
```

## Framework Integration Status

Based on the current RustF implementation:

✅ **SharedModule Trait** - Complete implementation in `src/shared.rs`  
✅ **SharedRegistry** - Full registry with lifecycle management  
✅ **Module Types** - Service, Util, Helper, Trait categorization  
✅ **Auto-discovery macro** - `auto_modules!()` in rustf-macros  
✅ **Standard Rust modules** - Normal `use` statement access  
✅ **Lifecycle management** - Initialize/shutdown methods  
✅ **App Integration** - SharedRegistry integrated with RustF app builder pattern  
✅ **CLI Support** - `rustf-cli new` creates `src/modules/` directory with README  
✅ **Global MODULE System** - Total.js-style singleton access via `MODULE::get_typed::<T>()`  
✅ **Type-Safe Access** - Compile-time type checking for module access  
✅ **MODULE Initialization** - Automatic during app startup via `MODULE::init()`  
✅ **Dynamic Module Discovery** - `MODULE::list()` and `MODULE::exists()` methods  

## Usage Recommendations

**For most applications:**
1. Use `.modules_from(auto_modules!())` in your app setup to enable MODULE system
2. Create services as structs with `new()` constructors and `install()` functions  
3. For stateless utilities: Use direct instantiation with `use` statements
4. For stateful services: Register and access via `MODULE::get_typed::<T>()`

**Choosing between direct instantiation and MODULE:**
1. **Use MODULE for:**
   - Database connection pools
   - Cache services (Redis, in-memory)
   - Configured services (email, SMS, payment)
   - Services with expensive initialization
   - Resources that should be shared application-wide

2. **Use direct instantiation for:**
   - Stateless utility functions
   - Request-specific services
   - Simple validators and formatters
   - Services that don't need configuration

**For simple modules that don't need lifecycle management:**
1. Create basic structs with static methods
2. Use `impl_shared_util!()` macro for utilities
3. Access directly via standard Rust module imports

**For complex services requiring singleton behavior:**
1. Implement `SharedModule` trait with `initialize()` and `shutdown()` methods
2. Register with `.modules_from()` to enable MODULE access
3. Access via `MODULE::get_typed::<T>()` throughout the application
4. Services are automatically initialized during app startup

The module system now provides full integration with the framework while maintaining flexibility for both simple and complex use cases, with clean separation of concerns between HTTP handling (controllers) and business logic (modules).

## Summary

RustF's module system provides:

✅ **Clean Architecture** - Separation of business logic from HTTP concerns  
✅ **Standard Rust Patterns** - Uses familiar module system and traits  
✅ **Auto-Discovery** - Compile-time scanning of module directories  
✅ **Type Safety** - Compile-time validation of module interfaces  
✅ **Lifecycle Management** - Optional initialize/shutdown methods  
✅ **Global MODULE System** - Total.js-style singleton access for shared services  
✅ **Flexible Access Patterns** - Support for both direct instantiation and singleton access  
✅ **Testing Support** - Easy to unit test individual modules  
✅ **Resource Efficiency** - Singleton pattern for expensive resources  

The system strikes a balance between simplicity and power, offering multiple patterns:
- **Direct instantiation** for simple, stateless utilities
- **Global MODULE access** for shared, stateful services
- **Hybrid approaches** for maximum flexibility

This allows developers to organize business logic effectively while maintaining clean architecture principles that make applications maintainable, testable, and resource-efficient.