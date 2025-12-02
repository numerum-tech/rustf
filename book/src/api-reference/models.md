# Model API Reference

RustF provides a database model system with query builders and type-safe database access.

## Model Traits

### DatabaseModel

```rust
pub trait DatabaseModel: Send + Sync {
    fn table_name(&self) -> &str;
    fn primary_key(&self) -> &str;
}
```

### ModelQuery

```rust
pub trait ModelQuery {
    fn find_by_id(&self, id: i64) -> Result<Option<Self>>;
    fn find_all(&self) -> Result<Vec<Self>>;
    fn create(&mut self) -> Result<()>;
    fn update(&mut self) -> Result<()>;
    fn delete(&self) -> Result<()>;
}
```

## Query Builder

### Basic Queries

```rust
use rustf::models::ModelQuery;

// Find by ID
let user = Users::find_by_id(1)?;

// Find all
let users = Users::find_all()?;

// Find with conditions
let active_users = Users::query()
    .where_eq("is_active", true)
    .find()?;
```

### Where Clauses

```rust
Users::query()
    .where_eq("status", "active")
    .where_ne("deleted", true)
    .where_gt("age", 18)
    .where_lt("created_at", "2024-01-01")
    .where_like("name", "%john%")
    .where_in("id", vec![1, 2, 3])
    .find()?;
```

### Ordering

```rust
Users::query()
    .order_by("created_at", OrderDirection::Desc)
    .find()?;
```

### Pagination

```rust
Users::query()
    .limit(10)
    .offset(20)
    .find()?;
```

### Aggregations

```rust
let count = Users::query().count()?;
let max_age = Users::query().max("age")?;
let avg_score = Users::query().avg("score")?;
```

## Model Operations

### Create

```rust
let mut user = User {
    name: "John".to_string(),
    email: "john@example.com".to_string(),
    ..Default::default()
};
user.create()?;
```

### Update

```rust
let mut user = Users::find_by_id(1)?;
user.name = "Jane".to_string();
user.update()?;
```

### Delete

```rust
let user = Users::find_by_id(1)?;
user.delete()?;
```

## Global Database Access

```rust
use rustf::db::DB;

// Execute raw SQL
DB::query("SELECT * FROM users WHERE id = ?", &[&1])?;

// Transaction
DB::transaction(|| {
    // ... operations ...
})?;
```

## Model Registration

Models are auto-discovered:

```rust
let app = RustF::new()
    .models(auto_models!());
```

## Examples

### Complete Model Usage

```rust
// Find user
let user = Users::find_by_id(1)?;

// Query with conditions
let active_users = Users::query()
    .where_eq("is_active", true)
    .order_by("created_at", OrderDirection::Desc)
    .limit(10)
    .find()?;

// Create new user
let mut new_user = User {
    name: "Alice".to_string(),
    email: "alice@example.com".to_string(),
    is_active: true,
    ..Default::default()
};
new_user.create()?;

// Update user
let mut user = Users::find_by_id(1)?;
user.name = "Bob".to_string();
user.update()?;

// Delete user
let user = Users::find_by_id(1)?;
user.delete()?;
```


