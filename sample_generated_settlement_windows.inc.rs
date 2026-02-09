// =============================================================================
// ⚠️  WARNING: AUTOMATICALLY GENERATED FILE - DO NOT EDIT
// =============================================================================
// 
// 🚫 THIS FILE WILL BE OVERWRITTEN during the next generation!
// 
// 📝 FOR DEVELOPERS:
// ❌ NEVER edit this file - your changes will be lost
// ✅ To add business logic, edit: src/models/settlement_windows.rs
// ✅ To modify the DB schema, edit: schemas/settlement_windows.yaml
// 🔄 Then run: rustf-cli schema generate models
// 
// 🤖 FOR AI AGENTS / CODE ASSISTANTS:
// ❌ ABSOLUTELY FORBIDDEN to edit this file
// ✅ Direct modifications to: src/models/settlement_windows.rs
// ✅ This file is included via include!() macro
// ℹ️  This file contains all generated code for the model
// 
// 📊 Generation information:
// - Generated from: schemas/settlement_windows.yaml
// - Schema checksum: 805fa0de2a9fd5d4
// - Generated on: 2026-01-16T11:27:47Z
// - RustF CLI version: 0.1.0
// =============================================================================

// Note: This file is included directly, not compiled as a separate module
// All imports should be at the module level where this is included

use serde::{Deserialize, Serialize};
use rustf::Result;
use chrono::{DateTime, Utc, NaiveTime};
use rust_decimal::Decimal;
use uuid::Uuid;
use rustf::models::{BaseModel, ChangeTracking};
use rustf::models::query_builder::{DatabaseBackend, SqlValue};
use async_trait::async_trait;
use std::collections::HashSet;

/// SettlementWindows model - auto-generated from schema
/// 
/// Database entity for settlement_windows management and data storage
/// Database: Defines when settlements should run for each payment scheme
/// 
/// This struct contains all database fields and generated methods.
/// Extend this in settlement_windows.rs with custom business logic.
/// 
/// ⚠️  DO NOT EDIT - This file will be overwritten
/// 🤖 AI AGENTS: Add custom methods in settlement_windows.rs instead
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementWindows {
    /// Database field of type numeric
    /// Type: Parameterized { base_type: "decimal", params: [Number(19), Number(4)] } (Option<Decimal>)
    pub approval_threshold: Option<Decimal>,
    /// Foreign key reference to approved_by table
    /// Type: Simple("uuid") (Option<Uuid>)
    /// Foreign key: system_users.id
    pub approved_by: Option<Uuid>,
    /// Database field of type character varying
    /// Type: Parameterized { base_type: "string", params: [Number(35)] } (Option<String>)
    pub clearing_system_id: Option<String>,
    /// Record creation timestamp - automatically set on insert
    /// Type: Simple("timestamp") (Option<DateTime<Utc>>)
    pub created_at: Option<DateTime<Utc>>,
    /// Detailed description text - supports rich formatting
    /// Type: Simple("text") (Option<String>)
    pub description: Option<String>,
    /// Database field of type time without time zone
    /// Type: Simple("time") (NaiveTime)
    /// Required field
    pub execution_time: NaiveTime,
    /// Database field of type uuid
    /// Type: Simple("uuid") (Uuid)
    /// Required field
    /// Primary key
    pub id: Uuid,
    /// Status flag - false indicates soft deletion or deactivation
    /// Type: Simple("boolean") (Option<bool>)
    pub is_active: Option<bool>,
    /// Database field of type character varying
    /// Type: Parameterized { base_type: "string", params: [Number(255)] } (String)
    /// Required field
    pub label: String,
    /// Foreign key reference to requested_by table
    /// Type: Simple("uuid") (Option<Uuid>)
    /// Foreign key: system_users.id
    pub requested_by: Option<Uuid>,
    /// Boolean flag for binary state tracking
    /// Type: Simple("boolean") (Option<bool>)
    pub requires_approval: Option<bool>,
    /// Database field of type ARRAY
    /// Type: Simple("array<uuid>") (Option<Vec<String>>)
    pub supported_message_type_ids: Option<Vec<String>>,
    /// Last modification timestamp - automatically updated on change
    /// Type: Simple("timestamp") (Option<DateTime<Utc>>)
    pub updated_at: Option<DateTime<Utc>>,
    /// Database field of type character varying
    /// Type: Parameterized { base_type: "string", params: [Number(20)] } (Option<String>)
    pub window_type: Option<String>,
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
/// rustf-cli model-metadata SettlementWindows --format json
/// ```
/// 
/// This provides field hints, validation rules, and schema information
/// without runtime overhead. Never add FIELD_HINTS or VALIDATION_RULES
/// runtime constants to this file.
impl SettlementWindows {
    /// List of fields that are enums
    pub const ENUM_FIELDS: &[&str] = &[];
    
    // =========================================================================
    // 🚀 ENUM VALUE CONSTANTS
    // =========================================================================
    // Use these constants when setting enum field values
    // Example: model.set_status(SettlementWindows::STATUS_ACTIVE);


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
    
    /// Get the approval_threshold field
    /// 
    /// Database field of type numeric
    pub fn approval_threshold(&self) -> &Option<Decimal> {
        &self.approval_threshold
    }

    /// Get the approved_by field
    /// 
    /// Foreign key reference to approved_by table
    pub fn approved_by(&self) -> &Option<Uuid> {
        &self.approved_by
    }

    /// Get the clearing_system_id field
    /// 
    /// Database field of type character varying
    pub fn clearing_system_id(&self) -> Option<&str> {
        self.clearing_system_id.as_deref()
    }

    /// Get the created_at field
    /// 
    /// Record creation timestamp - automatically set on insert
    pub fn created_at(&self) -> Option<DateTime<Utc>> {
        self.created_at
    }

    /// Get the description field
    /// 
    /// Detailed description text - supports rich formatting
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Get the execution_time field
    /// 
    /// Database field of type time without time zone
    pub fn execution_time(&self) -> NaiveTime {
        self.execution_time
    }

    /// Get the id field
    /// 
    /// Database field of type uuid
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the is_active field
    /// 
    /// Status flag - false indicates soft deletion or deactivation
    pub fn is_active(&self) -> Option<bool> {
        self.is_active
    }

    /// Get the label field
    /// 
    /// Database field of type character varying
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Get the requested_by field
    /// 
    /// Foreign key reference to requested_by table
    pub fn requested_by(&self) -> &Option<Uuid> {
        &self.requested_by
    }

    /// Get the requires_approval field
    /// 
    /// Boolean flag for binary state tracking
    pub fn requires_approval(&self) -> Option<bool> {
        self.requires_approval
    }

    /// Get the supported_message_type_ids field
    /// 
    /// Database field of type ARRAY
    pub fn supported_message_type_ids(&self) -> &Option<Vec<String>> {
        &self.supported_message_type_ids
    }

    /// Get the updated_at field
    /// 
    /// Last modification timestamp - automatically updated on change
    pub fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.updated_at
    }

    /// Get the window_type field
    /// 
    /// Database field of type character varying
    pub fn window_type(&self) -> Option<&str> {
        self.window_type.as_deref()
    }
    
    // =========================================================================
    // 🔧 FIELD SETTERS WITH CHANGE TRACKING
    // =========================================================================
    
    /// Database field of type numeric
    pub fn set_approval_threshold(&mut self, value: Option<Decimal>) {
        let is_null = value.is_none();
        self.approval_threshold = value;
        self.mark_changed("approval_threshold", is_null);
    }

    /// Foreign key reference to approved_by table
    pub fn set_approved_by(&mut self, value: Option<Uuid>) {
        let is_null = value.is_none();
        self.approved_by = value;
        self.mark_changed("approved_by", is_null);
    }

    /// Database field of type character varying
    pub fn set_clearing_system_id(&mut self, value: Option<impl Into<String>>) {
        self.clearing_system_id = value.map(|v| v.into());
        self.mark_changed("clearing_system_id", self.clearing_system_id.is_none());
    }

    /// Record creation timestamp - automatically set on insert
    pub fn set_created_at(&mut self, value: Option<DateTime<Utc>>) {
        let is_null = value.is_none();
        self.created_at = value;
        self.mark_changed("created_at", is_null);
    }

    /// Detailed description text - supports rich formatting
    pub fn set_description(&mut self, value: Option<impl Into<String>>) {
        self.description = value.map(|v| v.into());
        self.mark_changed("description", self.description.is_none());
    }

    /// Database field of type time without time zone
    pub fn set_execution_time(&mut self, value: NaiveTime) {
        self.execution_time = value;
        self.mark_changed("execution_time", false);
    }

    /// Status flag - false indicates soft deletion or deactivation
    pub fn set_is_active(&mut self, value: Option<bool>) {
        let is_null = value.is_none();
        self.is_active = value;
        self.mark_changed("is_active", is_null);
    }

    /// Database field of type character varying
    pub fn set_label(&mut self, value: impl Into<String>) {
        self.label = value.into();
        self.mark_changed("label", false);
    }

    /// Foreign key reference to requested_by table
    pub fn set_requested_by(&mut self, value: Option<Uuid>) {
        let is_null = value.is_none();
        self.requested_by = value;
        self.mark_changed("requested_by", is_null);
    }

    /// Boolean flag for binary state tracking
    pub fn set_requires_approval(&mut self, value: Option<bool>) {
        let is_null = value.is_none();
        self.requires_approval = value;
        self.mark_changed("requires_approval", is_null);
    }

    /// Database field of type ARRAY
    pub fn set_supported_message_type_ids(&mut self, value: Option<Vec<String>>) {
        let is_null = value.is_none();
        self.supported_message_type_ids = value;
        self.mark_changed("supported_message_type_ids", is_null);
    }

    /// Last modification timestamp - automatically updated on change
    pub fn set_updated_at(&mut self, value: Option<DateTime<Utc>>) {
        let is_null = value.is_none();
        self.updated_at = value;
        self.mark_changed("updated_at", is_null);
    }

    /// Database field of type character varying
    pub fn set_window_type(&mut self, value: Option<impl Into<String>>) {
        self.window_type = value.map(|v| v.into());
        self.mark_changed("window_type", self.window_type.is_none());
    }
}

// FromRow implementations for each database type
impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for SettlementWindows {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            approval_threshold: row.try_get("approval_threshold")?,
            approved_by: row.try_get("approved_by")?,
            clearing_system_id: row.try_get("clearing_system_id")?,
            created_at: row.try_get("created_at")?,
            description: row.try_get("description")?,
            execution_time: row.try_get("execution_time")?,
            id: row.try_get("id")?,
            is_active: row.try_get("is_active")?,
            label: row.try_get("label")?,
            requested_by: row.try_get("requested_by")?,
            requires_approval: row.try_get("requires_approval")?,
            supported_message_type_ids: row.try_get("supported_message_type_ids")?,
            updated_at: row.try_get("updated_at")?,
            window_type: row.try_get("window_type")?,
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
/// Example: SettlementWindows::types::Email resolves to Option<String>
pub mod types {
    use chrono::{DateTime, Utc, NaiveTime};
    use rust_decimal::Decimal;
    use uuid::Uuid;
    
    pub type ApprovalThreshold = Option<Decimal>;
    pub type ApprovedBy = Option<Uuid>;
    pub type ClearingSystemId = Option<String>;
    pub type CreatedAt = Option<DateTime<Utc>>;
    pub type Description = Option<String>;
    pub type ExecutionTime = NaiveTime;
    pub type Id = Uuid;
    pub type IsActive = Option<bool>;
    pub type Label = String;
    pub type RequestedBy = Option<Uuid>;
    pub type RequiresApproval = Option<bool>;
    pub type SupportedMessageTypeIds = Option<Vec<String>>;
    pub type UpdatedAt = Option<DateTime<Utc>>;
    pub type WindowType = Option<String>;
}

/// Column name constants for type-safe query building
/// 
/// Use these constants instead of hardcoding column names to avoid typos
/// and get compile-time validation of column names.
/// 
/// Example:
/// ```rust
/// let users = SettlementWindows::query()?
///     .where_eq(SettlementWindows::columns::IS_ACTIVE, true)
///     .order_by(SettlementWindows::columns::CREATED_AT, OrderDirection::Desc)
///     .get_all()
///     .await?;
/// ```
pub mod columns {
    pub const APPROVAL_THRESHOLD: &'static str = "approval_threshold";
    pub const APPROVED_BY: &'static str = "approved_by";
    pub const CLEARING_SYSTEM_ID: &'static str = "clearing_system_id";
    pub const CREATED_AT: &'static str = "created_at";
    pub const DESCRIPTION: &'static str = "description";
    pub const EXECUTION_TIME: &'static str = "execution_time";
    pub const ID: &'static str = "id";
    pub const IS_ACTIVE: &'static str = "is_active";
    pub const LABEL: &'static str = "label";
    pub const REQUESTED_BY: &'static str = "requested_by";
    pub const REQUIRES_APPROVAL: &'static str = "requires_approval";
    pub const SUPPORTED_MESSAGE_TYPE_IDS: &'static str = "supported_message_type_ids";
    pub const UPDATED_AT: &'static str = "updated_at";
    pub const WINDOW_TYPE: &'static str = "window_type";
}

/// Implementation of change tracking for efficient updates
impl ChangeTracking for SettlementWindows {
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
impl BaseModel for SettlementWindows {
    type IdType = Uuid;
    const TABLE_NAME: &'static str = "settlement_windows";
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
            "approval_threshold" => Ok(SqlValue::from(self.approval_threshold.clone())),
            "approved_by" => Ok(SqlValue::from(self.approved_by.clone())),
            "clearing_system_id" => Ok(SqlValue::from(self.clearing_system_id.clone())),
            "created_at" => Ok(SqlValue::from(self.created_at.clone())),
            "description" => Ok(SqlValue::from(self.description.clone())),
            "execution_time" => Ok(SqlValue::from(self.execution_time.clone())),
            "id" => Ok(SqlValue::from(self.id.clone())),
            "is_active" => Ok(SqlValue::from(self.is_active.clone())),
            "label" => Ok(SqlValue::from(self.label.clone())),
            "requested_by" => Ok(SqlValue::from(self.requested_by.clone())),
            "requires_approval" => Ok(SqlValue::from(self.requires_approval.clone())),
            "supported_message_type_ids" => Ok(SqlValue::from(self.supported_message_type_ids.clone())),
            "updated_at" => Ok(SqlValue::from(self.updated_at.clone())),
            "window_type" => Ok(SqlValue::from(self.window_type.clone())),
            _ => Err(rustf::error::Error::Validation(format!("Unknown field: {}", field_name))),
        }
    }
}

impl SettlementWindows {
    /// Create a builder for constructing new SettlementWindows instances
    /// 
    /// The builder pattern is the recommended way to create new models.
    /// It provides a fluent interface with validation and direct database saving.
    /// 
    /// # Example
    /// ```rust
    /// let new_model = SettlementWindows::builder()
    ///     .field1("value1")
    ///     .field2(42)
    ///     .save(&pool)
    ///     .await?;
    /// ```
    pub fn builder() -> SettlementWindowsBuilder {
        SettlementWindowsBuilder::new()
    }
}

/// Builder for SettlementWindows
/// 
/// Provides a fluent interface for constructing SettlementWindows instances.
/// Required fields must be set before calling `build()`, while optional fields
/// have sensible defaults.
pub struct SettlementWindowsBuilder {
    approval_threshold: Option<Option<Decimal>>,
    approved_by: Option<Option<Uuid>>,
    clearing_system_id: Option<Option<String>>,
    created_at: Option<Option<DateTime<Utc>>>,
    description: Option<Option<String>>,
    execution_time: Option<NaiveTime>,
    id: Option<Uuid>,
    is_active: Option<Option<bool>>,
    label: Option<String>,
    requested_by: Option<Option<Uuid>>,
    requires_approval: Option<Option<bool>>,
    supported_message_type_ids: Option<Option<Vec<String>>>,
    updated_at: Option<Option<DateTime<Utc>>>,
    window_type: Option<Option<String>>,
}

impl SettlementWindowsBuilder {
    /// Create a new builder with default values
    pub fn new() -> Self {
        Self {
            approval_threshold: None,
            approved_by: None,
            clearing_system_id: None,
            created_at: None,
            description: None,
            execution_time: None,
            id: None,
            is_active: None,
            label: None,
            requested_by: None,
            requires_approval: None,
            supported_message_type_ids: None,
            updated_at: None,
            window_type: None,
        }
    }
    
    /// Database field of type numeric
    pub fn approval_threshold(mut self, value: Option<Decimal>) -> Self {
        self.approval_threshold = Some(value);
        self
    }

    /// Foreign key reference to approved_by table
    pub fn approved_by(mut self, value: Option<Uuid>) -> Self {
        self.approved_by = Some(value);
        self
    }

    /// Database field of type character varying
    pub fn clearing_system_id(mut self, value: Option<impl Into<String>>) -> Self {
        self.clearing_system_id = Some(value.map(|v| v.into()));
        self
    }

    /// Record creation timestamp - automatically set on insert
    pub fn created_at(mut self, value: Option<DateTime<Utc>>) -> Self {
        self.created_at = Some(value);
        self
    }

    /// Detailed description text - supports rich formatting
    pub fn description(mut self, value: Option<impl Into<String>>) -> Self {
        self.description = Some(value.map(|v| v.into()));
        self
    }

    /// Database field of type time without time zone
    pub fn execution_time(mut self, value: NaiveTime) -> Self {
        self.execution_time = Some(value);
        self
    }

    /// Database field of type uuid
    pub fn id(mut self, value: Uuid) -> Self {
        self.id = Some(value);
        self
    }

    /// Status flag - false indicates soft deletion or deactivation
    pub fn is_active(mut self, value: Option<bool>) -> Self {
        self.is_active = Some(value);
        self
    }

    /// Database field of type character varying
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }

    /// Foreign key reference to requested_by table
    pub fn requested_by(mut self, value: Option<Uuid>) -> Self {
        self.requested_by = Some(value);
        self
    }

    /// Boolean flag for binary state tracking
    pub fn requires_approval(mut self, value: Option<bool>) -> Self {
        self.requires_approval = Some(value);
        self
    }

    /// Database field of type ARRAY
    pub fn supported_message_type_ids(mut self, value: Option<Vec<String>>) -> Self {
        self.supported_message_type_ids = Some(value);
        self
    }

    /// Last modification timestamp - automatically updated on change
    pub fn updated_at(mut self, value: Option<DateTime<Utc>>) -> Self {
        self.updated_at = Some(value);
        self
    }

    /// Database field of type character varying
    pub fn window_type(mut self, value: Option<impl Into<String>>) -> Self {
        self.window_type = Some(value.map(|v| v.into()));
        self
    }
    
    /// Validate the builder has all required fields
    /// Returns Ok(()) if valid, or Err with list of missing fields
    pub fn validate(&self) -> std::result::Result<(), Vec<&'static str>> {
        let mut missing = Vec::new();
        
        if self.execution_time.is_none() {
            missing.push("execution_time");
        }
        if self.label.is_none() {
            missing.push("label");
        }
        
        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }
    
    /// Build the SettlementWindows instance
    /// 
    /// # Returns
    /// * `Ok(SettlementWindows)` if all required fields are set
    /// * `Err(String)` if any required fields are missing
    pub fn build(self) -> std::result::Result<SettlementWindows, String> {
        // Validate all required fields are present
        if let Err(missing) = self.validate() {
            return Err(format!("Missing required fields: {}", missing.join(", ")));
        }
        
        Ok(SettlementWindows {
            approval_threshold: self.approval_threshold.flatten(),
            approved_by: self.approved_by.flatten(),
            clearing_system_id: self.clearing_system_id.flatten(),
            created_at: self.created_at.flatten(),
            description: self.description.flatten(),
            execution_time: self.execution_time.unwrap(),
            id: Uuid::new_v4(),
            is_active: self.is_active.flatten(),
            label: self.label.unwrap(),
            requested_by: self.requested_by.flatten(),
            requires_approval: self.requires_approval.flatten(),
            supported_message_type_ids: self.supported_message_type_ids.flatten(),
            updated_at: self.updated_at.flatten(),
            window_type: self.window_type.flatten(),
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
    /// let new_model = SettlementWindows::builder()
    ///     .field1("value1")
    ///     .field2(42)
    ///     .save()
    ///     .await?;
    /// ```
    pub async fn save(self) -> rustf::Result<SettlementWindows> {
        let mut model = self.build().map_err(|e| rustf::Error::Validation(e))?;
        // Clear any change tracking for new records
        model.clear_changes();
        SettlementWindows::create_internal(model).await
    }
}

impl SettlementWindows {
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
        insert_data.insert("approval_threshold".to_string(), SqlValue::from(model.approval_threshold));
        insert_data.insert("approved_by".to_string(), SqlValue::from(model.approved_by));
        insert_data.insert("clearing_system_id".to_string(), SqlValue::from(model.clearing_system_id));
        insert_data.insert("created_at".to_string(), SqlValue::from(model.created_at));
        insert_data.insert("description".to_string(), SqlValue::from(model.description));
        insert_data.insert("execution_time".to_string(), SqlValue::from(model.execution_time));
        insert_data.insert("id".to_string(), SqlValue::from(model.id));
        insert_data.insert("is_active".to_string(), SqlValue::from(model.is_active));
        insert_data.insert("label".to_string(), SqlValue::from(model.label));
        insert_data.insert("requested_by".to_string(), SqlValue::from(model.requested_by));
        insert_data.insert("requires_approval".to_string(), SqlValue::from(model.requires_approval));
        insert_data.insert("supported_message_type_ids".to_string(), SqlValue::from(model.supported_message_type_ids));
        insert_data.insert("updated_at".to_string(), SqlValue::from(model.updated_at));
        insert_data.insert("window_type".to_string(), SqlValue::from(model.window_type));
        
        let query_builder = QueryBuilder::new(DatabaseBackend::Postgres)
            .from("settlement_windows");
        let (sql, params) = query_builder.build_insert(&insert_data)
            .map_err(|e| rustf::Error::DatabaseQuery(format!("Failed to build insert query: {}", e)))?;
        
        // Execute insert and get the returned row
        let result = rustf::db::DB::execute_insert_returning(
            &sql,
            params,
            "settlement_windows",
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