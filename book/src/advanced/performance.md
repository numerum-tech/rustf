# Performance Optimization

This guide covers performance optimization techniques for RustF applications.

## Overview

RustF is designed for performance, but there are several strategies to optimize your application further.

## View Caching

### Enable View Caching

In production, enable view caching:

```toml
[views]
cache_enabled = true
```

This caches compiled templates to avoid recompilation on every request.

## Database Optimization

### Connection Pooling

Configure appropriate pool size:

```toml
[database]
max_connections = 50
```

### Query Optimization

- Use indexes on frequently queried columns
- Avoid N+1 queries
- Use pagination for large datasets
- Cache frequently accessed data

### Example: Optimized Query

```rust
// Bad: N+1 queries
for post in posts {
    let author = get_author(post.author_id)?; // Query per post
}

// Good: Single query with join
// Posts::query() returns Result, so use `?`. join() takes (table, on_expr).
// select() takes a slice of columns (use select_raw for raw expressions).
// Use .get().await? to fetch the list.
let posts_with_authors = Posts::query()?
    .join("users", "posts.author_id = users.id")
    .select_raw(&["posts.*", "users.name as author_name"])
    .get()
    .await?;
```

## Session Storage

### Use Redis for Sessions

For better performance and scalability:

```toml
[session.storage]
type = "redis"
url = "redis://localhost:6379"
```

Benefits:
- Faster than database storage
- Shared across multiple instances
- Automatic expiration

## Static File Serving

### Serve via Nginx/CDN

Don't serve static files through the application:

```nginx
# Nginx configuration
location /static/ {
    alias /path/to/public/;
    expires 30d;
    add_header Cache-Control "public, immutable";
}
```

## Response Compression

### Enable Gzip

Configure nginx for compression:

```nginx
gzip on;
gzip_types text/plain text/css application/json application/javascript;
gzip_min_length 1000;
```

## Caching Strategies

### Application-Level Caching

Use in-memory caching for frequently accessed data:

```rust
use std::sync::Arc;
use std::sync::RwLock;
use std::collections::HashMap;

static CACHE: LazyLock<Arc<RwLock<HashMap<String, Value>>>> = 
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

fn get_cached_data(key: &str) -> Option<Value> {
    CACHE.read().unwrap().get(key).cloned()
}

fn set_cached_data(key: String, value: Value) {
    CACHE.write().unwrap().insert(key, value);
}
```

### HTTP Caching

Set appropriate cache headers:

```rust
ctx.add_header("Cache-Control", "public, max-age=3600");
```

## Request Pooling (Intentionally Not Used)

RustF **does not** use object pooling for `Request` objects, and it is **not**
a recommended optimization. Benchmarks (`rustf/benches/pool.rs`) show pooling is
roughly **2x slower** than direct allocation here: the mutex-lock and reset
overhead outweigh any reuse benefit, and Rust's allocator already handles small
objects efficiently. Prefer direct allocation.

A `RequestPool` type still exists (`rustf::pool::global_request_pool()`), but its
`get()` is **synchronous** and returns a `PooledRequest` (no `.await`, no `?`):

```rust
use rustf::pool::global_request_pool;

// Note: kept for benchmarking/documentation; not used on the hot path.
let pooled_req = global_request_pool().get(); // sync, returns PooledRequest
```

## Async Operations

### Use Async for I/O

Always use async for database and network operations. Database access goes
through the async query builder or the parameterized helpers — there is no
`db.query("SELECT ...")` that takes a raw SQL string (`DB::query()` takes no
arguments and returns a `QueryBuilder`):

```rust
// Query builder (async)
let users = Users::query()?.get().await?;

// Or run parameterized raw SQL via the async helpers
use rustf::db::DB;
let rows = DB::fetch_all_with_params("SELECT * FROM users WHERE active = $1", vec![true.into()]).await?;
let affected = DB::execute_with_params("DELETE FROM sessions WHERE expired = $1", vec![true.into()]).await?;
```

## Memory Management

### Avoid Unnecessary Cloning

```rust
// Bad: Unnecessary clone
let data = expensive_data.clone();
process(data);

// Good: Use reference
process(&expensive_data);
```

### Use String Capacity Hints

```rust
// Pre-allocate string capacity
let mut output = String::with_capacity(estimated_size);
```

## Profiling

### Use Cargo Instruments

```bash
cargo install cargo-instruments
cargo instruments --template time
```

### Benchmark Critical Paths

```rust
#[cfg(test)]
mod benches {
    use super::*;
    use criterion::{black_box, criterion_group, criterion_main, Criterion};

    fn bench_query(c: &mut Criterion) {
        c.bench_function("query_users", |b| {
            b.iter(|| {
                black_box(Users::find_all().unwrap());
            });
        });
    }

    criterion_group!(benches, bench_query);
    criterion_main!(benches);
}
```

## Monitoring

### Track Performance Metrics

- Request latency
- Database query time
- Memory usage
- CPU usage
- Error rates

### Use APM Tools

Consider integrating:
- Prometheus for metrics
- Grafana for visualization
- Sentry for error tracking

## Best Practices

1. **Enable view caching** in production
2. **Use connection pooling** for databases
3. **Serve static files** via nginx/CDN
4. **Enable compression** (gzip)
5. **Use Redis** for sessions in multi-instance deployments
6. **Optimize database queries** (indexes, avoid N+1)
7. **Cache frequently accessed data**
8. **Profile and benchmark** critical paths
9. **Monitor performance** in production
10. **Use async** for all I/O operations

## Example: Optimized Handler

```rust
async fn optimized_handler(ctx: &mut Context) -> Result<()> {
    // Use cached data if available
    if let Some(cached) = get_cached_data("recent_posts") {
        return ctx.json(cached);
    }
    
    // Optimized query with join
    // Posts::query() returns Result (use `?`); join() takes (table, on_expr);
    // use .get().await? to fetch the list (find(id) is for a single row by id).
    let posts = Posts::query()?
        .join("users", "posts.author_id = users.id")
        .where_eq("published", true)
        .order_by("created_at", OrderDirection::Desc)
        .limit(10)
        .get()
        .await?;
    
    // Cache result
    set_cached_data("recent_posts".to_string(), json!(posts));
    
    ctx.json(json!(posts))
}
```











