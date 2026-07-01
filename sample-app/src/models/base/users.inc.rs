// =============================================================================
// ⚠️  WARNING: AUTOMATICALLY GENERATED FILE - DO NOT EDIT
// =============================================================================
// 
// 🚫 THIS FILE WILL BE OVERWRITTEN during the next generation!
// 
// 📝 FOR DEVELOPERS:
// ❌ NEVER edit this file - your changes will be lost
// ✅ To add business logic, edit: src/models/users.rs
// ✅ To modify the DB schema, edit: schemas/users.yaml
// 🔄 Then run: rustf-cli schema generate models
// 
// 🤖 FOR AI AGENTS / CODE ASSISTANTS:
// ❌ ABSOLUTELY FORBIDDEN to edit this file
// ✅ Direct modifications to: src/models/users.rs
// ✅ This file is included via include!() macro
// ℹ️  This file contains all generated code for the model
// 
// 📊 Generation information:
// - Generated from: schemas/users.yaml
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

/// Users model - auto-generated from schema
/// 
/// Application users who register, log in, and own task lists.
/// 
/// This struct contains all database fields and generated methods.
/// Extend this in users.rs with custom business logic.
/// 
/// ⚠️  DO NOT EDIT - This file will be overwritten
/// 🤖 AI AGENTS: Add custom methods in users.rs instead
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Users {
    /// Unix timestamp when the user account was created.
    /// Type: Simple("int") (i64)
    /// Required field
    pub created_at: i64,
    /// Name shown across the UI.
    /// Type: Parameterized { base_type: "string", params: [Number(120)] } (String)
    /// Required field
    pub display_name: String,
    /// Unique login identifier for the user.
    /// Type: Parameterized { base_type: "string", params: [Number(150)] } (String)
    /// Required field
    /// Unique constraint
    pub email: String,
    /// Primary key for the user record.
    /// Type: Simple("int") (i32)
    /// Required field
    /// Primary key
    pub id: i32,
    /// Bcrypt hash of the user's password.
    /// Type: Parameterized { base_type: "string", params: [Number(255)] } (String)
    /// Required field
    pub password_hash: String,
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
/// rustf-cli model-metadata Users --format json
/// ```
/// 
/// This provides field hints, validation rules, and schema information
/// without runtime overhead. Never add FIELD_HINTS or VALIDATION_RULES
/// runtime constants to this file.
impl Users {
    /// List of fields that are enums
    pub const ENUM_FIELDS: &[&str] = &[];
    
    // =========================================================================
    // 🚀 ENUM VALUE CONSTANTS
    // =========================================================================
    // Use these constants when setting enum field values
    // Example: model.set_status(Users::STATUS_ACTIVE);


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
    /// Unix timestamp when the user account was created.
    pub fn created_at(&self) -> i64 {
        self.created_at
    }

    /// Get the display_name field
    /// 
    /// Name shown across the UI.
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Get the email field
    /// 
    /// Unique login identifier for the user.
    pub fn email(&self) -> &str {
        &self.email
    }

    /// Get the id field
    /// 
    /// Primary key for the user record.
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Get the password_hash field
    /// 
    /// Bcrypt hash of the user's password.
    pub fn password_hash(&self) -> &str {
        &self.password_hash
    }
    
    // =========================================================================
    // 🔧 FIELD SETTERS WITH CHANGE TRACKING
    // =========================================================================
    
    /// Unix timestamp when the user account was created.
    pub fn set_created_at(&mut self, value: i64) {
        self.created_at = value;
        self.mark_changed("created_at", false);
    }

    /// Name shown across the UI.
    pub fn set_display_name(&mut self, value: impl Into<String>) {
        self.display_name = value.into();
        self.mark_changed("display_name", false);
    }

    /// Unique login identifier for the user.
    pub fn set_email(&mut self, value: impl Into<String>) {
        self.email = value.into();
        self.mark_changed("email", false);
    }

    /// Bcrypt hash of the user's password.
    pub fn set_password_hash(&mut self, value: impl Into<String>) {
        self.password_hash = value.into();
        self.mark_changed("password_hash", false);
    }
}

// FromRow implementations for each database type


impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for Users {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            created_at: row.try_get("created_at")?,
            display_name: row.try_get("display_name")?,
            email: row.try_get("email")?,
            id: row.try_get("id")?,
            password_hash: row.try_get("password_hash")?,
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
/// Example: Users::types::Email resolves to Option<String>
pub mod types {

    
    pub type created_at = i64;
    pub type display_name = String;
    pub type email = String;
    pub type id = i32;
    pub type password_hash = String;
}

/// Column name constants for type-safe query building
/// 
/// Use these constants instead of hardcoding column names to avoid typos
/// and get compile-time validation of column names.
/// 
/// Example:
/// ```rust
/// let users = Users::query()?
///     .where_eq(Users::columns::IS_ACTIVE, true)
///     .order_by(Users::columns::CREATED_AT, OrderDirection::Desc)
///     .get_all()
///     .await?;
/// ```
pub mod columns {
    pub const CREATED_AT: &'static str = "created_at";
    pub const DISPLAY_NAME: &'static str = "display_name";
    pub const EMAIL: &'static str = "email";
    pub const ID: &'static str = "id";
    pub const PASSWORD_HASH: &'static str = "password_hash";
}

/// Implementation of change tracking for efficient updates
impl ChangeTracking for Users {
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
impl BaseModel for Users {
    type IdType = i32;
    const TABLE_NAME: &'static str = "users";
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
            "display_name" => Ok(SqlValue::from(self.display_name.clone())),
            "email" => Ok(SqlValue::from(self.email.clone())),
            "id" => Ok(SqlValue::from(self.id.clone())),
            "password_hash" => Ok(SqlValue::from(self.password_hash.clone())),
            _ => Err(rustf::error::Error::Validation(format!("Unknown field: {}", field_name))),
        }
    }
}

impl Users {
    /// Create a builder for constructing new Users instances
    /// 
    /// The builder pattern is the recommended way to create new models.
    /// It provides a fluent interface with validation and direct database saving.
    /// 
    /// # Example
    /// ```rust
    /// let new_model = Users::builder()
    ///     .field1("value1")
    ///     .field2(42)
    ///     .save(&pool)
    ///     .await?;
    /// ```
    pub fn builder() -> UsersBuilder {
        UsersBuilder::new()
    }
}

/// Builder for Users
/// 
/// Provides a fluent interface for constructing Users instances.
/// Required fields must be set before calling `build()`, while optional fields
/// have sensible defaults.
pub struct UsersBuilder {
    created_at: Option<i64>,
    display_name: Option<String>,
    email: Option<String>,
    password_hash: Option<String>,
}

impl UsersBuilder {
    /// Create a new builder with default values
    pub fn new() -> Self {
        Self {
            created_at: None,
            display_name: None,
            email: None,
            password_hash: None,
        }
    }
    
    /// Unix timestamp when the user account was created.
    pub fn created_at(mut self, value: i64) -> Self {
        self.created_at = Some(value);
        self
    }

    /// Name shown across the UI.
    pub fn display_name(mut self, value: impl Into<String>) -> Self {
        self.display_name = Some(value.into());
        self
    }

    /// Unique login identifier for the user.
    pub fn email(mut self, value: impl Into<String>) -> Self {
        self.email = Some(value.into());
        self
    }

    /// Bcrypt hash of the user's password.
    pub fn password_hash(mut self, value: impl Into<String>) -> Self {
        self.password_hash = Some(value.into());
        self
    }
    
    /// Validate the builder has all required fields
    /// Returns Ok(()) if valid, or Err with list of missing fields
    pub fn validate(&self) -> Result<(), Vec<&'static str>> {
        let mut missing = Vec::new();
        
        if self.created_at.is_none() {
            missing.push("created_at");
        }
        if self.display_name.is_none() {
            missing.push("display_name");
        }
        if self.email.is_none() {
            missing.push("email");
        }
        if self.password_hash.is_none() {
            missing.push("password_hash");
        }
        
        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }
    
    /// Build the Users instance
    /// 
    /// # Returns
    /// * `Ok(Users)` if all required fields are set
    /// * `Err(String)` if any required fields are missing
    pub fn build(self) -> std::result::Result<Users, String> {
        // Validate all required fields are present
        if let Err(missing) = self.validate() {
            return Err(format!("Missing required fields: {}", missing.join(", ")));
        }
        
        Ok(Users {
            created_at: self.created_at.unwrap(),
            display_name: self.display_name.unwrap(),
            email: self.email.unwrap(),
            id: Default::default(), // Auto-generated
            password_hash: self.password_hash.unwrap(),
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
    /// let new_model = Users::builder()
    ///     .field1("value1")
    ///     .field2(42)
    ///     .save()
    ///     .await?;
    /// ```
    pub async fn save(self) -> rustf::Result<Users> {
        let mut model = self.build().map_err(|e| rustf::Error::Validation(e))?;
        // Clear any change tracking for new records
        model.clear_changes();
        Users::create_internal(model).await
    }
}

impl Users {
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
        insert_data.insert("display_name".to_string(), SqlValue::from(model.display_name));
        insert_data.insert("email".to_string(), SqlValue::from(model.email));
        insert_data.insert("password_hash".to_string(), SqlValue::from(model.password_hash));
        
        let query_builder = QueryBuilder::new(DatabaseBackend::SQLite)
            .from("users");
        let (sql, params) = query_builder.build_insert(&insert_data)
            .map_err(|e| rustf::Error::DatabaseQuery(format!("Failed to build insert query: {}", e)))?;
        
        // Execute insert and get the returned row
        let result = rustf::db::DB::execute_insert_returning(
            &sql,
            params,
            "users",
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
