# RustF Database & Model Layer Documentation

## Overview

RustF provides a modern, ergonomic database layer with:
- **Transparent database access** - No need to pass database connections around
- **Multi-database support** - PostgreSQL, MySQL/MariaDB, SQLite
- **Type-safe query builders** - Compile-time query validation
- **Laravel-style model queries** - Chainable, intuitive API
- **Schema-driven development** - YAML schemas generate type-safe models
- **Smart change tracking** - Only update modified fields

## Quick Start

### Basic Usage

```rust
use rustf::prelude::*;

// Find a user by ID
let user = Users::find(123).await?;

// Count all active users
let active_count = Users::query()?
    .where_eq("is_active", true)
    .count()
    .await?;

// Get paginated results
let users = Users::paginate(1, 20).await?;

// Delete a record
if let Some(user) = Users::find(456).await? {
    user.delete().await?;
}
```

## Database Configuration

### Initial Setup

Configure your database in `config.toml`:

```toml
[database]
url = "postgresql://user:password@localhost/mydb"  # or mysql:// or sqlite://
max_connections = 10
timeout = 30
```

The database connection is automatically initialized when your app starts. No manual setup required!

### Global Database Access

RustF uses a global `DB` singleton that's initialized once at startup:

```rust
use rustf::database::DB;

// Check if database is initialized
if DB::is_initialized() {
    // Database is ready
}

// Get the backend type
let backend = DB::backend(); // Some(DatabaseBackend::Postgres)
```

## Model System

### Generated Models

Models are generated from YAML schemas using the RustF CLI. Each model consists of:
1. **Base model** (`base/*.inc.rs`) - Auto-generated, never edit
2. **Wrapper model** (`*.rs`) - Your business logic, safe to edit

### Creating a Schema

Create a YAML schema in `schemas/users.yaml`:

```yaml
table: users
description: User accounts for the application
fields:
  - name: id
    type: integer
    primary_key: true
    auto_increment: true
    
  - name: email
    type: string
    max_length: 255
    nullable: false
    unique: true
    description: User's email address
    
  - name: username
    type: string
    max_length: 100
    nullable: false
    
  - name: is_active
    type: boolean
    default: true
    
  - name: created_at
    type: datetime
    nullable: true
    
  - name: status
    type: enum
    values: ["active", "inactive", "pending"]
    default: "pending"
    description: Account status
    
  - name: role
    type: enum  
    values: ["admin", "user", "moderator"]
    nullable: false
    default: "user"
    description: User role in the system
    
  - name: updated_at
    type: datetime
    nullable: true

indexes:
  - columns: [email]
    unique: true
  - columns: [username]
    unique: true
```

### Generating Models

```bash
# Generate models from schemas
rustf-cli schema generate models

# Force regeneration (backs up existing files)
rustf-cli schema generate models --force
```

This creates:
- `src/models/base/users.inc.rs` - Generated base model
- `src/models/users.rs` - Wrapper for your business logic (if doesn't exist)

## CRUD Operations

### Finding Records

```rust
// Find by ID - returns Option<Model>
let user = Users::find(123).await?;

// Find first record
let first_user = Users::first().await?;

// Find all records (use with caution!)
let all_users = Users::all().await?;

// Find with conditions
let active_users = Users::query()?
    .where_eq("is_active", true)
    .get()
    .await?;
```

### Creating Records

```rust
// Using the builder pattern (recommended)
let new_user = Users::builder()
    .email("user@example.com")
    .username("johndoe")
    .is_active(true)
    .save()  // Saves directly to database
    .await?;

// Manual creation (for complex cases)
let user = Users {
    id: 0,  // Will be set by database
    email: "user@example.com".to_string(),
    username: "johndoe".to_string(),
    is_active: true,
    created_at: Some(Utc::now()),
    updated_at: None,
    // ... other fields
};
// Then save using your custom method
```

### Updating Records

Models track changes automatically - only modified fields are updated:

```rust
// Find and update
let mut user = Users::find(123).await?.unwrap();

// Setters automatically track changes
user.set_email("newemail@example.com");
user.set_is_active(false);

// Only updates email and is_active fields
user.update().await?;

// Check if there are changes
if user.has_changes() {
    println!("User has unsaved changes");
}
```

### Deleting Records

```rust
// Delete by finding first
if let Some(user) = Users::find(123).await? {
    user.delete().await?;
}

// Delete with query
let deleted_count = Users::query()?
    .where_eq("is_active", false)
    .where_lt("last_login", "2024-01-01")
    .delete()
    .await?;
```

## Query Builder

### Basic Queries

```rust
// Simple equality check
let users = Users::query()?
    .where_eq("role", "admin")
    .get()
    .await?;

// Multiple conditions (AND)
let active_admins = Users::query()?
    .where_eq("role", "admin")
    .where_eq("is_active", true)
    .where_not_null("email_verified")
    .get()
    .await?;

// OR conditions
let users = Users::query()?
    .where_eq("role", "admin")
    .or_where_eq("role", "moderator")
    .get()
    .await?;
```

### Available Query Methods

#### WHERE Conditions
- `where_eq(column, value)` - Equal to
- `where_ne(column, value)` - Not equal to
- `where_gt(column, value)` - Greater than
- `where_lt(column, value)` - Less than
- `where_gte(column, value)` - Greater than or equal
- `where_lte(column, value)` - Less than or equal
- `where_like(column, pattern)` - LIKE pattern matching
- `where_not_like(column, pattern)` - NOT LIKE
- `where_in(column, vec![values])` - IN list
- `where_not_in(column, vec![values])` - NOT IN
- `where_between(column, start, end)` - BETWEEN range
- `where_null(column)` - IS NULL
- `where_not_null(column)` - IS NOT NULL

#### OR Conditions
All WHERE methods have `or_where_*` variants:
- `or_where_eq(column, value)`
- `or_where_gt(column, value)`
- `or_where_null(column)`
- etc.

#### Modifiers
- `order_by(column, OrderDirection::Asc/Desc)`
- `limit(n)`
- `offset(n)`
- `paginate(page, per_page)`

#### Field Selection
- `select(&[columns])` - Select specific fields instead of SELECT *
- `select_raw(&[expressions])` - Select with SQL expressions and aggregations
- `alias("name")` - Set a table alias for complex queries

#### Grouping
- `group_by(&[columns])` - Group results by specified columns

#### Execution Methods
- `get()` - Get all matching records as model instances
- `get_raw()` - Get results as JSON (useful for aggregations and JOINs)
- `first()` - Get first record
- `count()` - Count matching records
- `exists()` - Check if any records exist
- `delete()` - Delete matching records

### Complex Queries

```rust
// Pagination with conditions
let users = Users::query()?
    .where_eq("is_active", true)
    .where_like("email", "%@company.com")
    .order_by("created_at", OrderDirection::Desc)
    .paginate(2, 20)  // Page 2, 20 per page
    .get()
    .await?;

// Count with conditions
let admin_count = Users::query()?
    .where_eq("role", "admin")
    .where_not_null("verified_at")
    .count()
    .await?;

// Check existence
let has_admins = Users::query()?
    .where_eq("role", "admin")
    .exists()
    .await?;

// Complex date queries
let recent_users = Users::query()?
    .where_gt("created_at", "2024-01-01")
    .where_between("age", 18, 65)
    .where_in("status", vec!["active", "pending"])
    .order_by("created_at", OrderDirection::Desc)
    .limit(100)
    .get()
    .await?;
```

## Reusable Query Filters (ModelFilter)

ModelFilter allows you to build reusable query conditions that can be applied to multiple queries. This is especially useful for common filtering patterns across your application.

### Basic Usage

```rust
use rustf::models::ModelFilter;

// Create a reusable filter
let active_users = ModelFilter::new()
    .where_eq("is_active", true)
    .where_not_null("verified_at");

// Apply the same filter to different queries
let count = Users::query()?
    .apply_filter(&active_users)
    .count()
    .await?;

let users = Users::query()?
    .apply_filter(&active_users)
    .order_by("created_at", OrderDirection::Desc)
    .limit(10)
    .get()
    .await?;
```

### Conditional Filter Building

ModelFilter supports conditional building by reassigning the filter variable:

```rust
// Start with an empty filter
let mut filter = ModelFilter::new();

// Always apply base conditions
filter = filter.where_eq("is_active", true);

// Conditionally add filters based on user input
if let Some(search) = ctx.query("search") {
    filter = filter.where_like("name", &format!("%{}%", search));
}

if let Some(role) = ctx.query("role") {
    filter = filter.where_eq("role", role);
}

if let Some(min_age) = ctx.query("min_age") {
    filter = filter.where_gte("age", min_age.parse::<i32>()?);
}

// Use the conditionally built filter
let results = Users::query()?
    .apply_filter(&filter)
    .get()
    .await?;
```

### Combining Filters

You can combine multiple filters using the `and()` method:

```rust
// Create base filters
let active_filter = ModelFilter::new()
    .where_eq("is_active", true);

let verified_filter = ModelFilter::new()
    .where_not_null("email_verified_at")
    .where_not_null("phone_verified_at");

// Combine filters
let combined = active_filter.and(verified_filter);

// Use combined filter
let users = Users::query()?
    .apply_filter(&combined)
    .get()
    .await?;
```

### Common Filter Patterns

```rust
// Pagination filter with search
let mut filter = ModelFilter::new();
if let Some(search) = search_term {
    filter = filter.where_like("name", &format!("%{}%", search));
    // Note: OR conditions not yet supported, use multiple queries if needed
}
filter = filter.where_eq("is_active", true);

// Date range filter
let date_filter = ModelFilter::new()
    .where_gte("created_at", start_date)
    .where_lte("created_at", end_date);

// Complex status filter
let status_filter = ModelFilter::new()
    .where_in("status", vec!["active", "pending", "verified"])
    .where_not_null("approved_at");

// Null checks filter
let complete_profile = ModelFilter::new()
    .where_not_null("email")
    .where_not_null("phone")
    .where_not_null("address");
```

### Reusable Application Filters

Create application-wide filters as functions:

```rust
// In your models or a filters module
impl Users {
    pub fn active_filter() -> ModelFilter {
        ModelFilter::new()
            .where_eq("is_active", true)
            .where_not_null("email_verified_at")
    }
    
    pub fn admin_filter() -> ModelFilter {
        ModelFilter::new()
            .where_eq("role", "admin")
            .where_eq("is_active", true)
    }
}

// Usage
let admins = Users::query()?
    .apply_filter(&Users::admin_filter())
    .get()
    .await?;
```

## Advanced Query Features

### Selecting Specific Fields

Instead of fetching all columns with SELECT *, you can specify exactly which fields you need:

```rust
// Select specific fields only
let users = Users::query()?
    .select(&["id", "name", "email"])
    .where_eq("is_active", true)
    .get()
    .await?;

// With table prefixes (useful for JOINs)
let results = Posts::query()?
    .alias("p")
    .select(&["p.id", "p.title", "u.name as author_name"])
    .join("users AS u", "u.id = p.user_id")
    .get_raw()
    .await?;

// Raw SQL expressions for aggregations
let stats = Users::query()?
    .select_raw(&[
        "department",
        "COUNT(*) as user_count",
        "AVG(salary) as avg_salary"
    ])
    .group_by(&["department"])
    .get_raw()
    .await?;
```

### Table Aliasing

Use `alias()` to set a table alias, essential for self-joins and improving query readability:

```rust
// Simple aliasing
let users = Users::query()?
    .alias("u")
    .select(&["u.id", "u.name", "u.email"])
    .where_eq("u.is_active", true)
    .get()
    .await?;

// Self-join for hierarchical data (employee-manager)
let employees = Users::query()?
    .alias("emp")
    .select(&[
        "emp.id as employee_id",
        "emp.name as employee_name",
        "mgr.id as manager_id",
        "mgr.name as manager_name"
    ])
    .left_join("users AS mgr", "mgr.id = emp.manager_id")
    .get_raw()
    .await?;
```

### Advanced JOIN Queries

```rust
// Simple JOIN
let posts_with_authors = Posts::query()?
    .join("users", "users.id = posts.user_id")
    .where_eq("users.is_active", true)
    .get()
    .await?;

// LEFT JOIN with aliases to avoid column conflicts
let posts = Posts::query()?
    .alias("p")
    .select(&[
        "p.id as post_id",
        "p.title",
        "p.created_at as post_date",
        "u.id as user_id",
        "u.name as author_name",
        "COUNT(c.id) as comment_count"
    ])
    .join("users AS u", "u.id = p.user_id")
    .left_join("comments AS c", "c.post_id = p.id")
    .where_eq("p.is_published", true)
    .group_by(&["p.id", "p.title", "p.created_at", "u.id", "u.name"])
    .get_raw()
    .await?;
```

#### Handling Column Name Conflicts in JOINs

When joining tables with identical column names (like `id`, `created_at`), always use column aliases:

```rust
// ❌ BAD: Ambiguous column names
let results = Posts::query()?
    .join("users", "users.id = posts.user_id")
    .get_raw()
    .await?;
// Which 'id' is which? posts.id or users.id?

// ✅ GOOD: Clear column aliases
let results = Posts::query()?
    .alias("p")
    .select(&[
        "p.id as post_id",
        "p.created_at as post_date",
        "u.id as user_id",
        "u.created_at as user_joined"
    ])
    .join("users AS u", "u.id = p.user_id")
    .get_raw()
    .await?;
```

### GROUP BY and Aggregations

Perform aggregate queries using `group_by()` with `select_raw()`:

```rust
// Department statistics
let dept_stats = Users::query()?
    .alias("u")
    .select_raw(&[
        "u.department",
        "COUNT(*) as total_users",
        "AVG(u.salary) as avg_salary",
        "MAX(u.salary) as max_salary",
        "MIN(u.salary) as min_salary"
    ])
    .where_eq("u.is_active", true)
    .group_by(&["u.department"])
    .order_by("total_users", OrderDirection::Desc)
    .get_raw()
    .await?;

// Posts per user with filtering
let user_post_counts = Users::query()?
    .alias("u")
    .select_raw(&[
        "u.id",
        "u.name",
        "COUNT(p.id) as post_count",
        "MAX(p.created_at) as latest_post"
    ])
    .left_join("posts AS p", "p.user_id = u.id AND p.is_published = true")
    .group_by(&["u.id", "u.name"])
    .order_by("post_count", OrderDirection::Desc)
    .limit(10)
    .get_raw()
    .await?;
```

### Working with Raw Query Results

The `get_raw()` method returns `Vec<serde_json::Value>` for flexible result handling:

```rust
// Execute raw query
let results = Users::query()?
    .alias("u")
    .select_raw(&[
        "u.department",
        "COUNT(*) as count",
        "AVG(u.salary) as avg_salary"
    ])
    .group_by(&["u.department"])
    .get_raw()
    .await?;

// Loop over results
for row in &results {
    let dept = row["department"].as_str().unwrap_or("Unknown");
    let count = row["count"].as_i64().unwrap_or(0);
    let avg = row["avg_salary"].as_f64().unwrap_or(0.0);
    
    println!("Department: {} - {} users, avg salary: ${:.2}", dept, count, avg);
}

// Transform for view consumption
let view_data = results.iter().map(|row| {
    json!({
        "department": row["department"],
        "userCount": row["count"],
        "avgSalary": format!("${:.2}", row["avg_salary"].as_f64().unwrap_or(0.0))
    })
}).collect::<Vec<_>>();

// Pass to controller view
ctx.view("departments", json!({
    "stats": view_data,
    "total": results.len()
}))
```

### Combining Filters with Complex Queries

ModelFilter works seamlessly with all the new query features:

```rust
// Create reusable filter
let active_filter = ModelFilter::new()
    .where_eq("u.is_active", true)
    .where_not_null("u.verified_at");

// Use with SELECT and JOIN
let users = Users::query()?
    .alias("u")
    .select(&["u.id", "u.name", "COUNT(p.id) as posts"])
    .apply_filter(&active_filter)
    .left_join("posts AS p", "p.user_id = u.id")
    .group_by(&["u.id", "u.name"])
    .get_raw()
    .await?;
```

## Working with Models

### Field Access

Generated models provide getters and setters:

```rust
let user = Users::find(123).await?.unwrap();

// Getters
let email = user.email();        // &str for String fields
let id = user.id();              // i32/i64 for ID
let active = user.is_active();   // bool
let status = user.status();      // Option<&str> for enum
let created = user.created_at(); // Option<DateTime<Utc>>

// Setters (track changes automatically)
let mut user = user;
user.set_email("new@example.com");
user.set_is_active(false);

// Enum setters - PostgreSQL type casting is automatic
user.set_status(Some(Users::STATUS_ACTIVE));  // Uses constant
user.set_role("admin");  // Direct string also works

// Check what changed
if user.is_changed("email") {
    println!("Email was modified");
}

let changed_fields = user.changed_fields(); // Vec<String>
```

### Working with NULL Values

```rust
// Setting NULL values
user.set_email_verified(None);  // Sets to NULL
user.set_email_verified(Some(Utc::now()));  // Sets value

// Checking for NULL
let users = Users::query()?
    .where_null("email_verified")
    .get()
    .await?;
```

### Working with Enums

RustF provides intelligent enum handling with automatic PostgreSQL type casting:

```rust
// Enum constants are generated for each enum field
user.set_status(Some(Users::STATUS_ACTIVE));      // "active"
user.set_role(Users::ROLE_ADMIN);                 // "admin"

// PostgreSQL setters automatically handle type casting
// The setter adds ::enum_type_name suffix when needed
user.set_status(Some("active"));  // Becomes "active::status_enum" for PostgreSQL

// For query builders, use field-specific converter methods
let active_users = Users::query()?
    .where_eq("status", Users::as_status_enum("active"))
    .get()
    .await?;

// Converter methods handle database-specific requirements:
// PostgreSQL: Returns "active::status_enum" 
// MySQL/SQLite: Returns "active" (pass-through)
let admins = Users::query()?
    .where_eq("role", Users::as_role_enum("admin"))
    .where_eq("status", Users::as_status_enum("active"))
    .get()
    .await?;
```

#### Enum Constants

Generated models include constants for all enum values:

```rust
// Generated constants in your model
pub const STATUS_ACTIVE: &'static str = "active";
pub const STATUS_INACTIVE: &'static str = "inactive";
pub const STATUS_PENDING: &'static str = "pending";

pub const ROLE_ADMIN: &'static str = "admin";
pub const ROLE_USER: &'static str = "user";
pub const ROLE_MODERATOR: &'static str = "moderator";
```

#### Enum Converter Methods

Each enum field gets a converter method for query builder compatibility:

```rust
// Generated converter methods
pub fn as_status_enum(value: &str) -> String {
    // PostgreSQL: Adds type suffix
    // MySQL/SQLite: Pass-through
}

pub fn as_role_enum(value: &str) -> String {
    // Database-specific handling
}
```

### Custom Business Logic

Add your methods to the wrapper model:

```rust
// src/models/users.rs
impl Users {
    // Custom finder
    pub async fn find_by_email(email: &str) -> Result<Option<Self>> {
        Self::query()?
            .where_eq("email", email)
            .first()
            .await
    }
    
    // Business logic
    pub async fn verify_email(&mut self) -> Result<()> {
        self.set_email_verified(Some(Utc::now()));
        self.set_is_active(true);
        self.update().await
    }
    
    // Complex queries
    pub async fn find_inactive_users(days: i64) -> Result<Vec<Self>> {
        let cutoff = Utc::now() - Duration::days(days);
        Self::query()?
            .where_eq("is_active", false)
            .where_lt("last_login", cutoff.to_rfc3339())
            .get()
            .await
    }
}
```

## Type Safety

### Using Type Constants

Instead of magic strings, use generated type constants:

```rust
// ❌ Avoid
let users = Users::query()?.where_eq("email", email).get().await?;

// ✅ Prefer (compile-time checked)
let users = Users::query()?
    .where_eq(Users::columns::EMAIL, email)
    .get()
    .await?;

// For enums, use converter methods in queries
// ❌ Avoid - Won't work correctly with PostgreSQL
let users = Users::query()?
    .where_eq("status", "active")
    .get()
    .await?;

// ✅ Correct - Handles database-specific enum types
let users = Users::query()?
    .where_eq("status", Users::as_status_enum("active"))
    .get()
    .await?;
```

### SqlValue Conversions

The query builder accepts many types through `Into<SqlValue>`:

```rust
// All of these work
query.where_eq("age", 25);           // i32
query.where_eq("age", 25i64);        // i64
query.where_eq("name", "John");      // &str
query.where_eq("name", String::from("John")); // String
query.where_eq("name", &name);       // &String (reference)
query.where_eq("active", true);      // bool
query.where_eq("created", Utc::now()); // DateTime
query.where_eq("data", json!({"key": "value"})); // JSON
```

## Counting and Aggregation

### Count Operations

```rust
// Count all records
let total = Users::count().await?;

// Count with conditions
let active_count = Users::query()?
    .where_eq("is_active", true)
    .count()
    .await?;

// Count by group (using raw SQL for now)
// Future: Add group_by support
```

### Existence Checks

```rust
// Check if any users exist
let has_users = Users::query()?.exists().await?;

// Check with conditions
let has_admins = Users::query()?
    .where_eq("role", "admin")
    .exists()
    .await?;
```

## Pagination

```rust
// Simple pagination
let page = 2;
let per_page = 20;
let users = Users::paginate(page, per_page).await?;

// Pagination with conditions
let active_users = Users::query()?
    .where_eq("is_active", true)
    .order_by("created_at", OrderDirection::Desc)
    .paginate(page, per_page)
    .get()
    .await?;

// Manual limit/offset
let users = Users::query()?
    .limit(20)
    .offset(40)  // Skip first 40
    .get()
    .await?;
```

## Error Handling

All database operations return `Result<T>`:

```rust
use rustf::Result;

async fn get_user(id: i32) -> Result<Users> {
    match Users::find(id).await? {
        Some(user) => Ok(user),
        None => Err(rustf::Error::NotFound(format!("User {} not found", id))),
    }
}

// Handle different cases
match Users::find(id).await {
    Ok(Some(user)) => {
        // User found
    },
    Ok(None) => {
        // User not found
    },
    Err(e) => {
        // Database error
        log::error!("Database error: {}", e);
    }
}
```

## Best Practices

### 1. Use Query Builder Over Raw SQL

```rust
// ✅ Good - Type-safe, database-agnostic
let users = Users::query()?
    .where_eq("is_active", true)
    .get()
    .await?;

// ❌ Avoid - Database-specific, prone to SQL injection
let users = DB::execute_raw("SELECT * FROM users WHERE is_active = 1").await?;
```

### 2. Always Check Option Returns

```rust
// ✅ Good - Handle None case
if let Some(user) = Users::find(id).await? {
    // Work with user
} else {
    // Handle not found
}

// ❌ Bad - Will panic on None
let user = Users::find(id).await?.unwrap();
```

### 3. Use Transactions for Multiple Operations

```rust
// Future: Transaction support planned
// For now, ensure operations are idempotent
```

### 4. Leverage Change Tracking

```rust
// ✅ Good - Only updates changed fields
let mut user = Users::find(id).await?.unwrap();
user.set_email("new@example.com");
user.update().await?;  // Only updates email

// ❌ Less efficient - Updates all fields
// (Would update everything if using a different ORM)
```

### 5. Use Builder Pattern for Creation

```rust
// ✅ Good - Clear, validated
let user = Users::builder()
    .email("user@example.com")
    .username("johndoe")
    .status(Some(Users::STATUS_ACTIVE))  // Use enum constants
    .role(Users::ROLE_USER)
    .save()
    .await?;

// ❌ More verbose - Manual struct creation
let user = Users { /* all fields */ };
```

### 6. Use Enum Converter Methods in Queries

```rust
// ✅ Good - Works across all databases
let active_admins = Users::query()?
    .where_eq("status", Users::as_status_enum("active"))
    .where_eq("role", Users::as_role_enum("admin"))
    .get()
    .await?;

// ❌ Bad - May fail with PostgreSQL enums
let active_admins = Users::query()?
    .where_eq("status", "active")  // Missing type suffix for PostgreSQL
    .where_eq("role", "admin")
    .get()
    .await?;

// ✅ Good - Using constants with converter
let pending = Users::query()?
    .where_eq("status", Users::as_status_enum(Users::STATUS_PENDING))
    .get()
    .await?;
```

## Configuration Access

Use the global `CONF` for configuration values:

```rust
use rustf::CONF;

// Get database URL
let db_url = CONF::get_string("database.url");

// Get with default
let max_conn = CONF::get_or("database.max_connections", 10);

// Check feature flags
if CONF::get_bool("features.advanced_search").unwrap_or(false) {
    // Advanced search enabled
}
```

## Performance Tips

1. **Use `exists()` instead of `count() > 0`**
   ```rust
   // ✅ Faster
   let has_users = Users::query()?.exists().await?;
   
   // ❌ Slower
   let has_users = Users::query()?.count().await? > 0;
   ```

2. **Select only needed columns**
   ```rust
   // ✅ Good - Only fetch what you need
   let users = Users::query()?
       .select(&["id", "name", "email"])
       .where_eq("is_active", true)
       .get()
       .await?;
   
   // ❌ Avoid - Fetches all columns
   let users = Users::query()?
       .where_eq("is_active", true)
       .get()
       .await?;
   ```

3. **Use pagination for large datasets**
   ```rust
   // ✅ Good
   let users = Users::paginate(1, 100).await?;
   
   // ❌ Bad for large tables
   let all_users = Users::all().await?;
   ```

4. **Batch operations when possible**
   ```rust
   // Future: Bulk insert/update support
   ```

## Debugging

### Enable SQL Logging

```toml
# In config.toml
[logging]
level = "debug"  # Shows SQL queries
```

### Inspect Generated SQL

```rust
// Get the SQL without executing
let (sql, params) = Users::query()?
    .where_eq("email", "test@example.com")
    .to_sql()?;
    
println!("SQL: {}", sql);
println!("Params: {:?}", params);
```

## Multi-Database Support

RustF automatically handles dialect differences:

```rust
// This works on PostgreSQL, MySQL, and SQLite
let users = Users::query()?
    .where_like("email", "%@example.com")
    .limit(10)
    .get()
    .await?;

// Behind the scenes:
// PostgreSQL: "email LIKE $1 LIMIT 10"
// MySQL: "email LIKE ? LIMIT 10"  
// SQLite: "email LIKE ? LIMIT 10"
```

## Limitations & Future Features

### Current Limitations
- No transaction support yet
- No raw SQL bindings (only MySQL has execute_raw)
- No HAVING clause support (GROUP BY is supported)
- Limited aggregate function helpers (use select_raw() for now)

### Coming Soon
- Transaction support
- Migrations integration
- Bulk insert/update
- Query caching
- Connection retry logic

## Summary

RustF's database layer provides:
- **Zero boilerplate** - No connection passing
- **Type safety** - Compile-time query validation
- **Smart updates** - Change tracking built-in
- **Multi-database** - Write once, run anywhere
- **Developer friendly** - Intuitive API inspired by Laravel

Start with schemas, generate models, and enjoy a modern database experience in Rust!# 🗄️ RustF Multi-Database Query Builder

A type-safe, AI-friendly SQL query builder for Rust that supports PostgreSQL, MySQL, MariaDB, and SQLite with a unified API. Part of the RustF web framework ecosystem.

> 📖 **Complete Guide:** For comprehensive documentation including model generation, CLI tools, and AI agent guidelines, see the [CLI Tool Guide](../advanced/cli.md) and [Schemas Guide](schemas.md)

## 🌟 Features

- 🚀 **Type-safe query building** with compile-time validation
- 🔄 **Multi-database support** (PostgreSQL, MySQL, MariaDB, SQLite) 
- 🤖 **AI-friendly design** with predictable method names and clear error messages
- 📝 **Automatic SQL dialect handling** for cross-database compatibility
- 🛡️ **SQL injection protection** through parameterized queries
- 📊 **Schema builder** with database-specific type mapping
- 🔍 **Comprehensive WHERE clauses** including LIKE, IN, BETWEEN, NULL checks
- 🔗 **All JOIN types** (INNER, LEFT, RIGHT, FULL, CROSS)
- 📄 **Built-in pagination** and aggregation helpers
- ⚡ **Zero-cost abstractions** leveraging Rust's type system
- 🏗️ **Framework integration** with RustF model system and CLI tools

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Database Connection](#database-connection)
- [Query Building](#query-building)
  - [Basic Queries](#basic-queries)
  - [WHERE Conditions](#where-conditions)
  - [JOIN Operations](#join-operations)
  - [Aggregations](#aggregations)
  - [Ordering and Limiting](#ordering-and-limiting)
- [Schema Builder](#schema-builder)
- [Error Handling](#error-handling)
- [Database-Specific Features](#database-specific-features)
- [Examples](#examples)
- [API Reference](#api-reference)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
query-builder = "0.1.0"
sqlx = { version = "0.8", features = ["postgres", "mysql", "sqlite", "runtime-tokio-rustls", "uuid", "chrono", "rust_decimal"] }
tokio = { version = "1", features = ["full"] }
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = "0.1"
```

## ⚡ Quick Start

### 1. Using with RustF Models (Recommended)

```rust
use rustf::models::Users;

// Simple model operations
let user = Users::find(123).await?;
let admins = Users::where_eq("role", "admin").await?;

// Model-scoped query builder
let active_users = Users::query()?
    .where_eq("is_active", true)
    .where_gt("age", 18)
    .where_like("email", "%@gmail.com")
    .order_by("created_at", OrderDirection::Desc)
    .limit(10)
    .get()
    .await?;
```

### 2. Direct Query Builder Usage

```rust
use rustf::models::{AnyDatabase, QueryBuilder, OrderDirection};

#[tokio::main] 
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to any supported database
    let db = AnyDatabase::connect("postgresql://localhost/myapp").await?;
    
    // Build and execute a query
    let (sql, params) = db.query()
        .from("users")
        .where_gt("age", 18)
        .where_like("email", "%@gmail.com")
        .order_by("created_at", OrderDirection::Desc)
        .limit(10)
        .build()?;
    
    println!("Generated SQL: {}", sql);
    println!("Parameters: {:?}", params);
    
    Ok(())
}
```

### 3. Generate Models from Schema

```bash
# Create schema file
cat > schemas/users.yaml << EOF
name: users
table: users
fields:
  id: { type: integer, primary_key: true, auto_increment: true }
  name: { type: string, max_length: 100, required: true }
  email: { type: string, max_length: 255, unique: true, required: true }
  created_at: { type: timestamp, default: now }
EOF

# Generate models
rustf-cli schema generate models --schema-path schemas --output src/models
```

## Database Connection

The library automatically detects the database type from the connection URL:

```rust
// PostgreSQL
let pg_db = AnyDatabase::connect("postgresql://user:pass@localhost/dbname").await?;

// MySQL
let mysql_db = AnyDatabase::connect("mysql://user:pass@localhost/dbname").await?;

// SQLite
let sqlite_db = AnyDatabase::connect("sqlite://path/to/database.db").await?;

// In-memory SQLite
let memory_db = AnyDatabase::connect("sqlite::memory:").await?;
```

## Query Building

### Basic Queries

```rust
// Select all columns
let query = db.query()
    .from("users")
    .build()?;
// SELECT * FROM "users" (PostgreSQL)
// SELECT * FROM `users` (MySQL)

// Select specific columns
let query = db.query()
    .select(vec!["id", "name", "email"])
    .from("users")
    .build()?;
// SELECT id, name, email FROM "users"

// Count query
let count = db.query()
    .from("users")
    .count()
    .build()?;
// SELECT COUNT(*) FROM "users"
```

### WHERE Conditions

The library provides a comprehensive set of WHERE operations:

```rust
// Basic comparisons
query.where_eq("status", "active")      // WHERE status = ?
query.where_ne("status", "deleted")     // WHERE status <> ?
query.where_gt("age", 18)               // WHERE age > ?
query.where_gte("age", 18)              // WHERE age >= ?
query.where_lt("price", 100)            // WHERE price < ?
query.where_lte("price", 100)           // WHERE price <= ?

// Pattern matching
query.where_like("email", "%@gmail.com")     // WHERE email LIKE ?
query.where_not_like("email", "%spam%")      // WHERE email NOT LIKE ?

// IN clauses
query.where_in("status", vec!["active", "pending"])      // WHERE status IN (?, ?)
query.where_not_in("role", vec!["admin", "super"])       // WHERE role NOT IN (?, ?)

// NULL checks
query.where_null("deleted_at")          // WHERE deleted_at IS NULL
query.where_not_null("verified_at")     // WHERE verified_at IS NOT NULL

// Range queries
query.where_between("age", 18, 65)      // WHERE age BETWEEN ? AND ?

// OR conditions
query.where_eq("status", "active")
     .or_where_eq("role", "admin")      // WHERE status = ? OR role = ?

// Raw SQL conditions
query.where_raw("(status = 'active' OR created_at > NOW() - INTERVAL '1 day')")
```

### Combining Conditions

```rust
let users = db.query()
    .from("users")
    .where_eq("active", true)
    .where_gte("age", 18)
    .where_like("email", "%@%")
    .where_not_null("verified_at")
    .build()?;
// WHERE active = ? AND age >= ? AND email LIKE ? AND verified_at IS NOT NULL
```

### JOIN Operations

All standard SQL joins are supported:

```rust
// INNER JOIN
query.join("posts", "posts.user_id = users.id")

// LEFT JOIN
query.left_join("posts", "posts.user_id = users.id")

// RIGHT JOIN (not supported in SQLite)
query.right_join("posts", "posts.user_id = users.id")?

// FULL JOIN (not supported in MySQL/MariaDB)
query.full_join("posts", "posts.user_id = users.id")?

// CROSS JOIN
query.cross_join("categories")

// Complex join example
let results = db.query()
    .select(vec!["u.name", "u.email", "COUNT(p.id) as post_count"])
    .from("users u")
    .left_join("posts p", "p.user_id = u.id")
    .where_eq("u.active", true)
    .group_by(vec!["u.id", "u.name", "u.email"])
    .having("COUNT(p.id)", ">", 5)
    .order_by("post_count", OrderDirection::Desc)
    .build()?;
```

### Aggregations

Built-in aggregation helpers:

```rust
// Simple count
let count = db.query()
    .from("users")
    .count()
    .where_eq("active", true)
    .build()?;

// Count specific column
let count = db.query()
    .from("orders")
    .count_column("DISTINCT user_id")
    .build()?;

// Multiple aggregations
let stats = db.query()
    .from("orders")
    .aggregate(vec![
        ("COUNT", "*"),
        ("SUM", "total"),
        ("AVG", "total"),
        ("MAX", "total"),
        ("MIN", "total")
    ])
    .where_gte("created_at", "2024-01-01")
    .group_by(vec!["product_id"])
    .build()?;
```

### Ordering and Limiting

```rust
// Simple ordering
query.order_by("created_at", OrderDirection::Desc)

// Multiple order by
query.order_by_multiple(vec![
    ("status", OrderDirection::Asc),
    ("created_at", OrderDirection::Desc)
])

// Limiting results
query.limit(10)
query.offset(20)

// Pagination helper
query.paginate(2, 20)  // Page 2, 20 items per page
// Automatically calculates: LIMIT 20 OFFSET 20
```

## Schema Builder

Create database tables with automatic type mapping:

```rust
use query_builder::{SchemaBuilder, DatabaseBackend};

let schema = SchemaBuilder::new(DatabaseBackend::Postgres);

let sql = schema.create_table("users")
    .id()                                    // SERIAL PRIMARY KEY
    .string("email", Some(255))
        .not_null()
        .unique()                           // VARCHAR(255) NOT NULL UNIQUE
    .string("name", Some(100))              // VARCHAR(100)
    .boolean("active")
        .default("TRUE")                    // BOOLEAN DEFAULT TRUE
    .timestamp("created_at")
        .default("CURRENT_TIMESTAMP")       // TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    .build();

// Different databases get appropriate types:
// PostgreSQL: SERIAL, VARCHAR, BOOLEAN, TIMESTAMP WITH TIME ZONE
// MySQL: INT AUTO_INCREMENT, VARCHAR, TINYINT(1), DATETIME
// SQLite: INTEGER PRIMARY KEY AUTOINCREMENT, TEXT, INTEGER, TEXT
```

### Available Column Types

```rust
.id()                           // Auto-incrementing primary key
.uuid("column_name")            // UUID (or equivalent)
.string("column_name", Some(255))  // Variable-length string
.boolean("column_name")         // Boolean type
.timestamp("column_name")       // Timestamp/datetime
.integer("column_name")         // Integer
.float("column_name")           // Floating point
.text("column_name")            // Long text
```

### Column Modifiers

```rust
.not_null()                     // NOT NULL constraint
.unique()                       // UNIQUE constraint
.default("value")               // DEFAULT value
.primary_key()                  // PRIMARY KEY constraint
```

## Error Handling

The library provides AI-friendly error messages:

```rust
match result {
    Err(QueryError::MissingClause { clause }) => {
        // "Missing required clause: from. Add .from() to your query."
    },
    Err(QueryError::UnsupportedFeature { backend, feature }) => {
        // "Feature not supported in PostgreSQL: RIGHT JOIN"
    },
    Err(QueryError::InvalidColumn { column, available }) => {
        // "Invalid column name: 'usr_name'. Available columns: ['id', 'name', 'email']"
    },
    _ => {}
}
```

## Database-Specific Features

The library automatically handles database differences:

### Placeholders
- PostgreSQL: `$1, $2, $3`
- MySQL/SQLite: `?, ?, ?`

### Identifier Quoting
- PostgreSQL/SQLite: `"column_name"`
- MySQL/MariaDB: `` `column_name` ``

### RETURNING Clause
- Supported: PostgreSQL, SQLite
- Not supported: MySQL/MariaDB

### Boolean Types
- PostgreSQL: `BOOLEAN`
- MySQL/MariaDB: `TINYINT(1)`
- SQLite: `INTEGER`

## Examples

### Example 1: User Authentication Query

```rust
let user = db.query()
    .select(vec!["id", "email", "password_hash", "role"])
    .from("users")
    .where_eq("email", email)
    .where_eq("active", true)
    .where_null("deleted_at")
    .build()?;
```

### Example 2: Search with Pagination

```rust
let search_term = "%rust%";
let page = 2;

let posts = db.query()
    .select(vec!["id", "title", "content", "author_id", "created_at"])
    .from("posts")
    .where_like("title", &search_term)
    .or_where_like("content", &search_term)
    .where_eq("published", true)
    .order_by("created_at", OrderDirection::Desc)
    .paginate(page, 20)
    .build()?;
```

### Example 3: Dashboard Statistics

```rust
let stats = db.query()
    .from("orders")
    .aggregate(vec![
        ("COUNT", "*"),
        ("SUM", "total_amount"),
        ("AVG", "total_amount")
    ])
    .where_between("created_at", start_date, end_date)
    .where_eq("status", "completed")
    .group_by(vec!["DATE(created_at)"])
    .order_by("DATE(created_at)", OrderDirection::Asc)
    .build()?;
```

### Example 4: Complex Report Query

```rust
let report = db.query()
    .select(vec![
        "c.name as category",
        "p.name as product",
        "SUM(oi.quantity) as total_sold",
        "SUM(oi.quantity * oi.price) as revenue"
    ])
    .from("order_items oi")
    .join("orders o", "o.id = oi.order_id")
    .join("products p", "p.id = oi.product_id")
    .join("categories c", "c.id = p.category_id")
    .where_eq("o.status", "completed")
    .where_between("o.created_at", "2024-01-01", "2024-12-31")
    .group_by(vec!["c.id", "c.name", "p.id", "p.name"])
    .having("SUM(oi.quantity)", ">", 100)
    .order_by("revenue", OrderDirection::Desc)
    .limit(50)
    .build()?;
```

## API Reference

### QueryBuilder Methods

| Method | Description | Example |
|--------|-------------|---------|
| `select(columns)` | Select specific columns | `.select(vec!["id", "name"])` |
| `from(table)` | Specify table | `.from("users")` |
| `where_*` | WHERE conditions | `.where_eq("status", "active")` |
| `or_where_*` | OR WHERE conditions | `.or_where_eq("role", "admin")` |
| `join(table, on)` | INNER JOIN | `.join("posts", "posts.user_id = users.id")` |
| `left_join(table, on)` | LEFT JOIN | `.left_join("posts", "posts.user_id = users.id")` |
| `group_by(columns)` | GROUP BY | `.group_by(vec!["user_id"])` |
| `having(col, op, val)` | HAVING clause | `.having("COUNT(*)", ">", 5)` |
| `order_by(col, dir)` | ORDER BY | `.order_by("created_at", OrderDirection::Desc)` |
| `limit(n)` | LIMIT results | `.limit(10)` |
| `offset(n)` | OFFSET results | `.offset(20)` |
| `distinct()` | SELECT DISTINCT | `.distinct()` |
| `count()` | COUNT(*) | `.count()` |
| `paginate(page, per_page)` | Pagination helper | `.paginate(2, 20)` |

### WHERE Condition Methods

| Method | SQL Equivalent |
|--------|----------------|
| `where_eq(col, val)` | `WHERE col = val` |
| `where_ne(col, val)` | `WHERE col <> val` |
| `where_gt(col, val)` | `WHERE col > val` |
| `where_gte(col, val)` | `WHERE col >= val` |
| `where_lt(col, val)` | `WHERE col < val` |
| `where_lte(col, val)` | `WHERE col <= val` |
| `where_like(col, pattern)` | `WHERE col LIKE pattern` |
| `where_not_like(col, pattern)` | `WHERE col NOT LIKE pattern` |
| `where_in(col, values)` | `WHERE col IN (values)` |
| `where_not_in(col, values)` | `WHERE col NOT IN (values)` |
| `where_between(col, start, end)` | `WHERE col BETWEEN start AND end` |
| `where_null(col)` | `WHERE col IS NULL` |
| `where_not_null(col)` | `WHERE col IS NOT NULL` |

## Best Practices

1. **Use parameterized queries**: The library automatically parameterizes all values to prevent SQL injection.

2. **Check feature support**: Some features like RIGHT JOIN (SQLite) or FULL JOIN (MySQL) aren't universally supported. The library returns clear errors for unsupported features.

3. **Use type-safe values**: Pass proper Rust types that implement `Into<SqlValue>`.

4. **Handle errors appropriately**: Use the detailed error types to provide meaningful feedback to users.

5. **Leverage the schema builder**: Use it to ensure consistent table structures across different databases.

## 🤖 AI Agent Guidelines

When working with the RustF query builder:

**✅ Recommended:**
- Use model-scoped queries (`Users::query()`) for type safety
- Use type constants (`UsersBase::Types::email`) instead of hardcoding types
- Handle database-specific features gracefully with error checking
- Use the unified `AnyDatabase` API for cross-database compatibility

**❌ Avoid:**
- Hardcoding SQL strings when the query builder can generate them
- Ignoring database feature limitations (RIGHT JOIN in SQLite, etc.)
- Using raw parameters instead of type-safe SqlValue conversion

**Example AI-Friendly Error Handling:**
```rust
match query.right_join("posts", "posts.user_id = users.id") {
    Ok(q) => q,
    Err(QueryError::UnsupportedFeature { backend, feature }) => {
        // Fallback to LEFT JOIN for compatibility
        query.left_join("posts", "posts.user_id = users.id")?
    }
}
```

## 🔗 Related Documentation

- **[CLI Tool Guide](../advanced/cli.md)** - Database tools and code generation
- **[Schemas Guide](schemas.md)** - Schema-driven development
- **[Model Generation Guide](../../docs/MODEL_GENERATION.md)** - Schema-driven development
- **[RustF CLI Reference](../../docs/CLI_REFERENCE.md)** - Database and schema commands
- **[Multi-Database Best Practices](../../docs/MULTI_DATABASE.md)** - Cross-database compatibility

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Development Setup
```bash
# Clone the repository
git clone https://github.com/rustf/rustf.git
cd rustf

# Run tests
cargo test

# Test with different databases
docker-compose up -d postgres mysql
cargo test --features="postgres,mysql,sqlite"
```

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.
---

# Pagination Helper Guide

RustF provides a built-in pagination helper through the `U::paginate()` function that makes it easy to implement pagination in your web applications.

## Overview

The pagination helper creates a complete pagination object with:
- Page navigation (first, last, previous, next)
- Page number ranges
- URL generation with customizable patterns
- Template-friendly JSON output

## Basic Usage

### In Controllers

```rust
use rustf::prelude::*;

async fn list_users(ctx: &mut Context) -> Result<()> {
    // Parse page from query parameters
    let page = ctx.query("page")
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(1);
    
    let per_page = 20;
    
    // Get total count from database
    let total_users = Users::count().await?;
    
    // Fetch paginated data
    let users = Users::paginate(page, per_page).await?;
    
    // Create pagination object
    let pagination = U::paginate(
        total_users,           // Total items
        page,                  // Current page (1-based)
        per_page,              // Items per page
        "/users?page={0}"      // URL pattern with {0} placeholder
    );
    
    // Pass to view
    ctx.view("users/list", json!({
        "users": users,
        "pagination": pagination.to_json()
    }))
}
```

### In Templates (Total.js Syntax)

```html
<!-- Basic pagination controls -->
<div class="pagination">
    @{if pagination.isPrev}
        <a href="@{pagination.prev.url}">Previous</a>
    @{fi}
    
    @{foreach page in pagination.range}
        @{if page.selected}
            <span class="current">@{page.page}</span>
        @{else}
            <a href="@{page.url}">@{page.page}</a>
        @{fi}
    @{end}
    
    @{if pagination.isNext}
        <a href="@{pagination.next.url}">Next</a>
    @{fi}
</div>
```

### Complete Navigation Example

```html
<!-- Full pagination with first/last links -->
<div class="pagination">
    <!-- First & Previous -->
    @{if !pagination.isFirst}
        <a href="@{pagination.first.url}">« First</a>
    @{fi}
    
    @{if pagination.isPrev}
        <a href="@{pagination.prev.url}">‹ Previous</a>
    @{else}
        <span class="disabled">‹ Previous</span>
    @{fi}
    
    <!-- Page Numbers -->
    @{foreach page in pagination.range}
        @{if page.selected}
            <span class="current">@{page.page}</span>
        @{else}
            <a href="@{page.url}">@{page.page}</a>
        @{fi}
    @{end}
    
    <!-- Next & Last -->
    @{if pagination.isNext}
        <a href="@{pagination.next.url}">Next ›</a>
    @{else}
        <span class="disabled">Next ›</span>
    @{fi}
    
    @{if !pagination.isLast}
        <a href="@{pagination.last.url}">Last »</a>
    @{fi}
</div>

<!-- Page info -->
<p>Page @{pagination.page} of @{pagination.count} 
   (@{pagination.items} total items)</p>
```

## Pagination Object Structure

The `pagination.to_json()` method returns:

```json
{
  "items": 157,        // Total number of items
  "page": 5,           // Current page
  "count": 16,         // Total pages
  "per_page": 10,      // Items per page
  "isFirst": false,    // Is first page?
  "isLast": false,     // Is last page?
  "isPrev": true,      // Has previous page?
  "isNext": true,      // Has next page?
  "first": {
    "url": "/users?page=1"
  },
  "last": {
    "url": "/users?page=16"
  },
  "prev": {
    "url": "/users?page=4"
  },
  "next": {
    "url": "/users?page=6"
  },
  "range": [           // Page numbers for display
    {
      "page": 3,
      "url": "/users?page=3",
      "selected": false
    },
    {
      "page": 4,
      "url": "/users?page=4",
      "selected": false
    },
    {
      "page": 5,
      "url": "/users?page=5",
      "selected": true
    },
    // ... up to 7 pages by default
  ]
}
```

## Advanced Usage

### Custom URL Patterns

```rust
// Simple query parameter
let pagination = U::paginate(total, page, 20, "/posts?page={0}");

// With multiple parameters
let pagination = U::paginate(total, page, 20, "/posts?category=tech&page={0}");

// Path-based pagination
let pagination = U::paginate(total, page, 20, "/posts/page/{0}");

// With hash fragments
let pagination = U::paginate(total, page, 20, "/posts?page={0}#results");
```

### Pagination with Filters

```rust
async fn search_posts(ctx: &mut Context) -> Result<()> {
    let page = ctx.query("page")
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(1);
    let search = ctx.query("q").unwrap_or("");
    let category = ctx.query("category").unwrap_or("all");
    
    // Build query with filters
    let total = Posts::query()?
        .where_like("title", &format!("%{}%", search))
        .where_eq("category", category)
        .count()
        .await?;
    
    let posts = Posts::query()?
        .where_like("title", &format!("%{}%", search))
        .where_eq("category", category)
        .paginate(page, 20)
        .get()
        .await?;
    
    // Include filters in URL pattern
    let url_pattern = format!(
        "/search?q={}&category={}&page={{0}}", 
        U::encode(search),
        U::encode(category)
    );
    
    let pagination = U::paginate(total, page, 20, &url_pattern);
    
    ctx.view("search-results", json!({
        "posts": posts,
        "pagination": pagination.to_json(),
        "search": search,
        "category": category
    }))
}
```

## Styling Example

```css
.pagination {
    display: flex;
    justify-content: center;
    gap: 10px;
    margin: 20px 0;
}

.pagination a,
.pagination span {
    padding: 8px 12px;
    border: 1px solid #ddd;
    border-radius: 4px;
    text-decoration: none;
}

.pagination a:hover {
    background: #007bff;
    color: white;
}

.pagination .current {
    background: #007bff;
    color: white;
    font-weight: bold;
}

.pagination .disabled {
    color: #999;
    cursor: not-allowed;
}
```

## API Reference

### U::paginate()

```rust
pub fn paginate(
    total: i64,           // Total number of items
    page: u32,            // Current page (1-based)
    per_page: u32,        // Items per page
    url_pattern: &str     // URL pattern with {0} placeholder
) -> Pagination
```

### Pagination Methods

- `to_json()` - Convert to JSON for template use
- `is_first()` - Check if on first page
- `is_last()` - Check if on last page
- `has_prev()` - Check if previous page exists
- `has_next()` - Check if next page exists
- `first_url()` - Get URL for first page
- `last_url()` - Get URL for last page
- `prev_url()` - Get URL for previous page
- `next_url()` - Get URL for next page
- `range(max_items)` - Get page number range for display

## Best Practices

1. **Always validate page numbers** - Ensure page is within valid range
2. **Use reasonable per_page limits** - Typically 10-100 items
3. **Cache total counts** - For large datasets, consider caching counts
4. **Include page info** - Show "Page X of Y" for better UX
5. **Provide direct navigation** - Include first/last links for long lists
6. **Make it accessible** - Use proper ARIA labels and semantic HTML
7. **Handle edge cases** - Empty results, single page, invalid page numbers

## Example: Complete Implementation

See `/rustf-example/src/controllers/pagination_demo.rs` and `/rustf-example/views/pagination-demo.html` for a complete working example.