use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize, Deserialize)]
pub struct Pagination {
    pub items: i64,          // Total number of items
    pub page: u32,           // Current page (1-based)
    pub count: u32,          // Total pages
    pub per_page: u32,       // Items per page
    pub url_pattern: String, // URL pattern with {0} placeholder
}

impl Pagination {
    pub fn new(total: i64, page: u32, per_page: u32, url_pattern: String) -> Self {
        let count = ((total as f64) / (per_page as f64)).ceil() as u32;
        Self {
            items: total,
            page,
            count,
            per_page,
            url_pattern,
        }
    }

    // Helper methods for template
    pub fn is_first(&self) -> bool {
        self.page == 1
    }
    pub fn is_last(&self) -> bool {
        self.page >= self.count
    }
    pub fn has_prev(&self) -> bool {
        self.page > 1
    }
    pub fn has_next(&self) -> bool {
        self.page < self.count
    }

    pub fn first_url(&self) -> String {
        self.url_pattern.replace("{0}", "1")
    }

    pub fn last_url(&self) -> String {
        self.url_pattern.replace("{0}", &self.count.to_string())
    }

    pub fn prev_url(&self) -> String {
        if self.page > 1 {
            self.url_pattern
                .replace("{0}", &(self.page - 1).to_string())
        } else {
            "#".to_string()
        }
    }

    pub fn next_url(&self) -> String {
        if self.page < self.count {
            self.url_pattern
                .replace("{0}", &(self.page + 1).to_string())
        } else {
            "#".to_string()
        }
    }

    pub fn range(&self, max_items: usize) -> Vec<Value> {
        let mut pages = Vec::new();
        let start = if self.page <= 3 { 1 } else { self.page - 2 };
        let end = (start + max_items as u32 - 1).min(self.count);

        for i in start..=end {
            pages.push(json!({
                "page": i,
                "url": self.url_pattern.replace("{0}", &i.to_string()),
                "selected": i == self.page
            }));
        }
        pages
    }

    pub fn to_json(&self) -> Value {
        json!({
            "items": self.items,
            "page": self.page,
            "count": self.count,
            "per_page": self.per_page,
            "isFirst": self.is_first(),
            "isLast": self.is_last(),
            "hasPrev": self.has_prev(),
            "hasNext": self.has_next(),
            "first": { "url": self.first_url() },
            "last": { "url": self.last_url() },
            "prev": { "url": self.prev_url() },
            "next": { "url": self.next_url() },
            "range": self.range(7)
        })
    }
}

/// Convert a `PagedResult` from database queries to a `Pagination` for template rendering
///
/// This function bridges the gap between database pagination results (`PagedResult`)
/// and template pagination helpers (`Pagination`). It separates database concerns
/// from template rendering concerns.
///
/// # Arguments
/// * `paged_result` - The paginated result from a database query
/// * `url_pattern` - URL pattern with `{0}` placeholder for page number
///   (e.g., `/users?page={0}` or `/posts/{0}`)
///
/// # Returns
/// A `Pagination` object ready for template rendering
///
/// # Example
///
/// ```rust
/// use rustf::prelude::*;
///
/// // Get paginated results from database
/// let result = Users::query()?
///     .where_eq("is_active", true)
///     .get_paginated(2, 20)
///     .await?;
///
/// // Convert to Pagination for templates (separation of concerns)
/// let pagination = pagination_from_paged_result(&result, "/users?page={0}");
///
/// // Use in view
/// ctx.view("users/list", json!({
///     "users": result.rows,
///     "pagination": pagination.to_json()
/// }))
/// ```
///
/// # Template Usage
///
/// The resulting `Pagination` can be used in templates exactly like
/// `U::paginate()` results:
///
/// ```html
/// @{if pagination.isPrev}
///     <a href="@{pagination.prev.url}">Previous</a>
/// @{fi}
///
/// @{foreach page in pagination.range}
///     @{if page.selected}
///         <span class="current">@{page.page}</span>
///     @{else}
///         <a href="@{page.url}">@{page.page}</a>
///     @{fi}
/// @{end}
///
/// @{if pagination.isNext}
///     <a href="@{pagination.next.url}">Next</a>
/// @{fi}
/// ```
pub fn pagination_from_paged_result<T>(paged_result: &crate::models::PagedResult<T>, url_pattern: impl Into<String>) -> Pagination {
    Pagination::new(
        paged_result.total_rows,
        paged_result.page,
        paged_result.per_page,
        url_pattern.into(),
    )
}
