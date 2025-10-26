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

Start with schemas, generate models, and enjoy a modern database experience in Rust!