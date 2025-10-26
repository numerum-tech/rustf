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
