# 🗄️ RustF Multi-Database Query Builder

A type-safe, AI-friendly SQL query builder for Rust that supports PostgreSQL, MySQL, MariaDB, and SQLite with a unified API. Part of the RustF web framework ecosystem.

> 📖 **Complete Guide:** For comprehensive documentation including model generation, CLI tools, and AI agent guidelines, see the [Database Tools Guide](../../docs/DATABASE_TOOLS_GUIDE.md)

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

- **[Complete Database Tools Guide](../../docs/DATABASE_TOOLS_GUIDE.md)** - Comprehensive documentation
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