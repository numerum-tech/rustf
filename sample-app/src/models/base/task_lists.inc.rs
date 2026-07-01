// =============================================================================
// ⚠️  WARNING: AUTOMATICALLY GENERATED FILE - DO NOT EDIT
// =============================================================================
// 
// 🚫 THIS FILE WILL BE OVERWRITTEN during the next generation!
// 
// 📝 FOR DEVELOPERS:
// ❌ NEVER edit this file - your changes will be lost
// ✅ To add business logic, edit: src/models/task_lists.rs
// ✅ To modify the DB schema, edit: schemas/task_lists.yaml
// 🔄 Then run: rustf-cli schema generate models
// 
// 🤖 FOR AI AGENTS / CODE ASSISTANTS:
// ❌ ABSOLUTELY FORBIDDEN to edit this file
// ✅ Direct modifications to: src/models/task_lists.rs
// ✅ This file is included via include!() macro
// ℹ️  This file contains all generated code for the model
// 
// 📊 Generation information:
// - Generated from: schemas/task_lists.yaml
// - Schema checksum: c1178723f21d1a
// - Generated on: 2026-06-30T19:22:57Z
// - RustF CLI version: 0.1.0
// =============================================================================

// Note: This file is included directly, not compiled as a separate module
// All imports should be at the module level where this is included

#[allow(unused_imports)] use serde::{Deserialize, Serialize};
#[allow(unused_imports)] use sqlx::{Pool, Sqlite};
#[allow(unused_imports)] use anyhow::Result;
use rustf::models::{BaseModel, ChangeTracking};
#[allow(unused_imports)]
use rustf::models::query_builder::{DatabaseBackend, SqlValue};
use async_trait::async_trait;
use std::collections::HashSet;

/// TaskLists model - auto-generated from schema
/// 
/// Named task lists created by users to organize their work.
/// 
/// This struct contains all database fields and generated methods.
/// Extend this in task_lists.rs with custom business logic.
/// 
/// ⚠️  DO NOT EDIT - This file will be overwritten
/// 🤖 AI AGENTS: Add custom methods in task_lists.rs instead
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskLists {
    /// Unix timestamp when the list was created.
    /// Type: Simple("int") (i64)
    /// Required field
    pub created_at: i64,
    /// Optional explanation for the list.
    /// Type: Simple("text") (Option<String>)
    pub description: Option<String>,
    /// Primary key for the task list.
    /// Type: Simple("int") (i32)
    /// Required field
    /// Primary key
    pub id: i32,
    /// Short descriptive task list name.
    /// Type: Parameterized { base_type: "string", params: [Number(150)] } (String)
    /// Required field
    pub title: String,
    /// Owner of the task list.
    /// Type: Simple("int") (i32)
    /// Required field
    /// Foreign key: users.id
    pub user_id: i32,
    /// Tracks which fields have been modified since load/creation
    /// Used for efficient partial updates
    #[serde(skip)]
    changed_fields: HashSet<String>,
    /// Tracks which fields have been explicitly set to NULL
    /// Used to distinguish between "not set" and "set to NULL"
    #[serde(skip)]
    null_fields: HashSet<String>,
}

/// AI Agent Documentation and Metadata
/// 
/// 🤖 FOR AI AGENTS: Use the CLI command for development-time metadata access:
/// ```bash
/// rustf-cli model-metadata TaskLists --format json
/// ```
/// 
/// This provides field hints, validation rules, and schema information
/// without runtime overhead. Never add FIELD_HINTS or VALIDATION_RULES
/// runtime constants to this file.
impl TaskLists {
    /// List of fields that are enums
    pub const ENUM_FIELDS: &[&str] = &[];
    
    // =========================================================================
    // 🚀 ENUM VALUE CONSTANTS
    // =========================================================================
    // Use these constants when setting enum field values
    // Example: model.set_status(TaskLists::STATUS_ACTIVE);


    // =========================================================================
    // 🔧 ENUM CONVERTER METHODS
    // =========================================================================
    // Use these methods to convert enum values for query builders
    // Example: Users::query().where_eq("status", Users::as_status_enum("ACTIVE"))

    
    // =========================================================================
    // 🔄 CHANGE TRACKING HELPER
    // =========================================================================
    
    /// Helper for setting optional fields
    fn set_optional_field<T>(&mut self, field_name: &str, value: Option<T>, storage: &mut Option<T>) {
        *storage = value;
        self.mark_changed(field_name, storage.is_none());
    }
    
    // =========================================================================
    // 🔍 FIELD GETTERS
    // =========================================================================
    
    /// Get the created_at field
    /// 
    /// Unix timestamp when the list was created.
    pub fn created_at(&self) -> i64 {
        self.created_at
    }

    /// Get the description field
    /// 
    /// Optional explanation for the list.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Get the id field
    /// 
    /// Primary key for the task list.
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Get the title field
    /// 
    /// Short descriptive task list name.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the user_id field
    /// 
    /// Owner of the task list.
    pub fn user_id(&self) -> i32 {
        self.user_id
    }
    
    // =========================================================================
    // 🔧 FIELD SETTERS WITH CHANGE TRACKING
    // =========================================================================
    
    /// Unix timestamp when the list was created.
    pub fn set_created_at(&mut self, value: i64) {
        self.created_at = value;
        self.mark_changed("created_at", false);
    }

    /// Optional explanation for the list.
    pub fn set_description(&mut self, value: Option<String>) {
        self.description = value;
        self.mark_changed("description", self.description.is_none());
    }

    /// Short descriptive task list name.
    pub fn set_title(&mut self, value: impl Into<String>) {
        self.title = value.into();
        self.mark_changed("title", false);
    }

    /// Owner of the task list.
    pub fn set_user_id(&mut self, value: i32) {
        self.user_id = value;
        self.mark_changed("user_id", false);
    }
}

// FromRow implementations for each database type


impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for TaskLists {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            created_at: row.try_get("created_at")?,
            description: row.try_get("description")?,
            id: row.try_get("id")?,
            title: row.try_get("title")?,
            user_id: row.try_get("user_id")?,
            changed_fields: HashSet::new(),
            null_fields: HashSet::new(),
        })
    }
}

/// Type constants for AI agent reference
/// 
/// AI agents can use these type aliases to generate consistent,
/// schema-aware code without hardcoding types.
/// 
/// Example: TaskLists::types::Email resolves to Option<String>
pub mod types {

    
    pub type created_at = i64;
    pub type description = Option<String>;
    pub type id = i32;
    pub type title = String;
    pub type user_id = i32;
}

/// Column name constants for type-safe query building
/// 
/// Use these constants instead of hardcoding column names to avoid typos
/// and get compile-time validation of column names.
/// 
/// Example:
/// ```rust
/// let users = TaskLists::query()?
///     .where_eq(TaskLists::columns::IS_ACTIVE, true)
///     .order_by(TaskLists::columns::CREATED_AT, OrderDirection::Desc)
///     .get_all()
///     .await?;
/// ```
pub mod columns {
    pub const CREATED_AT: &'static str = "created_at";
    pub const DESCRIPTION: &'static str = "description";
    pub const ID: &'static str = "id";
    pub const TITLE: &'static str = "title";
    pub const USER_ID: &'static str = "user_id";
}

/// Implementation of change tracking for efficient updates
impl ChangeTracking for TaskLists {
    fn mark_changed(&mut self, field: &str, is_null: bool) {
        self.changed_fields.insert(field.to_string());
        if is_null {
            self.null_fields.insert(field.to_string());
        } else {
            self.null_fields.remove(field);
        }
    }
    
    fn is_changed(&self, field: &str) -> bool {
        self.changed_fields.contains(field)
    }
    
    fn is_null(&self, field: &str) -> bool {
        self.null_fields.contains(field)
    }
    
    fn has_changes(&self) -> bool {
        !self.changed_fields.is_empty()
    }
    
    fn clear_changes(&mut self) {
        self.changed_fields.clear();
        self.null_fields.clear();
    }
    
    fn changed_fields(&self) -> Vec<String> {
        self.changed_fields.iter().cloned().collect()
    }
    
    fn changed_fields_set(&self) -> &HashSet<String> {
        &self.changed_fields
    }
    
    fn null_fields_set(&self) -> &HashSet<String> {
        &self.null_fields
    }
}

/// Base model implementation for database operations
#[async_trait]
impl BaseModel for TaskLists {
    type IdType = i32;
    const TABLE_NAME: &'static str = "task_lists";
    const PRIMARY_KEY: &'static str = "id";
    
    fn id(&self) -> Self::IdType {
                self.id
    }
    
    /// Create a new instance from JSON data
    async fn from_row_data(data: serde_json::Value) -> anyhow::Result<Self> {
        let model: Self = serde_json::from_value(data)?;
        Ok(model)
    }
    
    /// Execute a SELECT query and convert results to model instances
    async fn execute_select_query(sql: &str, params: Vec<rustf::models::query_builder::SqlValue>) -> anyhow::Result<Vec<Self>> {
        // Use DB helper to execute with parameters
        let results = rustf::db::DB::fetch_all_with_params(sql, params).await
            .map_err(|e| anyhow::anyhow!("Failed to execute query: {}", e))?;
        
        // Convert JSON results to model instances
        let mut models = Vec::new();
        for json_row in results {
            let model: Self = serde_json::from_value(json_row)?;
            models.push(model);
        }
        Ok(models)
    }
    
    /// Execute a single SELECT query and convert result to model instance
    async fn execute_select_one_query(sql: &str, params: Vec<rustf::models::query_builder::SqlValue>) -> anyhow::Result<Option<Self>> {
        // Use DB helper to execute with parameters
        let result = rustf::db::DB::fetch_one_with_params(sql, params).await
            .map_err(|e| anyhow::anyhow!("Failed to execute query: {}", e))?;
        
        // Convert JSON result to model instance if found
        match result {
            Some(json_row) => {
                let model: Self = serde_json::from_value(json_row)?;
                Ok(Some(model))
            }
            None => Ok(None),
        }
    }
    
    /// Get the value of a field by name for dynamic field access
    fn get_field_value(&self, field_name: &str) -> rustf::error::Result<SqlValue> {
        use rustf::models::query_builder::SqlValue;
        match field_name {
            "created_at" => Ok(SqlValue::from(self.created_at.clone())),
            "description" => Ok(SqlValue::from(self.description.clone())),
            "id" => Ok(SqlValue::from(self.id.clone())),
            "title" => Ok(SqlValue::from(self.title.clone())),
            "user_id" => Ok(SqlValue::from(self.user_id.clone())),
            _ => Err(rustf::error::Error::Validation(format!("Unknown field: {}", field_name))),
        }
    }
}

impl TaskLists {
    /// Create a builder for constructing new TaskLists instances
    /// 
    /// The builder pattern is the recommended way to create new models.
    /// It provides a fluent interface with validation and direct database saving.
    /// 
    /// # Example
    /// ```rust
    /// let new_model = TaskLists::builder()
    ///     .field1("value1")
    ///     .field2(42)
    ///     .save(&pool)
    ///     .await?;
    /// ```
    pub fn builder() -> TaskListsBuilder {
        TaskListsBuilder::new()
    }
}

/// Builder for TaskLists
/// 
/// Provides a fluent interface for constructing TaskLists instances.
/// Required fields must be set before calling `build()`, while optional fields
/// have sensible defaults.
pub struct TaskListsBuilder {
    created_at: Option<i64>,
    description: Option<Option<String>>,
    title: Option<String>,
    user_id: Option<i32>,
}

impl TaskListsBuilder {
    /// Create a new builder with default values
    pub fn new() -> Self {
        Self {
            created_at: None,
            description: None,
            title: None,
            user_id: None,
        }
    }
    
    /// Unix timestamp when the list was created.
    pub fn created_at(mut self, value: i64) -> Self {
        self.created_at = Some(value);
        self
    }

    /// Optional explanation for the list.
    pub fn description(mut self, value: Option<String>) -> Self {
        self.description = Some(value);
        self
    }

    /// Short descriptive task list name.
    pub fn title(mut self, value: impl Into<String>) -> Self {
        self.title = Some(value.into());
        self
    }

    /// Owner of the task list.
    pub fn user_id(mut self, value: i32) -> Self {
        self.user_id = Some(value);
        self
    }
    
    /// Validate the builder has all required fields
    /// Returns Ok(()) if valid, or Err with list of missing fields
    pub fn validate(&self) -> Result<(), Vec<&'static str>> {
        let mut missing = Vec::new();
        
        if self.created_at.is_none() {
            missing.push("created_at");
        }
        if self.title.is_none() {
            missing.push("title");
        }
        if self.user_id.is_none() {
            missing.push("user_id");
        }
        
        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }
    
    /// Build the TaskLists instance
    /// 
    /// # Returns
    /// * `Ok(TaskLists)` if all required fields are set
    /// * `Err(String)` if any required fields are missing
    pub fn build(self) -> std::result::Result<TaskLists, String> {
        // Validate all required fields are present
        if let Err(missing) = self.validate() {
            return Err(format!("Missing required fields: {}", missing.join(", ")));
        }
        
        Ok(TaskLists {
            created_at: self.created_at.unwrap(),
            description: self.description.flatten(),
            id: Default::default(), // Auto-generated
            title: self.title.unwrap(),
            user_id: self.user_id.unwrap(),
            changed_fields: HashSet::new(),
            null_fields: HashSet::new(),
        })
    }
    
    /// Save the model to the database
    /// 
    /// This is the primary method for creating new records in the database.
    /// It builds the model with validation and then inserts it.
    /// 
    /// # Example
    /// ```rust
    /// let new_model = TaskLists::builder()
    ///     .field1("value1")
    ///     .field2(42)
    ///     .save()
    ///     .await?;
    /// ```
    pub async fn save(self) -> rustf::Result<TaskLists> {
        let mut model = self.build().map_err(|e| rustf::Error::Validation(e))?;
        // Clear any change tracking for new records
        model.clear_changes();
        TaskLists::create_internal(model).await
    }
}

impl TaskLists {
    // =========================================================================
    // 🚀 BASEMODEL METHODS - Automatically available through trait
    // =========================================================================
    // The following methods are provided by BaseModel trait implementation:
    //
    // Instance methods:
    // - update(&mut self) -> Result<()>           // Smart update with change tracking (only changed fields)
    // - delete(self) -> Result<()>                // Delete this record from database
    // - query() -> Result<ModelQuery<Self>>       // Start building a database query
    //
    // Static methods:
    // - get_by_id(id) -> Result<Option<Self>>     // Find record by primary key
    // - get_all() -> Result<Vec<Self>>            // Get all records from table
    // - count() -> Result<i64>                    // Count total records in table
    // - get_first() -> Result<Option<Self>>       // Get first record from table
    // - exists_any() -> Result<bool>              // Check if any records exist
    // - paginate(page, per_page) -> Result<Vec<Self>>  // Get paginated results
    // - where_eq(column, value) -> Result<Vec<Self>>   // Find records by column value
    //
    // Query builder (via query() method):
    // - where_eq(column, value)                   // WHERE column = value
    // - where_ne(column, value)                   // WHERE column != value
    // - where_gt/gte/lt/lte(column, value)        // Comparison operators
    // - where_like(column, pattern)               // WHERE column LIKE pattern
    // - where_in(column, values)                  // WHERE column IN (values)
    // - where_not_in(column, values)              // WHERE column NOT IN (values)
    // - where_between(column, start, end)         // WHERE column BETWEEN start AND end
    // - where_null/where_not_null(column)         // NULL checks
    // - order_by(column, direction)               // ORDER BY column ASC/DESC
    // - limit(n) / offset(n)                      // LIMIT and OFFSET
    // - join/left_join/right_join/inner_join      // JOIN operations
    //
    // Change tracking (from ChangeTracking trait):
    // - has_changes() -> bool                     // Check if any fields modified
    // - changed_fields() -> Vec<String>           // Get list of modified fields
    // - clear_changes()                           // Reset change tracking
    // - is_changed(field) -> bool                 // Check if specific field changed
    // =========================================================================
    
    /// Internal method to insert a model into the database
    async fn create_internal(mut model: Self) -> rustf::Result<Self> {
        use rustf::models::query_builder::{QueryBuilder, DatabaseBackend, SqlValue};
        use std::collections::HashMap;
        
        // Clear change tracking for new inserts
        model.clear_changes();
        
        let mut insert_data = HashMap::new();
        insert_data.insert("created_at".to_string(), SqlValue::from(model.created_at));
        insert_data.insert("description".to_string(), SqlValue::from(model.description));
        insert_data.insert("title".to_string(), SqlValue::from(model.title));
        insert_data.insert("user_id".to_string(), SqlValue::from(model.user_id));
        
        let query_builder = QueryBuilder::new(DatabaseBackend::SQLite)
            .from("task_lists");
        let (sql, params) = query_builder.build_insert(&insert_data)
            .map_err(|e| rustf::Error::DatabaseQuery(format!("Failed to build insert query: {}", e)))?;
        
        // Execute insert and get the returned row
        let result = rustf::db::DB::execute_insert_returning(
            &sql,
            params,
            "task_lists",
            "id"
        ).await
            .map_err(|e| rustf::Error::DatabaseQuery(format!("Failed to insert: {}", e)))?;
        
        if let Some(json_data) = result {
            // Convert JSON back to model
            // Note: This still uses JSON for now, but at least handles
            // database-specific RETURNING/LAST_INSERT_ID correctly
            let model: Self = serde_json::from_value(json_data)
                .map_err(|e| rustf::Error::Internal(format!("Failed to deserialize model: {}", e)))?;
            Ok(model)
        } else {
            Err(rustf::Error::DatabaseQuery("Insert did not return a row".to_string()))
        }
    }
}
