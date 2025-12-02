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
pool_size = 20
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
let posts_with_authors = Posts::query()
    .join("users", "posts.author_id", "users.id")
    .select("posts.*, users.name as author_name")
    .find()?;
```

## Session Storage

### Use Redis for Sessions

For better performance and scalability:

```toml
[session]
storage = "redis"
redis_url = "redis://localhost:6379"
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

## Request Pooling

RustF includes request pooling for high-performance scenarios:

```rust
use rustf::pool::global_request_pool;

// Get pooled request
let pooled_req = global_request_pool().get().await?;

// Use pooled request
// ... process request ...

// Return to pool (automatic)
```

## Async Operations

### Use Async for I/O

Always use async for database and network operations:

```rust
// Good: Async database query
let users = db.query_async("SELECT * FROM users").await?;

// Bad: Blocking operation
let users = db.query("SELECT * FROM users")?; // Blocks thread
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
    let posts = Posts::query()
        .join("users", "posts.author_id", "users.id")
        .where_eq("published", true)
        .order_by("created_at", OrderDirection::Desc)
        .limit(10)
        .find()?;
    
    // Cache result
    set_cached_data("recent_posts".to_string(), json!(posts));
    
    ctx.json(json!(posts))
}
```


