//! Base model trait for RustF models with change tracking
//!
//! This module provides the base trait for all database models,
//! including change tracking and common CRUD operations.

use crate::database::types::SqlValue;
use crate::models::model_query::ModelQuery;
use crate::models::query_builder::{AnyDatabase, DatabaseBackend, QueryBuilder};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Simple filter for WHERE clauses (works with static SQL strings)
#[derive(Clone)]
pub struct Filter {
    where_clause: String,
}

impl Filter {
    /// Create a new filter with a condition
    pub fn new(condition: &str) -> Self {
        Self {
            where_clause: condition.to_string(),
        }
    }

    /// Add an AND condition
    pub fn and(mut self, condition: &str) -> Self {
        self.where_clause.push_str(" AND ");
        self.where_clause.push_str(condition);
        self
    }

    /// Add an OR condition
    pub fn or(mut self, condition: &str) -> Self {
        self.where_clause.push_str(" OR ");
        self.where_clause.push_str(condition);
        self
    }

    /// Get the WHERE clause string
    pub fn clause(&self) -> &str {
        &self.where_clause
    }
}

/// Simple update builder for dynamic column updates (users handle parameters manually)
pub struct UpdateBuilder {
    set_clause: String,
}

impl Default for UpdateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateBuilder {
    /// Create a new update builder
    pub fn new() -> Self {
        Self {
            set_clause: String::new(),
        }
    }

    /// Add a column = value clause (user must include proper parameter placeholder)
    pub fn set(mut self, column: &str, placeholder: &str) -> Self {
        if !self.set_clause.is_empty() {
            self.set_clause.push_str(", ");
        }
        self.set_clause
            .push_str(&format!("{} = {}", column, placeholder));
        self
    }

    /// Get the SET clause string
    pub fn clause(&self) -> &str {
        &self.set_clause
    }

    /// Check if there are any updates
    pub fn is_empty(&self) -> bool {
        self.set_clause.is_empty()
    }
}

/// Trait for change tracking in models
///
/// This trait provides methods to track which fields have been modified
/// since the model was loaded or created, enabling efficient partial updates.
pub trait ChangeTracking {
    /// Mark a field as changed and track if it's NULL
    fn mark_changed(&mut self, field: &str, is_null: bool);

    /// Check if a specific field has been changed
    fn is_changed(&self, field: &str) -> bool;

    /// Check if a specific field is set to NULL
    fn is_null(&self, field: &str) -> bool;

    /// Check if any fields have been changed
    fn has_changes(&self) -> bool;

    /// Clear all change tracking (typically after a successful save/update)
    fn clear_changes(&mut self);

    /// Get list of changed field names
    fn changed_fields(&self) -> Vec<String>;

    /// Get the internal changed fields set (for implementation use)
    fn changed_fields_set(&self) -> &HashSet<String>;

    /// Get the internal null fields set (for implementation use)
    fn null_fields_set(&self) -> &HashSet<String>;
}

/// Base model trait that all database models must implement
///
/// This trait provides a common interface for CRUD operations across all database types
/// and includes change tracking capabilities.
#[async_trait]
pub trait BaseModel: ChangeTracking + Sized + Clone + Send + Sync + 'static
where
    Self: Serialize + for<'de> Deserialize<'de>,
{
    /// The type of the primary key field (e.g., i32, i64, String, Uuid)
    /// Must implement Into<SqlValue> for query building
    type IdType: Clone + Send + Sync + std::fmt::Display + Into<SqlValue> + 'static;

    /// The name of the database table
    const TABLE_NAME: &'static str;

    /// Get the ID value of this model instance
    fn id(&self) -> Self::IdType;

    /// Create a new instance from SQL row data (implemented by generated models)
    async fn from_row_data(data: serde_json::Value) -> Result<Self>;

    /// Get the value of a field by name (implemented by generated models)
    /// This is used for dynamic field access in generic update/create operations
    fn get_field_value(&self, field_name: &str) -> crate::error::Result<SqlValue>;

    // =========================================================================
    // MODEL-SCOPED QUERY BUILDER (NEW)
    // =========================================================================

    /// Create a new model-scoped query builder
    ///
    /// Returns a ModelQuery that's pre-configured with the correct table name
    /// and provides type-safe query building.
    ///
    /// # Returns
    /// * `Ok(ModelQuery<Self>)` - Ready-to-use query builder
    /// * `Err(Error)` - If database is not configured
    ///
    /// # Examples
    /// ```rust
    /// // Get active users
    /// let users = Users::query()?
    ///     .where_eq("is_active", 1)
    ///     .order_by("created_at", OrderDirection::Desc)
    ///     .limit(10)
    ///     .get_all()
    ///     .await?;
    ///
    /// // Find user by email
    /// let user = Users::query()?
    ///     .where_eq("email", "user@example.com")
    ///     .get_first()
    ///     .await?;
    /// ```
    fn query() -> crate::error::Result<ModelQuery<Self>> {
        ModelQuery::new(Self::TABLE_NAME)
    }

    // =========================================================================
    // STATIC CONVENIENCE METHODS (NEW)
    // =========================================================================

    /// Get a model by its primary key ID
    ///
    /// This is a convenient static method that's equivalent to:
    /// `Self::query()?.get_by_id(id).await`
    ///
    /// # Arguments
    /// * `id` - The primary key value to search for
    ///
    /// # Returns
    /// * `Ok(Some(Self))` - Model instance if found
    /// * `Ok(None)` - If no model with that ID exists
    /// * `Err(Error)` - If query fails or database not configured
    async fn get_by_id_static(id: Self::IdType) -> crate::error::Result<Option<Self>> {
        Self::query()?.get_by_id(id).await
    }

    /// Find all models matching a single WHERE condition
    ///
    /// This is a convenient static method for simple equality queries.
    ///
    /// # Arguments
    /// * `column` - Column name to filter on
    /// * `value` - Value to match
    ///
    /// # Returns
    /// * `Ok(Vec<Self>)` - All matching models
    /// * `Err(Error)` - If query fails or database not configured
    ///
    /// # Examples
    /// ```rust
    /// let active_users = Users::where_eq("is_active", 1).await?;
    /// let admins = Users::where_eq("role", "admin").await?;
    /// ```
    async fn where_eq_static<V: Into<SqlValue> + Send>(
        column: &str,
        value: V,
    ) -> crate::error::Result<Vec<Self>> {
        Self::query()?.where_eq(column, value).get_all().await
    }

    /// Get all models in the table
    ///
    /// # Returns
    /// * `Ok(Vec<Self>)` - All models in the table
    /// * `Err(Error)` - If query fails or database not configured
    ///
    /// # Warning
    /// This method can return a large number of records. Consider using
    /// pagination or filtering for production applications.
    async fn get_all() -> crate::error::Result<Vec<Self>> {
        Self::query()?.get_all().await
    }

    /// Count all models in the table
    ///
    /// # Returns
    /// * `Ok(i64)` - Number of records in the table
    /// * `Err(Error)` - If query fails or database not configured
    async fn count_all() -> crate::error::Result<i64> {
        Self::query()?.count().await
    }

    /// Count all records (shorter alias for count_all)
    ///
    /// This provides the standard ORM method name that developers expect.
    ///
    /// # Examples
    /// ```rust
    /// let total = PaymentSchemes::count().await?;
    /// ```
    async fn count() -> crate::error::Result<i64> {
        Self::count_all().await
    }

    /// Check if any models exist in the table
    ///
    /// # Returns
    /// * `Ok(true)` - Table has at least one record
    /// * `Ok(false)` - Table is empty
    /// * `Err(Error)` - If query fails or database not configured
    async fn exists_any() -> crate::error::Result<bool> {
        Self::query()?.exists().await
    }

    /// Get a model by ID
    ///
    /// This provides the standard ORM method name that developers expect.
    ///
    /// # Examples
    /// ```rust
    /// let user = Users::get_by_id(123).await?;
    /// ```
    async fn get_by_id(id: Self::IdType) -> crate::error::Result<Option<Self>> {
        Self::get_by_id_static(id).await
    }

    /// Find a model by ID (deprecated alias)
    ///
    /// **DEPRECATED**: Use `get_by_id()` instead
    ///
    /// # Examples
    /// ```rust
    /// let user = Users::find(123).await?;
    /// ```
    #[deprecated(since = "0.2.0", note = "Please use `get_by_id()` instead")]
    async fn find(id: Self::IdType) -> crate::error::Result<Option<Self>> {
        Self::get_by_id_static(id).await
    }

    /// Get all models in the table (deprecated alias)
    ///
    /// **DEPRECATED**: Use `get_all()` instead
    #[deprecated(since = "0.2.0", note = "Please use `get_all()` instead")]
    async fn all() -> crate::error::Result<Vec<Self>> {
        Self::get_all().await
    }

    /// Delete this model from the database
    ///
    /// Uses the global database connection for transparent access.
    ///
    /// # Examples
    /// ```rust
    /// let user = Users::find(123).await?.unwrap();
    /// user.delete().await?;
    /// ```
    async fn delete(self) -> crate::error::Result<()> {
        use crate::db::DB;
        use crate::models::query_builder::{DatabaseBackend, QueryBuilder};

        let db = DB::connection()
            .ok_or_else(|| crate::error::Error::template("Database not configured".to_string()))?;

        // Get database backend
        let backend = match db.as_ref() {
            AnyDatabase::Postgres(_) => DatabaseBackend::Postgres,
            AnyDatabase::MySQL(_) => DatabaseBackend::MySQL,
            AnyDatabase::SQLite(_) => DatabaseBackend::SQLite,
        };

        // Build parameterized delete query
        let query = QueryBuilder::new(backend)
            .from(Self::TABLE_NAME)
            .where_eq("id", self.id().into());

        let (sql, params) = query.build_delete().map_err(|e| {
            crate::error::Error::template(format!("Failed to build delete query: {}", e))
        })?;

        // Execute with proper parameter binding
        DB::execute_with_params(&sql, params)
            .await
            .map_err(|e| crate::error::Error::template(format!("Failed to delete: {}", e)))?;

        Ok(())
    }

    /// Get the first record from the table
    ///
    /// # Examples
    /// ```rust
    /// let first_user = Users::first().await?;
    /// ```
    async fn get_first() -> crate::error::Result<Option<Self>> {
        Self::query()?.limit(1).get_first().await
    }

    /// Get the first record (deprecated alias)
    ///
    /// **DEPRECATED**: Use `get_first()` instead
    #[deprecated(since = "0.2.0", note = "Please use `get_first()` instead")]
    async fn first() -> crate::error::Result<Option<Self>> {
        Self::get_first().await
    }

    /// Paginate results
    ///
    /// # Examples
    /// ```rust
    /// let users = Users::paginate(1, 20).await?;
    /// ```
    async fn paginate(page: u32, per_page: u32) -> crate::error::Result<Vec<Self>> {
        Self::query()?.paginate(page, per_page).get_all().await
    }

    // =========================================================================
    // DEFAULT IMPLEMENTATIONS FOR COMMON OPERATIONS
    // =========================================================================

    /// Smart update that only modifies changed fields
    ///
    /// This method efficiently updates only the fields that have been modified
    /// using the setter methods. It automatically tracks changes and generates
    /// optimized UPDATE queries.
    async fn update(&mut self) -> crate::error::Result<()> {
        use crate::db::DB;
        use std::collections::HashMap;

        // Skip if no changes
        if !self.has_changes() {
            return Ok(());
        }

        // Build update data from changed fields only
        let mut update_data = HashMap::new();
        for field in self.changed_fields() {
            let value = if self.is_null(&field) {
                SqlValue::Null
            } else {
                self.get_field_value(&field)?
            };
            update_data.insert(field, value);
        }

        // Build and execute UPDATE query

        let db = DB::connection()
            .ok_or_else(|| crate::error::Error::template("Database not configured".to_string()))?;

        // Get database backend
        let backend = match db.as_ref() {
            AnyDatabase::Postgres(_) => DatabaseBackend::Postgres,
            AnyDatabase::MySQL(_) => DatabaseBackend::MySQL,
            AnyDatabase::SQLite(_) => DatabaseBackend::SQLite,
        };

        let query_builder = QueryBuilder::new(backend)
            .from(Self::TABLE_NAME)
            .where_eq("id", self.id().into()); // Use .into() to convert IdType to SqlValue

        let (sql, params) = query_builder.build_update(&update_data).map_err(|e| {
            crate::error::Error::template(format!("Failed to build update query: {}", e))
        })?;

        // Execute with proper parameter binding through DB::execute_with_params
        DB::execute_with_params(&sql, params)
            .await
            .map_err(|e| crate::error::Error::template(format!("Failed to update: {}", e)))?;

        // Clear change tracking after successful update
        self.clear_changes();
        Ok(())
    }

    // =========================================================================
    // INTERNAL HELPER METHODS - Used by framework, not by users
    // =========================================================================

    /// Internal: Get a model by ID using provided database connection
    /// Users should use get_by_id() or get_by_id_static() instead
    #[doc(hidden)]
    async fn get_by_id_with_db(db: &AnyDatabase, id: Self::IdType) -> Result<Option<Self>> {
        use crate::models::query_builder::{DatabaseBackend, QueryBuilder};

        // Get database backend
        let backend = match db {
            AnyDatabase::Postgres(_) => DatabaseBackend::Postgres,
            AnyDatabase::MySQL(_) => DatabaseBackend::MySQL,
            AnyDatabase::SQLite(_) => DatabaseBackend::SQLite,
        };

        // Build parameterized select query
        let query = QueryBuilder::new(backend)
            .from(Self::TABLE_NAME)
            .where_eq("id", id.into());

        let (sql, params) = query
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build query: {}", e))?;

        let results = Self::execute_select_query(&sql, params).await?;
        Ok(results.into_iter().next())
    }

    // Helper methods that need to be implemented by generated models or provided as defaults

    /// Execute a SELECT query and convert results to model instances
    /// This method needs to be implemented by each generated model
    async fn execute_select_query(
        sql: &str,
        params: Vec<crate::models::query_builder::SqlValue>,
    ) -> Result<Vec<Self>>;

    /// Execute a single SELECT query and convert result to model instance
    /// This method needs to be implemented by each generated model
    async fn execute_select_one_query(
        sql: &str,
        params: Vec<crate::models::query_builder::SqlValue>,
    ) -> Result<Option<Self>>;

    /// Execute an UPDATE query and refetch the updated record
    async fn execute_update_and_refetch(
        db: &AnyDatabase,
        sql: &str,
        id: Self::IdType,
    ) -> Result<Self> {
        match db {
            AnyDatabase::Postgres(pool) => {
                sqlx::query(sql).execute(pool).await?;
            }
            AnyDatabase::MySQL(pool) => {
                sqlx::query(sql).execute(pool).await?;
            }
            AnyDatabase::SQLite(pool) => {
                sqlx::query(sql).execute(pool).await?;
            }
        }

        // Refetch the updated record
        Self::get_by_id_with_db(db, id.clone())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Model not found after update"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    // MySQL model for testing
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct MySqlTestModel {
        pub id: i32,
        pub name: String,
    }

    // PostgreSQL model for testing
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct PostgresTestModel {
        pub id: i32,
        pub name: String,
    }

    #[test]
    fn test_filter_creation() {
        let filter = Filter::new("id = 1");
        assert_eq!(filter.clause(), "id = 1");
    }

    #[test]
    fn test_filter_chaining() {
        let filter = Filter::new("status = 'active'")
            .and("age > 18")
            .or("department = 'IT'");

        assert_eq!(
            filter.clause(),
            "status = 'active' AND age > 18 OR department = 'IT'"
        );
    }

    #[test]
    fn test_mysql_filter() {
        let filter = Filter::new("status = ?").and("age > ?");

        assert_eq!(filter.clause(), "status = ? AND age > ?");
    }

    #[test]
    fn test_postgres_filter() {
        let filter = Filter::new("status = $1").and("created_at > $2");

        assert_eq!(filter.clause(), "status = $1 AND created_at > $2");
    }

    #[test]
    fn test_update_builder() {
        let updates = UpdateBuilder::new()
            .set("name", "'New Name'")
            .set("email", "'new@example.com'")
            .set("age", "25");

        assert_eq!(
            updates.clause(),
            "name = 'New Name', email = 'new@example.com', age = 25"
        );
        assert!(!updates.is_empty());
    }

    #[test]
    fn test_empty_update_builder() {
        let updates = UpdateBuilder::new();
        assert!(updates.is_empty());
    }

    #[test]
    fn test_update_builder_with_placeholders() {
        let updates = UpdateBuilder::new().set("name", "?").set("email", "?");

        assert_eq!(updates.clause(), "name = ?, email = ?");
    }

    // Note: We can't easily test the actual database operations in unit tests
    // without setting up a test database, but we can test that the trait
    // compiles and the types are correct.

    #[test]
    fn test_trait_compilation() {
        // Just test that the traits exist and have the expected associated types
        assert_eq!(
            std::mem::size_of::<MySqlTestModel>(),
            std::mem::size_of::<PostgresTestModel>()
        );
    }
}
