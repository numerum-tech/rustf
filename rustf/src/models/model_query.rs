//! Model-scoped query builder for RustF framework
//!
//! This module provides a ModelQuery struct that enables Laravel-style
//! model queries like `UserModel::query().where_eq("email", email).first().await?`

use crate::database::types::SqlValue;
use crate::db::DB;
use crate::error::{Error, Result};
use crate::models::base_model::BaseModel;
use crate::models::filter::ModelFilter;
use crate::models::query_builder::{OrderDirection, QueryBuilder, QueryError};
use std::marker::PhantomData;

/// Model-scoped query builder that provides type-safe, chainable query operations
///
/// This struct wraps the generic QueryBuilder and provides model-specific
/// functionality with automatic table names and typed results.
pub struct ModelQuery<T> {
    query_builder: QueryBuilder,
    _phantom: PhantomData<T>,
}

impl<T: BaseModel> ModelQuery<T> {
    /// Create a new ModelQuery for the specified table
    ///
    /// This is typically called from the model's `query()` method rather than directly.
    pub fn new(table_name: &str) -> Result<Self> {
        let query_builder = DB::query()?.from(table_name);

        Ok(Self {
            query_builder,
            _phantom: PhantomData,
        })
    }

    // =========================================================================
    // TABLE ALIASING
    // =========================================================================

    /// Set an alias for the main table
    ///
    /// This is useful for:
    /// - Self-joins (joining the same table multiple times)
    /// - Simplifying long table names
    /// - Making queries more readable
    ///
    /// # Example
    /// ```rust
    /// // Simple aliasing
    /// let users = Users::query()?
    ///     .alias("u")
    ///     .select(&["u.id", "u.name"])
    ///     .where_eq("u.is_active", true)
    ///     .get()
    ///     .await?;
    ///
    /// // Self-join for hierarchical data
    /// let employees = Users::query()?
    ///     .alias("emp")
    ///     .select(&["emp.name as employee", "mgr.name as manager"])
    ///     .left_join("users AS mgr", "mgr.id = emp.manager_id")
    ///     .where_not_null("emp.manager_id")
    ///     .get_raw()
    ///     .await?;
    /// ```
    pub fn alias(mut self, alias: &str) -> Self {
        self.query_builder = self.query_builder.as_alias(alias);
        self
    }

    // =========================================================================
    // WHERE CONDITIONS
    // =========================================================================

    /// Add WHERE column = value condition
    pub fn where_eq<V: Into<SqlValue>>(mut self, column: &str, value: V) -> Self {
        self.query_builder = self.query_builder.where_eq(column, value);
        self
    }

    /// Add WHERE column != value condition
    pub fn where_ne<V: Into<SqlValue>>(mut self, column: &str, value: V) -> Self {
        self.query_builder = self.query_builder.where_ne(column, value);
        self
    }

    /// Add WHERE column > value condition
    pub fn where_gt<V: Into<SqlValue>>(mut self, column: &str, value: V) -> Self {
        self.query_builder = self.query_builder.where_gt(column, value);
        self
    }

    /// Add WHERE column < value condition
    pub fn where_lt<V: Into<SqlValue>>(mut self, column: &str, value: V) -> Self {
        self.query_builder = self.query_builder.where_lt(column, value);
        self
    }

    /// Add WHERE column >= value condition
    pub fn where_gte<V: Into<SqlValue>>(mut self, column: &str, value: V) -> Self {
        self.query_builder = self.query_builder.where_gte(column, value);
        self
    }

    /// Add WHERE column <= value condition
    pub fn where_lte<V: Into<SqlValue>>(mut self, column: &str, value: V) -> Self {
        self.query_builder = self.query_builder.where_lte(column, value);
        self
    }

    /// Add WHERE column LIKE value condition
    pub fn where_like<V: Into<SqlValue>>(mut self, column: &str, value: V) -> Self {
        self.query_builder = self.query_builder.where_like(column, value);
        self
    }

    /// Add WHERE column NOT LIKE value condition
    pub fn where_not_like(mut self, column: &str, pattern: &str) -> Self {
        self.query_builder = self.query_builder.where_not_like(column, pattern);
        self
    }

    /// Add WHERE column IN (values) condition
    pub fn where_in<V: Into<SqlValue>>(mut self, column: &str, values: Vec<V>) -> Self {
        self.query_builder = self.query_builder.where_in(column, values);
        self
    }

    /// Add WHERE column NOT IN (values) condition
    pub fn where_not_in<V: Into<SqlValue>>(mut self, column: &str, values: Vec<V>) -> Self {
        self.query_builder = self.query_builder.where_not_in(column, values);
        self
    }

    /// Add WHERE column BETWEEN start AND end condition
    pub fn where_between<V: Into<SqlValue>>(mut self, column: &str, start: V, end: V) -> Self {
        self.query_builder = self.query_builder.where_between(column, start, end);
        self
    }

    /// Add WHERE column IS NULL condition
    pub fn where_null(mut self, column: &str) -> Self {
        self.query_builder = self.query_builder.where_null(column);
        self
    }

    /// Add WHERE column IS NOT NULL condition
    pub fn where_not_null(mut self, column: &str) -> Self {
        self.query_builder = self.query_builder.where_not_null(column);
        self
    }

    /// Apply a reusable filter to this query
    ///
    /// This allows you to define common filters once and apply them to multiple queries.
    ///
    /// # Example
    /// ```rust
    /// let active_filter = ModelFilter::new()
    ///     .where_eq("is_active", true)
    ///     .where_not_null("verified_at");
    ///
    /// let count = Users::query()?
    ///     .apply_filter(&active_filter)
    ///     .count()
    ///     .await?;
    ///
    /// let users = Users::query()?
    ///     .apply_filter(&active_filter)
    ///     .limit(10)
    ///     .get()
    ///     .await?;
    /// ```
    pub fn apply_filter(mut self, filter: &ModelFilter) -> Self {
        // Apply each condition from the filter to the query builder
        self.query_builder
            .where_conditions
            .extend(filter.get_conditions().iter().cloned());
        self
    }

    // =========================================================================
    // OR WHERE CONDITIONS
    // =========================================================================

    /// Add OR WHERE column = value condition
    pub fn or_where_eq<V: Into<SqlValue>>(mut self, column: &str, value: V) -> Self {
        self.query_builder = self.query_builder.or_where_eq(column, value);
        self
    }

    /// Add OR WHERE column != value condition
    pub fn or_where_ne<V: Into<SqlValue>>(mut self, column: &str, value: V) -> Self {
        self.query_builder = self.query_builder.or_where_ne(column, value);
        self
    }

    /// Add OR WHERE column > value condition
    pub fn or_where_gt<V: Into<SqlValue>>(mut self, column: &str, value: V) -> Self {
        self.query_builder = self.query_builder.or_where_gt(column, value);
        self
    }

    /// Add OR WHERE column < value condition
    pub fn or_where_lt<V: Into<SqlValue>>(mut self, column: &str, value: V) -> Self {
        self.query_builder = self.query_builder.or_where_lt(column, value);
        self
    }

    /// Add OR WHERE column LIKE pattern condition
    pub fn or_where_like(mut self, column: &str, pattern: &str) -> Self {
        self.query_builder = self.query_builder.or_where_like(column, pattern);
        self
    }

    /// Add OR WHERE column IN (values) condition
    pub fn or_where_in<V: Into<SqlValue>>(mut self, column: &str, values: Vec<V>) -> Self {
        self.query_builder = self.query_builder.or_where_in(column, values);
        self
    }

    /// Add OR WHERE column IS NULL condition
    pub fn or_where_null(mut self, column: &str) -> Self {
        self.query_builder = self.query_builder.or_where_null(column);
        self
    }

    // =========================================================================
    // FIELD SELECTION
    // =========================================================================

    /// Select specific fields instead of SELECT *
    ///
    /// # Example
    /// ```rust
    /// // Select specific columns
    /// let users = Users::query()?
    ///     .select(&["id", "name", "email"])
    ///     .where_eq("is_active", true)
    ///     .get()
    ///     .await?;
    ///
    /// // Select with table prefixes (useful for JOINs)
    /// let results = Users::query()?
    ///     .select(&["users.id", "users.name", "posts.title"])
    ///     .join("posts", "posts.user_id = users.id")
    ///     .get()
    ///     .await?;
    /// ```
    pub fn select(mut self, columns: &[&str]) -> Self {
        let columns_vec: Vec<String> = columns.iter().map(|s| s.to_string()).collect();
        self.query_builder = self.query_builder.select(columns_vec);
        self
    }

    /// Select with raw SQL expressions
    ///
    /// Use this for complex selections including SQL functions and aggregations.
    ///
    /// # Example
    /// ```rust
    /// let results = Users::query()?
    ///     .select_raw(&[
    ///         "users.name",
    ///         "COUNT(posts.id) as post_count",
    ///         "MAX(posts.created_at) as latest_post"
    ///     ])
    ///     .left_join("posts", "posts.user_id = users.id")
    ///     .group_by(&["users.id", "users.name"])
    ///     .get_raw()
    ///     .await?;
    /// ```
    pub fn select_raw(mut self, expressions: &[&str]) -> Self {
        let expr_vec: Vec<String> = expressions.iter().map(|s| s.to_string()).collect();
        self.query_builder = self.query_builder.select(expr_vec);
        self
    }

    // =========================================================================
    // ORDERING AND LIMITS
    // =========================================================================

    /// Add ORDER BY column ASC/DESC
    pub fn order_by(mut self, column: &str, direction: OrderDirection) -> Self {
        self.query_builder = self.query_builder.order_by(column, direction);
        self
    }

    /// Set LIMIT
    pub fn limit(mut self, limit: i64) -> Self {
        self.query_builder = self.query_builder.limit(limit);
        self
    }

    /// Set OFFSET
    pub fn offset(mut self, offset: i64) -> Self {
        self.query_builder = self.query_builder.offset(offset);
        self
    }

    /// Paginate results (page starts at 1)
    pub fn paginate(mut self, page: u32, per_page: u32) -> Self {
        self.query_builder = self.query_builder.paginate(page, per_page);
        self
    }

    // =========================================================================
    // GROUPING
    // =========================================================================

    /// Add GROUP BY clause
    ///
    /// # Example
    /// ```rust
    /// let results = Users::query()?
    ///     .select_raw(&["department", "COUNT(*) as count"])
    ///     .group_by(&["department"])
    ///     .get_raw()
    ///     .await?;
    /// ```
    pub fn group_by(mut self, columns: &[&str]) -> Self {
        for column in columns {
            self.query_builder = self.query_builder.group_by(column.to_string());
        }
        self
    }

    // =========================================================================
    // JOINS (if needed)
    // =========================================================================

    /// Add INNER JOIN
    pub fn join(mut self, table: &str, on: &str) -> Self {
        self.query_builder = self.query_builder.join(table, on);
        self
    }

    /// Add LEFT JOIN
    pub fn left_join(mut self, table: &str, on: &str) -> Self {
        self.query_builder = self.query_builder.left_join(table, on);
        self
    }

    // =========================================================================
    // QUERY EXECUTION - Returns Model Instances
    // =========================================================================

    /// Execute query and return all matching records
    ///
    /// # Returns
    /// * `Ok(Vec<T>)` - All records matching the query
    /// * `Err(Error)` - If query execution fails
    ///
    /// # Examples
    /// ```rust
    /// let users = Users::query()
    ///     .where_eq("is_active", 1)
    ///     .order_by("created_at", OrderDirection::Desc)
    ///     .get_all()
    ///     .await?;
    /// ```
    pub async fn get_all(self) -> Result<Vec<T>> {
        let (sql, params) = self
            .query_builder
            .build()
            .map_err(|e| Error::template(format!("Query build failed: {}", e)))?;

        log::debug!("Executing query: {}", sql);
        log::debug!("With parameters: {:?}", params);

        T::execute_select_query(&sql, params)
            .await
            .map_err(|e| Error::template(format!("Query execution failed: {}", e)))
    }

    /// Execute query and return the first matching record
    ///
    /// Automatically adds LIMIT 1 to the query for efficiency.
    ///
    /// # Returns
    /// * `Ok(Some(T))` - First record if found
    /// * `Ok(None)` - If no records match
    /// * `Err(Error)` - If query execution fails
    pub async fn get_first(mut self) -> Result<Option<T>> {
        // Add LIMIT 1 for efficiency
        self.query_builder = self.query_builder.limit(1);

        let results = self.get_all().await?;
        Ok(results.into_iter().next())
    }

    /// Find a record by its primary key ID
    ///
    /// This is a convenience method that adds WHERE id = ? to the query.
    ///
    /// # Arguments
    /// * `id` - The primary key value to search for
    ///
    /// # Returns
    /// * `Ok(Some(T))` - Record if found
    /// * `Ok(None)` - If no record with that ID exists
    /// * `Err(Error)` - If query execution fails
    pub async fn get_by_id(mut self, id: T::IdType) -> Result<Option<T>> {
        self.query_builder = self.query_builder.where_eq("id", id.into());
        self.get_first().await
    }

    /// Get one record (alias for get_first)
    ///
    /// This is an alias that AI agents and developers often expect.
    ///
    /// # Returns
    /// * `Ok(Some(T))` - First record if found
    /// * `Ok(None)` - If no records match
    /// * `Err(Error)` - If query execution fails
    pub async fn get_one(self) -> Result<Option<T>> {
        self.get_first().await
    }

    // Deprecated aliases for backward compatibility

    /// Execute query and return all matching records
    ///
    /// **DEPRECATED**: Use `get_all()` instead
    #[deprecated(since = "0.2.0", note = "Please use `get_all()` instead")]
    pub async fn get(self) -> Result<Vec<T>> {
        self.get_all().await
    }

    /// Execute query and return the first matching record
    ///
    /// **DEPRECATED**: Use `get_first()` instead
    #[deprecated(since = "0.2.0", note = "Please use `get_first()` instead")]
    pub async fn first(self) -> Result<Option<T>> {
        self.get_first().await
    }

    /// Find a record by its primary key ID
    ///
    /// **DEPRECATED**: Use `get_by_id()` instead
    #[deprecated(since = "0.2.0", note = "Please use `get_by_id()` instead")]
    pub async fn find(self, id: T::IdType) -> Result<Option<T>> {
        self.get_by_id(id).await
    }

    fn extract_count_from_json(value: serde_json::Value) -> Result<i64> {
        fn to_i64(val: &serde_json::Value) -> Option<i64> {
            match val {
                serde_json::Value::Number(n) => n.as_i64().or_else(|| n.as_u64().map(|u| u as i64)),
                serde_json::Value::String(s) => s.parse::<i64>().ok(),
                _ => None,
            }
        }

        if let Some(i) = to_i64(&value) {
            return Ok(i);
        }

        if let serde_json::Value::Object(map) = value {
            for key in ["count", "COUNT(*)", "count(*)", "COUNT(1)"] {
                if let Some(val) = map.get(key).and_then(to_i64) {
                    return Ok(val);
                }
            }

            if map.len() == 1 {
                if let Some((_key, val)) = map.iter().next() {
                    if let Some(i) = to_i64(val) {
                        return Ok(i);
                    }
                }
            }
        }

        Err(Error::template(
            "Count query did not return a numeric result",
        ))
    }

    /// Count the number of records matching the query
    ///
    /// # Returns
    /// * `Ok(i64)` - Number of matching records
    /// * `Err(Error)` - If query execution fails
    pub async fn count(mut self) -> Result<i64> {
        // Replace SELECT with COUNT(*)
        self.query_builder = self.query_builder.count();

        let (sql, params) = self
            .query_builder
            .build()
            .map_err(|e| Error::template(format!("Count query build failed: {}", e)))?;

        log::debug!(
            "Executing count query: {} with {} parameters",
            sql,
            params.len()
        );

        let row = DB::fetch_one_with_params(&sql, params)
            .await
            .map_err(|e| Error::template(format!("Count query failed: {}", e)))?;

        match row {
            Some(value) => Self::extract_count_from_json(value),
            None => Ok(0),
        }
    }

    /// Check if any records exist matching the query
    ///
    /// This is more efficient than counting when you only need to know
    /// if records exist.
    ///
    /// # Returns
    /// * `Ok(true)` - At least one record exists
    /// * `Ok(false)` - No records match
    /// * `Err(Error)` - If query execution fails
    pub async fn exists(self) -> Result<bool> {
        let count = self.count().await?;
        Ok(count > 0)
    }

    /// Execute raw query and return results as JSON
    ///
    /// This is useful for queries with custom SELECT expressions, aggregations,
    /// or complex JOINs that don't map directly to the model structure.
    ///
    /// # Returns
    ///
    /// A vector of JSON objects representing the query results.
    ///
    /// # Example
    ///
    /// ```rust
    /// let results = Users::query()?
    ///     .select_raw(&[
    ///         "department",
    ///         "COUNT(*) as user_count",
    ///         "AVG(salary) as avg_salary"
    ///     ])
    ///     .group_by(&["department"])
    ///     .get_raw()
    ///     .await?;
    ///
    /// for row in results {
    ///     let dept = row["department"].as_str().unwrap();
    ///     let count = row["user_count"].as_i64().unwrap();
    ///     let avg_salary = row["avg_salary"].as_f64().unwrap();
    ///     println!("{}: {} users, avg salary: {}", dept, count, avg_salary);
    /// }
    /// ```
    pub async fn get_raw(self) -> Result<Vec<serde_json::Value>> {
        let (sql, params) = self
            .query_builder
            .build()
            .map_err(|e| Error::template(format!("Query build failed: {}", e)))?;

        println!("[DEBUG] Executing raw SQL: {}", sql);

        let rows = DB::fetch_all_with_params(&sql, params)
            .await
            .map_err(|e| Error::database_query(e.to_string()))?;

        Ok(rows)
    }

    /// Get the underlying QueryBuilder for advanced operations
    ///
    /// This provides access to the raw QueryBuilder if you need functionality
    /// not exposed by ModelQuery.
    pub fn query_builder(&self) -> &QueryBuilder {
        &self.query_builder
    }

    /// Get the raw SQL and parameters (for debugging)
    ///
    /// # Returns
    /// * `Ok((String, Vec<SqlValue>))` - SQL string and parameters
    /// * `Err(QueryError)` - If query building fails
    pub fn to_sql(&self) -> std::result::Result<(String, Vec<SqlValue>), QueryError> {
        self.query_builder.build()
    }
}
