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