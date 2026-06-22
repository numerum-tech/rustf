# Model API Reference

RustF provides a database model system with query builders and type-safe database access.

## Model Traits

### BaseModel

All database models implement the `BaseModel` trait. It uses associated constants
(`TABLE_NAME`, `PRIMARY_KEY`) and an associated type (`IdType`) rather than methods.
The trait is `async` (it uses `#[async_trait]`).

```rust
#[async_trait]
pub trait BaseModel: ChangeTracking + Sized + Clone + Send + Sync + 'static
where
    Self: Serialize + for<'de> Deserialize<'de>,
{
    /// The Rust type of the primary key (e.g. i32, i64, String, Uuid)
    type IdType: Clone + Send + Sync + Display + Into<SqlValue> + 'static;

    /// The database table name
    const TABLE_NAME: &'static str;

    /// The primary key column name (e.g. "id")
    const PRIMARY_KEY: &'static str;

    /// Get the ID value of this instance
    fn id(&self) -> Self::IdType;

    // ... plus static helpers (query, get_by_id, get_all, count, ...)
    // and instance methods (update, delete) shown below.
}
```

A generated model declares these like so:

```rust
impl BaseModel for Users {
    type IdType = i32;
    const TABLE_NAME: &'static str = "users";
    const PRIMARY_KEY: &'static str = "id";

    fn id(&self) -> Self::IdType {
        self.id
    }
    // ...
}
```

### ModelQuery

`ModelQuery<T>` is a **struct** (not a trait) that wraps the query builder and
returns typed results. You obtain one from `Model::query()` and chain methods on it.

```rust
pub struct ModelQuery<T> { /* ... */ }
```

## Query Builder

### Basic Queries

All of these are `async` and return `rustf::Result`.

```rust
use rustf::prelude::*;

// Fetch by primary key -> Result<Option<Users>>
let user = Users::get_by_id(1).await?;

// Fetch all rows -> Result<Vec<Users>>
let users = Users::get_all().await?;

// Query with conditions. `query()` returns Result, so it needs `?`.
// Terminate the chain with `.get()` (Vec) or `.get_first()` (Option).
let active_users = Users::query()?
    .where_eq("is_active", true)
    .get()
    .await?;
```

### Where Clauses

```rust
Users::query()?
    .where_eq("status", "active")
    .where_ne("deleted", true)
    .where_gt("age", 18)
    .where_lt("created_at", "2024-01-01")
    .where_like("name", "%john%")
    .where_in("id", vec![1, 2, 3])
    .get()
    .await?;
```

### Ordering

```rust
Users::query()?
    .order_by("created_at", OrderDirection::Desc)
    .get()
    .await?;
```

### Pagination

```rust
// Simple pagination (legacy)
Users::query()?
    .limit(10)
    .offset(20)
    .get_all()
    .await?;

// Pagination with metadata (recommended)
let result = Users::query()?
    .where_eq("is_active", true)
    .get_paginated(2, 20)
    .await?;

// Access data and metadata
for user in &result.rows {
    println!("User: {}", user.username);
}
println!("Page {} of {}", result.page, result.total_pages);
println!("Total: {}", result.total_rows);

// Convert to Pagination for templates
let pagination = U::pagination_from_paged_result(&result, "/users?page={0}");
```

### Aggregations

`count()` is an async terminator on `ModelQuery`. There are no dedicated `max()` /
`avg()` helpers — for those, use `select_raw(...)` with `get_raw()`, which returns
the rows as `serde_json::Value`.

```rust
// Count matching rows -> Result<i64>
let count = Users::query()?
    .where_eq("is_active", true)
    .count()
    .await?;

// MAX / AVG via raw SELECT expressions
let rows = Users::query()?
    .select_raw(&["MAX(age) as max_age", "AVG(score) as avg_score"])
    .get_raw()
    .await?;

let max_age = rows[0]["max_age"].as_i64().unwrap_or(0);
let avg_score = rows[0]["avg_score"].as_f64().unwrap_or(0.0);
```

## Model Operations

`BaseModel` itself exposes only `update(&mut self)` and `delete(self)` for
persistence — there is no `create()` or `save()` on the trait. New records are
inserted through the **generated builder**, whose `save()` validates the fields
and performs the INSERT.

### Create

```rust
// Each generated model has a typed builder with one method per column.
let user = Users::builder()
    .username("jdoe")
    .email("john@example.com")
    .first_name("John")
    .last_name("Doe")
    .save()
    .await?; // -> rustf::Result<Users>
```

### Update

`update()` uses change tracking and only writes the fields you modified via the
generated `set_*` setters.

```rust
let mut user = Users::get_by_id(1).await?.unwrap();
user.set_first_name("Jane");
user.update().await?; // consumes &mut self
```

### Delete

```rust
let user = Users::get_by_id(1).await?.unwrap();
user.delete().await?; // consumes self
```

## Global Database Access

`DB::query()` takes **no arguments** and returns a `Result<QueryBuilder>` you build
up programmatically. For raw parameterized SQL, use `DB::fetch_all_with_params`
(returns rows as `serde_json::Value`) or `DB::execute_with_params` (returns the
number of affected rows).

```rust
use rustf::db::DB;
use rustf::database::types::SqlValue;

// Start a programmatic query builder (no SQL string, no args)
let qb = DB::query()?;

// Raw parameterized SELECT -> Result<Vec<serde_json::Value>>
let rows = DB::fetch_all_with_params(
    "SELECT * FROM users WHERE id = ?",
    vec![SqlValue::Int(1)],
).await?;

// Raw parameterized INSERT/UPDATE/DELETE -> Result<u64> (rows affected)
let affected = DB::execute_with_params(
    "UPDATE users SET is_active = ? WHERE id = ?",
    vec![SqlValue::Bool(false), SqlValue::Int(1)],
).await?;
```

> **Transactions:** there is no `DB::transaction(...)` helper in the current API.
> Use raw SQL (`BEGIN` / `COMMIT` / `ROLLBACK`) via `DB::execute_with_params`, or
> the underlying SQLx pool obtained from `DB` (e.g. `DB::sqlite_pool()`), if you
> need transactional control.

## Model Registration

Models are auto-discovered:

```rust
let app = RustF::new()
    .models(auto_models!());
```

## Examples

### Complete Model Usage

```rust
// Find user by primary key
let user = Users::get_by_id(1).await?;

// Query with conditions
let active_users = Users::query()?
    .where_eq("is_active", true)
    .order_by("created_at", OrderDirection::Desc)
    .limit(10)
    .get()
    .await?;

// Create new user via the generated builder
let new_user = Users::builder()
    .username("alice")
    .email("alice@example.com")
    .first_name("Alice")
    .save()
    .await?;

// Update user (only changed fields are written)
let mut user = Users::get_by_id(1).await?.unwrap();
user.set_first_name("Bob");
user.update().await?;

// Delete user
let user = Users::get_by_id(1).await?.unwrap();
user.delete().await?;
```







