# Real-World Application Example

This guide demonstrates building a complete blog application with RustF, covering all major framework features.

## Application Overview

We'll build a blog application with:
- User authentication
- Blog post management
- Comments system
- Admin panel
- Search functionality

## Project Structure

```
blog-app/
├── src/
│   ├── main.rs
│   ├── controllers/
│   │   ├── home.rs
│   │   ├── auth.rs
│   │   ├── posts.rs
│   │   ├── comments.rs
│   │   └── admin/
│   │       └── dashboard.rs
│   ├── models/
│   │   ├── users.rs
│   │   ├── posts.rs
│   │   └── comments.rs
│   ├── modules/
│   │   ├── user_service.rs
│   │   └── post_service.rs
│   └── middleware/
│       └── auth.rs
├── views/
│   ├── layouts/
│   │   └── application.html
│   ├── home/
│   │   └── index.html
│   ├── posts/
│   │   ├── index.html
│   │   ├── show.html
│   │   └── create.html
│   └── auth/
│       ├── login.html
│       └── register.html
└── config.toml
```

## Main Application

### src/main.rs

```rust
use rustf::prelude::*;

#[tokio::main]
async fn main() -> rustf::Result<()> {
    env_logger::init();
    
    let app = RustF::new()
        .controllers(auto_controllers!())
        .models(auto_models!())
        .middleware_from(auto_middleware!());
    
    println!("🚀 Blog application running at http://127.0.0.1:8000");
    app.start().await
}
```

## Controllers

### Home Controller

```rust
// src/controllers/home.rs
use rustf::prelude::*;

pub fn install() -> Vec<Route> {
    routes![
        GET "/" => index,
        GET "/search" => search,
    ]
}

async fn index(ctx: &mut Context) -> Result<()> {
    // Get recent posts
    let recent_posts = get_recent_posts(5)?;
    
    ctx.repository_set("recent_posts", json!(recent_posts));
    ctx.view("/home/index", json!({
        "title": "Welcome to My Blog"
    }))
}

async fn search(ctx: &mut Context) -> Result<()> {
    let query = ctx.query("q").unwrap_or("");
    
    if query.is_empty() {
        return ctx.redirect("/");
    }
    
    let results = search_posts(query)?;
    
    ctx.view("/home/search", json!({
        "title": format!("Search: {}", query),
        "query": query,
        "results": results
    }))
}
```

### Posts Controller

```rust
// src/controllers/posts.rs
use rustf::prelude::*;

pub fn install() -> Vec<Route> {
    routes![
        GET  "/posts"          => index,
        GET  "/posts/{id}"     => show,
        GET  "/posts/create"   => create_form,
        POST "/posts"          => create,
        GET  "/posts/{id}/edit" => edit_form,
        PUT  "/posts/{id}"     => update,
        DELETE "/posts/{id}"   => delete,
    ]
}

async fn index(ctx: &mut Context) -> Result<()> {
    let page = ctx.int_query_or("page", 1);
    let posts = get_posts_paginated(page, 10)?;
    
    ctx.view("/posts/index", json!({
        "title": "All Posts",
        "posts": posts,
        "page": page
    }))
}

async fn show(ctx: &mut Context) -> Result<()> {
    let post_id = ctx.int_param("id")?;
    let post = get_post_by_id(post_id)?;
    
    if post.is_none() {
        return ctx.throw404(Some("Post not found"));
    }
    
    let comments = get_comments_for_post(post_id)?;
    
    ctx.view("/posts/show", json!({
        "title": post.title.clone(),
        "post": post,
        "comments": comments
    }))
}

async fn create(ctx: &mut Context) -> Result<()> {
    // Require authentication
    ctx.require_auth()?;
    
    let form: CreatePostForm = ctx.body_form_typed()?;
    
    // Validation
    validate_post_form(&form)?;
    
    // Create post
    let post = create_new_post(&form, get_current_user_id(ctx)?)?;
    
    ctx.flash_success("Post created successfully!")?;
    ctx.redirect(&format!("/posts/{}", post.id))
}
```

## Models

### Post Model

```rust
// src/models/posts.rs
use rustf::prelude::*;

pub struct Posts {
    base: PostsBase,
}

impl Posts {
    pub fn new() -> Self {
        Self {
            base: PostsBase::new(),
        }
    }
    
    pub fn find_recent(&self, limit: i32) -> Result<Vec<Post>> {
        self.base.query()
            .where_eq("published", true)
            .order_by("created_at", OrderDirection::Desc)
            .limit(limit)
            .find()
    }
    
    pub fn search(&self, query: &str) -> Result<Vec<Post>> {
        self.base.query()
            .where_like("title", &format!("%{}%", query))
            .or_where_like("content", &format!("%{}%", query))
            .where_eq("published", true)
            .find()
    }
}
```

## Modules

### Post Service

```rust
// src/modules/post_service.rs
use rustf::prelude::*;

pub struct PostService;

impl PostService {
    pub fn create_post(&self, data: &CreatePostData, author_id: i32) -> Result<Post> {
        // Business logic for creating posts
        // Validation, formatting, etc.
    }
    
    pub fn can_edit(&self, post: &Post, user_id: i32) -> bool {
        post.author_id == user_id || is_admin(user_id)
    }
}

impl_shared_service!(PostService);
```

## Middleware

### Auth Middleware

```rust
// src/middleware/auth.rs
use rustf::prelude::*;

pub struct AuthMiddleware;

impl InboundMiddleware for AuthMiddleware {
    fn handle(&self, ctx: &mut Context) -> MiddlewareResult {
        let path = ctx.path();
        
        // Protected paths
        if path.starts_with("/posts/create") || 
           path.starts_with("/posts/") && ctx.req.method() == "POST" {
            
            if !ctx.has_session() || 
               !ctx.session().map(|s| s.is_authenticated()).unwrap_or(false) {
                ctx.flash_error("Please login to continue")?;
                ctx.redirect("/auth/login");
                return MiddlewareResult::Stop;
            }
        }
        
        MiddlewareResult::Continue
    }
}

pub fn install(registry: &mut MiddlewareRegistry) {
    registry.register("auth", AuthMiddleware);
}
```

## Configuration

### config.toml

```toml
[server]
host = "127.0.0.1"
port = 8000

[database]
url = "sqlite:blog.db"

[session]
timeout = 3600
secure = false
http_only = true

[views]
directory = "views"
cache_enabled = false
default_layout = "layouts/application"
```

## Views

### Layout

```html
<!-- views/layouts/application.html -->
<!DOCTYPE html>
<html>
<head>
    <title>@{model.title} - My Blog</title>
    <link rel="stylesheet" href="/css/style.css">
</head>
<body>
    <nav>
        <a href="/">Home</a>
        <a href="/posts">Posts</a>
        @{if repository.user}
            <a href="/posts/create">New Post</a>
            <a href="/auth/logout">Logout</a>
        @{else}
            <a href="/auth/login">Login</a>
        @{fi}
    </nav>
    
    @{if flash.success_msg}
        <div class="alert success">@{flash.success_msg}</div>
    @{fi}
    
    @{if flash.error_msg}
        <div class="alert error">@{flash.error_msg}</div>
    @{fi}
    
    <main>
        {{@body}}
    </main>
</body>
</html>
```

## Testing

### Manual Testing

```bash
# Start server
cargo run

# Test endpoints
curl http://localhost:8000/
curl http://localhost:8000/posts
curl http://localhost:8000/posts/1
```

### Integration Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_create_post() {
        // Test post creation
    }
    
    #[tokio::test]
    async fn test_search_posts() {
        // Test search functionality
    }
}
```

## Deployment

See the [Deployment Guide](../deployment/production.md) for production deployment instructions.

## Next Steps

- Add email notifications
- Add RSS feed
- Add social sharing
- Add analytics
- Add caching layer
- Add CDN for static assets





