//! Paginated result type for database queries
//!
//! This module provides the `PagedResult<T>` struct that combines
//! paginated data rows with pagination metadata.

use serde::{Deserialize, Serialize};

/// Paginated result containing data rows and pagination metadata
///
/// This struct is returned by `.get_paginated()` methods and provides
/// both the actual data for the current page and metadata about the
/// pagination state.
///
/// # Example
///
/// ```ignore
/// let result = Users::query()?
///     .where_eq("is_active", true)
///     .get_paginated(2, 20)
///     .await?;
///
/// // Access the data
/// for user in &result.rows {
///     println!("User: {}", user.name);
/// }
///
/// // Access pagination metadata
/// println!("Page {} of {}", result.page, result.total_pages);
/// println!("Total rows: {}", result.total_rows);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedResult<T> {
    /// The actual data rows for the current page
    pub rows: Vec<T>,

    /// Current page number (1-based)
    pub page: u32,

    /// Number of items per page
    pub per_page: u32,

    /// Total number of rows matching the query (across all pages)
    pub total_rows: i64,

    /// Total number of pages
    pub total_pages: u32,

    /// Whether there is a next page
    pub has_next: bool,

    /// Whether there is a previous page
    pub has_prev: bool,
}

impl<T> PagedResult<T> {
    /// Create a new `PagedResult` from query results
    ///
    /// # Arguments
    /// * `rows` - The data rows for the current page
    /// * `page` - Current page number (1-based)
    /// * `per_page` - Number of items per page
    /// * `total_rows` - Total number of rows matching the query
    ///
    /// # Returns
    /// A new `PagedResult` with calculated metadata
    pub fn new(rows: Vec<T>, page: u32, per_page: u32, total_rows: i64) -> Self {
        let total_pages = if total_rows == 0 {
            0
        } else {
            ((total_rows as f64) / (per_page as f64)).ceil() as u32
        };
        let has_next = page < total_pages;
        let has_prev = page > 1;

        Self {
            rows,
            page,
            per_page,
            total_rows,
            total_pages,
            has_next,
            has_prev,
        }
    }

    /// Check if this is the first page
    pub fn is_first(&self) -> bool {
        self.page == 1
    }

    /// Check if this is the last page
    pub fn is_last(&self) -> bool {
        self.page >= self.total_pages || self.total_pages == 0
    }

    /// Get the next page number (if available)
    pub fn next_page(&self) -> Option<u32> {
        if self.has_next {
            Some(self.page + 1)
        } else {
            None
        }
    }

    /// Get the previous page number (if available)
    pub fn prev_page(&self) -> Option<u32> {
        if self.has_prev {
            Some(self.page - 1)
        } else {
            None
        }
    }
}
