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
// - Schema checksum: e633ba05badee0d7
// - Generated on: 2025-12-02T20:18:51Z
// - RustF CLI version: 0.1.0
// =============================================================================

// Note: This file is included directly, not compiled as a separate module
// All imports should be at the module level where this is included

use serde::{Deserialize, Serialize};
use sqlx::{Pool, MySql};
use anyhow::Result;
use chrono::{DateTime, Utc, NaiveDate};
use serde_json;
use rustf::models::{BaseModel, ChangeTracking};
use rustf::models::query_builder::{DatabaseBackend, SqlValue};
use async_trait::async_trait;
use std::collections::HashSet;

/// Users model - auto-generated from schema
/// 
/// User authentication and profile management. Handle passwords securely with bcrypt.
/// 
/// This struct contains all database fields and generated methods.
/// Extend this in users.rs with custom business logic.
/// 
/// ⚠️  DO NOT EDIT - This file will be overwritten
/// 🤖 AI AGENTS: Add custom methods in users.rs instead
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Users {
    /// Integer value for counting or quantity tracking
    /// Type: Simple("timestamp") (Option<DateTime<Utc>>)
    pub account_locked_until: Option<DateTime<Utc>>,
    /// Large text field for extended content
    /// Type: Simple("text") (Option<String>)
    pub address: Option<String>,
    /// Unique identifier code - ensure uniqueness constraints
    /// Type: Simple("json") (Option<serde_json::Value>)
    pub backup_codes: Option<serde_json::Value>,
    /// Unique identifier code - ensure uniqueness constraints
    /// Type: Simple("json") (Option<serde_json::Value>)
    pub backup_codes_used: Option<serde_json::Value>,
    /// Database field of type date
    /// Type: Simple("date") (NaiveDate)
    /// Required field
    pub birthdate: NaiveDate,
    /// Record creation timestamp - automatically set on insert
    /// Type: Simple("timestamp") (Option<DateTime<Utc>>)
    pub created_at: Option<DateTime<Utc>>,
    /// Valid email format required. Used for authentication and communication.
    /// Type: Parameterized { base_type: "string", params: [Number(150)] } (String)
    /// Required field
    /// Unique constraint
    pub email: String,
    /// Valid email format required. Used for authentication and communication.
    /// Type: Simple("tinyint") (i8)
    /// Required field
    pub email_verified: i8,
    /// Integer value for counting or identification
    /// Type: Simple("int") (Option<i32>)
    pub failed_login_attempts: Option<i32>,
    /// Display name for user interface and identification
    /// Type: Parameterized { base_type: "string", params: [Number(100)] } (String)
    /// Required field
    pub first_name: String,
    /// Enumerated value - validate against allowed options
    /// Type: Enum { type_name: "enum", values: ["F", "M", "X"], transitions: None } (String)
    /// Required field
    pub gender: String,
    /// Integer value for counting or identification
    /// Type: Simple("mediumint") (i32)
    /// Required field
    /// Primary key
    pub id: i32,
    /// Status flag - false indicates soft deletion or deactivation
    /// Type: Simple("tinyint") (Option<i8>)
    pub is_active: Option<i8>,
    /// Display name for user interface and identification
    /// Type: Parameterized { base_type: "string", params: [Number(100)] } (Option<String>)
    pub job_title: Option<String>,
    /// String field with length constraints
    /// Type: Parameterized { base_type: "string", params: [Number(10)] } (Option<String>)
    pub language_preference: Option<String>,
    /// Timestamp field for temporal data tracking
    /// Type: Simple("timestamp") (Option<DateTime<Utc>>)
    pub last_failed_login_at: Option<DateTime<Utc>>,
    /// Display name for user interface and identification
    /// Type: Parameterized { base_type: "string", params: [Number(100)] } (String)
    /// Required field
    pub last_name: String,
    /// Timestamp field for temporal data tracking
    /// Type: Simple("timestamp") (Option<DateTime<Utc>>)
    pub last_success_login_at: Option<DateTime<Utc>>,
    /// Foreign key reference to manager table
    /// Type: Simple("mediumint") (Option<i32>)
    /// Foreign key: users.id
    pub manager_id: Option<i32>,
    /// Phone number with international format support
    /// Type: Parameterized { base_type: "string", params: [Number(20)] } (Option<String>)
    /// Unique constraint
    pub mobile_number: Option<String>,
    /// Enumerated value - validate against allowed options
    /// Type: Enum { type_name: "enum", values: ["Email", "InApp", "SMS", "Both"], transitions: None } (Option<String>)
    pub notification_preference: Option<String>,
    /// Always store as bcrypt hash. Never store plain passwords!
    /// Type: Parameterized { base_type: "string", params: [Number(255)] } (String)
    /// Required field
    pub password_hash: String,
    /// Phone number with international format support
    /// Type: Parameterized { base_type: "string", params: [Number(20)] } (Option<String>)
    pub phone_number: Option<String>,
    /// Phone number with international format support
    /// Type: Simple("tinyint") (i8)
    /// Required field
    pub phone_verified: i8,
    /// Last modification timestamp - automatically updated on change
    /// Type: Simple("timestamp") (Option<DateTime<Utc>>)
    pub profile_photo_updated_at: Option<DateTime<Utc>>,
    /// URL field - validate format and accessibility
    /// Type: Parameterized { base_type: "string", params: [Number(255)] } (Option<String>)
    pub profile_photo_url: Option<String>,
    /// Integer value for counting or identification
    /// Type: Simple("tinyint") (Option<i8>)
    pub totp_enabled: Option<i8>,
    /// Sensitive token - store securely and never log
    /// Type: Parameterized { base_type: "string", params: [Number(255)] } (Option<String>)
    pub totp_secret: Option<String>,
    /// Last modification timestamp - automatically updated on change
    /// Type: Simple("timestamp") (Option<DateTime<Utc>>)
    pub updated_at: Option<DateTime<Utc>>,
    /// Display name for user interface and identification
    /// Type: Parameterized { base_type: "string", params: [Number(150)] } (String)
    /// Required field
    /// Unique constraint
    pub username: String,
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
    pub const ENUM_FIELDS: &[&str] = &["gender", "notification_preference"];
    
    // =========================================================================
    // 🚀 ENUM VALUE CONSTANTS
    // =========================================================================
    // Use these constants when setting enum field values
    // Example: model.set_status(Users::STATUS_ACTIVE);
    /// F value for gender field
    pub const GENDER_F: &'static str = "F";
    /// M value for gender field
    pub const GENDER_M: &'static str = "M";
    /// X value for gender field
    pub const GENDER_X: &'static str = "X";
    /// Email value for notification_preference field
    pub const NOTIFICATION_PREFERENCE_Email: &'static str = "Email";
    /// InApp value for notification_preference field
    pub const NOTIFICATION_PREFERENCE_InApp: &'static str = "InApp";
    /// SMS value for notification_preference field
    pub const NOTIFICATION_PREFERENCE_SMS: &'static str = "SMS";
    /// Both value for notification_preference field
    pub const NOTIFICATION_PREFERENCE_Both: &'static str = "Both";

    // =========================================================================
    // 🔧 ENUM CONVERTER METHODS
    // =========================================================================
    // Use these methods to convert enum values for query builders
    // Example: Users::query().where_eq("status", Users::as_status_enum("ACTIVE"))
    /// Convert a value to MySQL enum format for gender field
    /// 
    /// # Example
    /// ```
    /// let value = Users::as_gender_enum("ACTIVE");
    /// // Returns: "ACTIVE" (pass-through for MySQL)
    /// ```
    pub fn as_gender_enum(value: &str) -> String {
        value.to_string()
    }

    /// Convert a value to MySQL enum format for notification_preference field
    /// 
    /// # Example
    /// ```
    /// let value = Users::as_notification_preference_enum("ACTIVE");
    /// // Returns: "ACTIVE" (pass-through for MySQL)
    /// ```
    pub fn as_notification_preference_enum(value: &str) -> String {
        value.to_string()
    }
    
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
    
    /// Get the account_locked_until field
    /// 
    /// Integer value for counting or quantity tracking
    pub fn account_locked_until(&self) -> Option<DateTime<Utc>> {
        self.account_locked_until
    }

    /// Get the address field
    /// 
    /// Large text field for extended content
    pub fn address(&self) -> Option<&str> {
        self.address.as_deref()
    }

    /// Get the backup_codes field
    /// 
    /// Unique identifier code - ensure uniqueness constraints
    pub fn backup_codes(&self) -> &Option<serde_json::Value> {
        &self.backup_codes
    }

    /// Get the backup_codes_used field
    /// 
    /// Unique identifier code - ensure uniqueness constraints
    pub fn backup_codes_used(&self) -> &Option<serde_json::Value> {
        &self.backup_codes_used
    }

    /// Get the birthdate field
    /// 
    /// Database field of type date
    pub fn birthdate(&self) -> NaiveDate {
        self.birthdate
    }

    /// Get the created_at field
    /// 
    /// Record creation timestamp - automatically set on insert
    pub fn created_at(&self) -> Option<DateTime<Utc>> {
        self.created_at
    }

    /// Get the email field
    /// 
    /// Valid email format required. Used for authentication and communication.
    pub fn email(&self) -> &str {
        &self.email
    }

    /// Get the email_verified field
    /// 
    /// Valid email format required. Used for authentication and communication.
    pub fn email_verified(&self) -> i8 {
        self.email_verified
    }

    /// Get the failed_login_attempts field
    /// 
    /// Integer value for counting or identification
    pub fn failed_login_attempts(&self) -> Option<i32> {
        self.failed_login_attempts
    }

    /// Get the first_name field
    /// 
    /// Display name for user interface and identification
    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    /// Get the gender field
    /// 
    /// Enumerated value - validate against allowed options
    pub fn gender(&self) -> &str {
        &self.gender
    }

    /// Get the id field
    /// 
    /// Integer value for counting or identification
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Get the is_active field
    /// 
    /// Status flag - false indicates soft deletion or deactivation
    pub fn is_active(&self) -> Option<i8> {
        self.is_active
    }

    /// Get the job_title field
    /// 
    /// Display name for user interface and identification
    pub fn job_title(&self) -> Option<&str> {
        self.job_title.as_deref()
    }

    /// Get the language_preference field
    /// 
    /// String field with length constraints
    pub fn language_preference(&self) -> Option<&str> {
        self.language_preference.as_deref()
    }

    /// Get the last_failed_login_at field
    /// 
    /// Timestamp field for temporal data tracking
    pub fn last_failed_login_at(&self) -> Option<DateTime<Utc>> {
        self.last_failed_login_at
    }

    /// Get the last_name field
    /// 
    /// Display name for user interface and identification
    pub fn last_name(&self) -> &str {
        &self.last_name
    }

    /// Get the last_success_login_at field
    /// 
    /// Timestamp field for temporal data tracking
    pub fn last_success_login_at(&self) -> Option<DateTime<Utc>> {
        self.last_success_login_at
    }

    /// Get the manager_id field
    /// 
    /// Foreign key reference to manager table
    pub fn manager_id(&self) -> Option<i32> {
        self.manager_id
    }

    /// Get the mobile_number field
    /// 
    /// Phone number with international format support
    pub fn mobile_number(&self) -> Option<&str> {
        self.mobile_number.as_deref()
    }

    /// Get the notification_preference field
    /// 
    /// Enumerated value - validate against allowed options
    pub fn notification_preference(&self) -> Option<&str> {
        self.notification_preference.as_deref()
    }

    /// Get the password_hash field
    /// 
    /// Always store as bcrypt hash. Never store plain passwords!
    pub fn password_hash(&self) -> &str {
        &self.password_hash
    }

    /// Get the phone_number field
    /// 
    /// Phone number with international format support
    pub fn phone_number(&self) -> Option<&str> {
        self.phone_number.as_deref()
    }

    /// Get the phone_verified field
    /// 
    /// Phone number with international format support
    pub fn phone_verified(&self) -> i8 {
        self.phone_verified
    }

    /// Get the profile_photo_updated_at field
    /// 
    /// Last modification timestamp - automatically updated on change
    pub fn profile_photo_updated_at(&self) -> Option<DateTime<Utc>> {
        self.profile_photo_updated_at
    }

    /// Get the profile_photo_url field
    /// 
    /// URL field - validate format and accessibility
    pub fn profile_photo_url(&self) -> Option<&str> {
        self.profile_photo_url.as_deref()
    }

    /// Get the totp_enabled field
    /// 
    /// Integer value for counting or identification
    pub fn totp_enabled(&self) -> Option<i8> {
        self.totp_enabled
    }

    /// Get the totp_secret field
    /// 
    /// Sensitive token - store securely and never log
    pub fn totp_secret(&self) -> Option<&str> {
        self.totp_secret.as_deref()
    }

    /// Get the updated_at field
    /// 
    /// Last modification timestamp - automatically updated on change
    pub fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.updated_at
    }

    /// Get the username field
    /// 
    /// Display name for user interface and identification
    pub fn username(&self) -> &str {
        &self.username
    }
    
    // =========================================================================
    // 🔧 FIELD SETTERS WITH CHANGE TRACKING
    // =========================================================================
    
    /// Integer value for counting or quantity tracking
    pub fn set_account_locked_until(&mut self, value: Option<DateTime<Utc>>) {
        self.account_locked_until = value;
        self.mark_changed("account_locked_until", self.account_locked_until.is_none());
    }

    /// Large text field for extended content
    pub fn set_address(&mut self, value: Option<impl Into<String>>) {
        self.address = value.map(|v| v.into());
        self.mark_changed("address", self.address.is_none());
    }

    /// Unique identifier code - ensure uniqueness constraints
    pub fn set_backup_codes(&mut self, value: Option<serde_json::Value>) {
        self.backup_codes = value;
        self.mark_changed("backup_codes", self.backup_codes.is_none());
    }

    /// Unique identifier code - ensure uniqueness constraints
    pub fn set_backup_codes_used(&mut self, value: Option<serde_json::Value>) {
        self.backup_codes_used = value;
        self.mark_changed("backup_codes_used", self.backup_codes_used.is_none());
    }

    /// Database field of type date
    pub fn set_birthdate(&mut self, value: NaiveDate) {
        self.birthdate = value;
        self.mark_changed("birthdate", false);
    }

    /// Record creation timestamp - automatically set on insert
    pub fn set_created_at(&mut self, value: Option<DateTime<Utc>>) {
        self.created_at = value;
        self.mark_changed("created_at", self.created_at.is_none());
    }

    /// Valid email format required. Used for authentication and communication.
    pub fn set_email(&mut self, value: impl Into<String>) {
        self.email = value.into();
        self.mark_changed("email", false);
    }

    /// Valid email format required. Used for authentication and communication.
    pub fn set_email_verified(&mut self, value: i8) {
        self.email_verified = value;
        self.mark_changed("email_verified", false);
    }

    /// Integer value for counting or identification
    pub fn set_failed_login_attempts(&mut self, value: Option<i32>) {
        self.failed_login_attempts = value;
        self.mark_changed("failed_login_attempts", self.failed_login_attempts.is_none());
    }

    /// Display name for user interface and identification
    pub fn set_first_name(&mut self, value: impl Into<String>) {
        self.first_name = value.into();
        self.mark_changed("first_name", false);
    }

    /// Enumerated value - validate against allowed options
    pub fn set_gender(&mut self, value: impl Into<String>) {
        self.gender = value.into();
        self.mark_changed("gender", false);
    }

    /// Status flag - false indicates soft deletion or deactivation
    pub fn set_is_active(&mut self, value: Option<i8>) {
        self.is_active = value;
        self.mark_changed("is_active", self.is_active.is_none());
    }

    /// Display name for user interface and identification
    pub fn set_job_title(&mut self, value: Option<impl Into<String>>) {
        self.job_title = value.map(|v| v.into());
        self.mark_changed("job_title", self.job_title.is_none());
    }

    /// String field with length constraints
    pub fn set_language_preference(&mut self, value: Option<impl Into<String>>) {
        self.language_preference = value.map(|v| v.into());
        self.mark_changed("language_preference", self.language_preference.is_none());
    }

    /// Timestamp field for temporal data tracking
    pub fn set_last_failed_login_at(&mut self, value: Option<DateTime<Utc>>) {
        self.last_failed_login_at = value;
        self.mark_changed("last_failed_login_at", self.last_failed_login_at.is_none());
    }

    /// Display name for user interface and identification
    pub fn set_last_name(&mut self, value: impl Into<String>) {
        self.last_name = value.into();
        self.mark_changed("last_name", false);
    }

    /// Timestamp field for temporal data tracking
    pub fn set_last_success_login_at(&mut self, value: Option<DateTime<Utc>>) {
        self.last_success_login_at = value;
        self.mark_changed("last_success_login_at", self.last_success_login_at.is_none());
    }

    /// Foreign key reference to manager table
    pub fn set_manager_id(&mut self, value: Option<i32>) {
        self.manager_id = value;
        self.mark_changed("manager_id", self.manager_id.is_none());
    }

    /// Phone number with international format support
    pub fn set_mobile_number(&mut self, value: Option<impl Into<String>>) {
        self.mobile_number = value.map(|v| v.into());
        self.mark_changed("mobile_number", self.mobile_number.is_none());
    }

    /// Enumerated value - validate against allowed options
    pub fn set_notification_preference(&mut self, value: Option<impl Into<String>>) {
        self.notification_preference = value.map(|v| v.into());
        self.mark_changed("notification_preference", self.notification_preference.is_none());
    }

    /// Always store as bcrypt hash. Never store plain passwords!
    pub fn set_password_hash(&mut self, value: impl Into<String>) {
        self.password_hash = value.into();
        self.mark_changed("password_hash", false);
    }

    /// Phone number with international format support
    pub fn set_phone_number(&mut self, value: Option<impl Into<String>>) {
        self.phone_number = value.map(|v| v.into());
        self.mark_changed("phone_number", self.phone_number.is_none());
    }

    /// Phone number with international format support
    pub fn set_phone_verified(&mut self, value: i8) {
        self.phone_verified = value;
        self.mark_changed("phone_verified", false);
    }

    /// Last modification timestamp - automatically updated on change
    pub fn set_profile_photo_updated_at(&mut self, value: Option<DateTime<Utc>>) {
        self.profile_photo_updated_at = value;
        self.mark_changed("profile_photo_updated_at", self.profile_photo_updated_at.is_none());
    }

    /// URL field - validate format and accessibility
    pub fn set_profile_photo_url(&mut self, value: Option<impl Into<String>>) {
        self.profile_photo_url = value.map(|v| v.into());
        self.mark_changed("profile_photo_url", self.profile_photo_url.is_none());
    }

    /// Integer value for counting or identification
    pub fn set_totp_enabled(&mut self, value: Option<i8>) {
        self.totp_enabled = value;
        self.mark_changed("totp_enabled", self.totp_enabled.is_none());
    }

    /// Sensitive token - store securely and never log
    pub fn set_totp_secret(&mut self, value: Option<impl Into<String>>) {
        self.totp_secret = value.map(|v| v.into());
        self.mark_changed("totp_secret", self.totp_secret.is_none());
    }

    /// Last modification timestamp - automatically updated on change
    pub fn set_updated_at(&mut self, value: Option<DateTime<Utc>>) {
        self.updated_at = value;
        self.mark_changed("updated_at", self.updated_at.is_none());
    }

    /// Display name for user interface and identification
    pub fn set_username(&mut self, value: impl Into<String>) {
        self.username = value.into();
        self.mark_changed("username", false);
    }
}

// FromRow implementations for each database type

impl sqlx::FromRow<'_, sqlx::mysql::MySqlRow> for Users {
    fn from_row(row: &sqlx::mysql::MySqlRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            account_locked_until: row.try_get("account_locked_until")?,
            address: row.try_get("address")?,
            backup_codes: row.try_get("backup_codes")?,
            backup_codes_used: row.try_get("backup_codes_used")?,
            birthdate: row.try_get("birthdate")?,
            created_at: row.try_get("created_at")?,
            email: row.try_get("email")?,
            email_verified: row.try_get("email_verified")?,
            failed_login_attempts: row.try_get("failed_login_attempts")?,
            first_name: row.try_get("first_name")?,
            gender: row.try_get("gender")?,
            id: row.try_get("id")?,
            is_active: row.try_get("is_active")?,
            job_title: row.try_get("job_title")?,
            language_preference: row.try_get("language_preference")?,
            last_failed_login_at: row.try_get("last_failed_login_at")?,
            last_name: row.try_get("last_name")?,
            last_success_login_at: row.try_get("last_success_login_at")?,
            manager_id: row.try_get("manager_id")?,
            mobile_number: row.try_get("mobile_number")?,
            notification_preference: row.try_get("notification_preference")?,
            password_hash: row.try_get("password_hash")?,
            phone_number: row.try_get("phone_number")?,
            phone_verified: row.try_get("phone_verified")?,
            profile_photo_updated_at: row.try_get("profile_photo_updated_at")?,
            profile_photo_url: row.try_get("profile_photo_url")?,
            totp_enabled: row.try_get("totp_enabled")?,
            totp_secret: row.try_get("totp_secret")?,
            updated_at: row.try_get("updated_at")?,
            username: row.try_get("username")?,
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
    use chrono::{DateTime, Utc, NaiveDate};
    
    pub type account_locked_until = Option<DateTime<Utc>>;
    pub type address = Option<String>;
    pub type backup_codes = Option<serde_json::Value>;
    pub type backup_codes_used = Option<serde_json::Value>;
    pub type birthdate = NaiveDate;
    pub type created_at = Option<DateTime<Utc>>;
    pub type email = String;
    pub type email_verified = i8;
    pub type failed_login_attempts = Option<i32>;
    pub type first_name = String;
    pub type gender = String;
    pub type id = i32;
    pub type is_active = Option<i8>;
    pub type job_title = Option<String>;
    pub type language_preference = Option<String>;
    pub type last_failed_login_at = Option<DateTime<Utc>>;
    pub type last_name = String;
    pub type last_success_login_at = Option<DateTime<Utc>>;
    pub type manager_id = Option<i32>;
    pub type mobile_number = Option<String>;
    pub type notification_preference = Option<String>;
    pub type password_hash = String;
    pub type phone_number = Option<String>;
    pub type phone_verified = i8;
    pub type profile_photo_updated_at = Option<DateTime<Utc>>;
    pub type profile_photo_url = Option<String>;
    pub type totp_enabled = Option<i8>;
    pub type totp_secret = Option<String>;
    pub type updated_at = Option<DateTime<Utc>>;
    pub type username = String;
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
    pub const ACCOUNT_LOCKED_UNTIL: &'static str = "account_locked_until";
    pub const ADDRESS: &'static str = "address";
    pub const BACKUP_CODES: &'static str = "backup_codes";
    pub const BACKUP_CODES_USED: &'static str = "backup_codes_used";
    pub const BIRTHDATE: &'static str = "birthdate";
    pub const CREATED_AT: &'static str = "created_at";
    pub const EMAIL: &'static str = "email";
    pub const EMAIL_VERIFIED: &'static str = "email_verified";
    pub const FAILED_LOGIN_ATTEMPTS: &'static str = "failed_login_attempts";
    pub const FIRST_NAME: &'static str = "first_name";
    pub const GENDER: &'static str = "gender";
    pub const ID: &'static str = "id";
    pub const IS_ACTIVE: &'static str = "is_active";
    pub const JOB_TITLE: &'static str = "job_title";
    pub const LANGUAGE_PREFERENCE: &'static str = "language_preference";
    pub const LAST_FAILED_LOGIN_AT: &'static str = "last_failed_login_at";
    pub const LAST_NAME: &'static str = "last_name";
    pub const LAST_SUCCESS_LOGIN_AT: &'static str = "last_success_login_at";
    pub const MANAGER_ID: &'static str = "manager_id";
    pub const MOBILE_NUMBER: &'static str = "mobile_number";
    pub const NOTIFICATION_PREFERENCE: &'static str = "notification_preference";
    pub const PASSWORD_HASH: &'static str = "password_hash";
    pub const PHONE_NUMBER: &'static str = "phone_number";
    pub const PHONE_VERIFIED: &'static str = "phone_verified";
    pub const PROFILE_PHOTO_UPDATED_AT: &'static str = "profile_photo_updated_at";
    pub const PROFILE_PHOTO_URL: &'static str = "profile_photo_url";
    pub const TOTP_ENABLED: &'static str = "totp_enabled";
    pub const TOTP_SECRET: &'static str = "totp_secret";
    pub const UPDATED_AT: &'static str = "updated_at";
    pub const USERNAME: &'static str = "username";
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
            "account_locked_until" => Ok(SqlValue::from(self.account_locked_until.clone())),
            "address" => Ok(SqlValue::from(self.address.clone())),
            "backup_codes" => Ok(SqlValue::from(self.backup_codes.clone())),
            "backup_codes_used" => Ok(SqlValue::from(self.backup_codes_used.clone())),
            "birthdate" => Ok(SqlValue::from(self.birthdate.clone())),
            "created_at" => Ok(SqlValue::from(self.created_at.clone())),
            "email" => Ok(SqlValue::from(self.email.clone())),
            "email_verified" => Ok(SqlValue::from(self.email_verified.clone())),
            "failed_login_attempts" => Ok(SqlValue::from(self.failed_login_attempts.clone())),
            "first_name" => Ok(SqlValue::from(self.first_name.clone())),
            "gender" => Ok(SqlValue::Enum(self.gender.clone())),
            "id" => Ok(SqlValue::from(self.id.clone())),
            "is_active" => Ok(SqlValue::from(self.is_active.clone())),
            "job_title" => Ok(SqlValue::from(self.job_title.clone())),
            "language_preference" => Ok(SqlValue::from(self.language_preference.clone())),
            "last_failed_login_at" => Ok(SqlValue::from(self.last_failed_login_at.clone())),
            "last_name" => Ok(SqlValue::from(self.last_name.clone())),
            "last_success_login_at" => Ok(SqlValue::from(self.last_success_login_at.clone())),
            "manager_id" => Ok(SqlValue::from(self.manager_id.clone())),
            "mobile_number" => Ok(SqlValue::from(self.mobile_number.clone())),
            "notification_preference" => Ok(self.notification_preference.clone().map(SqlValue::Enum).unwrap_or(SqlValue::Null)),
            "password_hash" => Ok(SqlValue::from(self.password_hash.clone())),
            "phone_number" => Ok(SqlValue::from(self.phone_number.clone())),
            "phone_verified" => Ok(SqlValue::from(self.phone_verified.clone())),
            "profile_photo_updated_at" => Ok(SqlValue::from(self.profile_photo_updated_at.clone())),
            "profile_photo_url" => Ok(SqlValue::from(self.profile_photo_url.clone())),
            "totp_enabled" => Ok(SqlValue::from(self.totp_enabled.clone())),
            "totp_secret" => Ok(SqlValue::from(self.totp_secret.clone())),
            "updated_at" => Ok(SqlValue::from(self.updated_at.clone())),
            "username" => Ok(SqlValue::from(self.username.clone())),
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
    account_locked_until: Option<Option<DateTime<Utc>>>,
    address: Option<Option<String>>,
    backup_codes: Option<Option<serde_json::Value>>,
    backup_codes_used: Option<Option<serde_json::Value>>,
    birthdate: Option<NaiveDate>,
    created_at: Option<Option<DateTime<Utc>>>,
    email: Option<String>,
    email_verified: Option<i8>,
    failed_login_attempts: Option<Option<i32>>,
    first_name: Option<String>,
    gender: Option<String>,
    is_active: Option<Option<i8>>,
    job_title: Option<Option<String>>,
    language_preference: Option<Option<String>>,
    last_failed_login_at: Option<Option<DateTime<Utc>>>,
    last_name: Option<String>,
    last_success_login_at: Option<Option<DateTime<Utc>>>,
    manager_id: Option<Option<i32>>,
    mobile_number: Option<Option<String>>,
    notification_preference: Option<Option<String>>,
    password_hash: Option<String>,
    phone_number: Option<Option<String>>,
    phone_verified: Option<i8>,
    profile_photo_updated_at: Option<Option<DateTime<Utc>>>,
    profile_photo_url: Option<Option<String>>,
    totp_enabled: Option<Option<i8>>,
    totp_secret: Option<Option<String>>,
    updated_at: Option<Option<DateTime<Utc>>>,
    username: Option<String>,
}

impl UsersBuilder {
    /// Create a new builder with default values
    pub fn new() -> Self {
        Self {
            account_locked_until: None,
            address: None,
            backup_codes: None,
            backup_codes_used: None,
            birthdate: None,
            created_at: None,
            email: None,
            email_verified: None,
            failed_login_attempts: None,
            first_name: None,
            gender: None,
            is_active: None,
            job_title: None,
            language_preference: None,
            last_failed_login_at: None,
            last_name: None,
            last_success_login_at: None,
            manager_id: None,
            mobile_number: None,
            notification_preference: None,
            password_hash: None,
            phone_number: None,
            phone_verified: None,
            profile_photo_updated_at: None,
            profile_photo_url: None,
            totp_enabled: None,
            totp_secret: None,
            updated_at: None,
            username: None,
        }
    }
    
    /// Integer value for counting or quantity tracking
    pub fn account_locked_until(mut self, value: Option<DateTime<Utc>>) -> Self {
        self.account_locked_until = Some(value);
        self
    }

    /// Large text field for extended content
    pub fn address(mut self, value: Option<impl Into<String>>) -> Self {
        self.address = Some(value.map(|v| v.into()));
        self
    }

    /// Unique identifier code - ensure uniqueness constraints
    pub fn backup_codes(mut self, value: Option<serde_json::Value>) -> Self {
        self.backup_codes = Some(value);
        self
    }

    /// Unique identifier code - ensure uniqueness constraints
    pub fn backup_codes_used(mut self, value: Option<serde_json::Value>) -> Self {
        self.backup_codes_used = Some(value);
        self
    }

    /// Database field of type date
    pub fn birthdate(mut self, value: NaiveDate) -> Self {
        self.birthdate = Some(value);
        self
    }

    /// Record creation timestamp - automatically set on insert
    pub fn created_at(mut self, value: Option<DateTime<Utc>>) -> Self {
        self.created_at = Some(value);
        self
    }

    /// Valid email format required. Used for authentication and communication.
    pub fn email(mut self, value: impl Into<String>) -> Self {
        self.email = Some(value.into());
        self
    }

    /// Valid email format required. Used for authentication and communication.
    pub fn email_verified(mut self, value: i8) -> Self {
        self.email_verified = Some(value);
        self
    }

    /// Integer value for counting or identification
    pub fn failed_login_attempts(mut self, value: Option<i32>) -> Self {
        self.failed_login_attempts = Some(value);
        self
    }

    /// Display name for user interface and identification
    pub fn first_name(mut self, value: impl Into<String>) -> Self {
        self.first_name = Some(value.into());
        self
    }

    /// Enumerated value - validate against allowed options
    pub fn gender(mut self, value: impl Into<String>) -> Self {
        self.gender = Some(value.into());
        self
    }

    /// Status flag - false indicates soft deletion or deactivation
    pub fn is_active(mut self, value: Option<i8>) -> Self {
        self.is_active = Some(value);
        self
    }

    /// Display name for user interface and identification
    pub fn job_title(mut self, value: Option<impl Into<String>>) -> Self {
        self.job_title = Some(value.map(|v| v.into()));
        self
    }

    /// String field with length constraints
    pub fn language_preference(mut self, value: Option<impl Into<String>>) -> Self {
        self.language_preference = Some(value.map(|v| v.into()));
        self
    }

    /// Timestamp field for temporal data tracking
    pub fn last_failed_login_at(mut self, value: Option<DateTime<Utc>>) -> Self {
        self.last_failed_login_at = Some(value);
        self
    }

    /// Display name for user interface and identification
    pub fn last_name(mut self, value: impl Into<String>) -> Self {
        self.last_name = Some(value.into());
        self
    }

    /// Timestamp field for temporal data tracking
    pub fn last_success_login_at(mut self, value: Option<DateTime<Utc>>) -> Self {
        self.last_success_login_at = Some(value);
        self
    }

    /// Foreign key reference to manager table
    pub fn manager_id(mut self, value: Option<i32>) -> Self {
        self.manager_id = Some(value);
        self
    }

    /// Phone number with international format support
    pub fn mobile_number(mut self, value: Option<impl Into<String>>) -> Self {
        self.mobile_number = Some(value.map(|v| v.into()));
        self
    }

    /// Enumerated value - validate against allowed options
    pub fn notification_preference(mut self, value: Option<impl Into<String>>) -> Self {
        self.notification_preference = Some(value.map(|v| v.into()));
        self
    }

    /// Always store as bcrypt hash. Never store plain passwords!
    pub fn password_hash(mut self, value: impl Into<String>) -> Self {
        self.password_hash = Some(value.into());
        self
    }

    /// Phone number with international format support
    pub fn phone_number(mut self, value: Option<impl Into<String>>) -> Self {
        self.phone_number = Some(value.map(|v| v.into()));
        self
    }

    /// Phone number with international format support
    pub fn phone_verified(mut self, value: i8) -> Self {
        self.phone_verified = Some(value);
        self
    }

    /// Last modification timestamp - automatically updated on change
    pub fn profile_photo_updated_at(mut self, value: Option<DateTime<Utc>>) -> Self {
        self.profile_photo_updated_at = Some(value);
        self
    }

    /// URL field - validate format and accessibility
    pub fn profile_photo_url(mut self, value: Option<impl Into<String>>) -> Self {
        self.profile_photo_url = Some(value.map(|v| v.into()));
        self
    }

    /// Integer value for counting or identification
    pub fn totp_enabled(mut self, value: Option<i8>) -> Self {
        self.totp_enabled = Some(value);
        self
    }

    /// Sensitive token - store securely and never log
    pub fn totp_secret(mut self, value: Option<impl Into<String>>) -> Self {
        self.totp_secret = Some(value.map(|v| v.into()));
        self
    }

    /// Last modification timestamp - automatically updated on change
    pub fn updated_at(mut self, value: Option<DateTime<Utc>>) -> Self {
        self.updated_at = Some(value);
        self
    }

    /// Display name for user interface and identification
    pub fn username(mut self, value: impl Into<String>) -> Self {
        self.username = Some(value.into());
        self
    }
    
    /// Validate the builder has all required fields
    /// Returns Ok(()) if valid, or Err with list of missing fields
    pub fn validate(&self) -> Result<(), Vec<&'static str>> {
        let mut missing = Vec::new();
        
        if self.birthdate.is_none() {
            missing.push("birthdate");
        }
        if self.email.is_none() {
            missing.push("email");
        }
        if self.email_verified.is_none() {
            missing.push("email_verified");
        }
        if self.first_name.is_none() {
            missing.push("first_name");
        }
        if self.gender.is_none() {
            missing.push("gender");
        }
        if self.last_name.is_none() {
            missing.push("last_name");
        }
        if self.password_hash.is_none() {
            missing.push("password_hash");
        }
        if self.phone_verified.is_none() {
            missing.push("phone_verified");
        }
        if self.username.is_none() {
            missing.push("username");
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
            account_locked_until: self.account_locked_until.flatten(),
            address: self.address.flatten(),
            backup_codes: self.backup_codes.flatten(),
            backup_codes_used: self.backup_codes_used.flatten(),
            birthdate: self.birthdate.unwrap(),
            created_at: self.created_at.flatten(),
            email: self.email.unwrap(),
            email_verified: self.email_verified.unwrap(),
            failed_login_attempts: self.failed_login_attempts.flatten(),
            first_name: self.first_name.unwrap(),
            gender: self.gender.unwrap(),
            id: Default::default(), // Auto-generated
            is_active: self.is_active.flatten(),
            job_title: self.job_title.flatten(),
            language_preference: self.language_preference.flatten(),
            last_failed_login_at: self.last_failed_login_at.flatten(),
            last_name: self.last_name.unwrap(),
            last_success_login_at: self.last_success_login_at.flatten(),
            manager_id: self.manager_id.flatten(),
            mobile_number: self.mobile_number.flatten(),
            notification_preference: self.notification_preference.flatten(),
            password_hash: self.password_hash.unwrap(),
            phone_number: self.phone_number.flatten(),
            phone_verified: self.phone_verified.unwrap(),
            profile_photo_updated_at: self.profile_photo_updated_at.flatten(),
            profile_photo_url: self.profile_photo_url.flatten(),
            totp_enabled: self.totp_enabled.flatten(),
            totp_secret: self.totp_secret.flatten(),
            updated_at: self.updated_at.flatten(),
            username: self.username.unwrap(),
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
        insert_data.insert("account_locked_until".to_string(), SqlValue::from(model.account_locked_until));
        insert_data.insert("address".to_string(), SqlValue::from(model.address));
        insert_data.insert("backup_codes".to_string(), SqlValue::from(model.backup_codes));
        insert_data.insert("backup_codes_used".to_string(), SqlValue::from(model.backup_codes_used));
        insert_data.insert("birthdate".to_string(), SqlValue::from(model.birthdate));
        insert_data.insert("created_at".to_string(), SqlValue::from(model.created_at));
        insert_data.insert("email".to_string(), SqlValue::from(model.email));
        insert_data.insert("email_verified".to_string(), SqlValue::from(model.email_verified));
        insert_data.insert("failed_login_attempts".to_string(), SqlValue::from(model.failed_login_attempts));
        insert_data.insert("first_name".to_string(), SqlValue::from(model.first_name));
        insert_data.insert("gender".to_string(), SqlValue::Enum(model.gender.clone()));
        insert_data.insert("is_active".to_string(), SqlValue::from(model.is_active));
        insert_data.insert("job_title".to_string(), SqlValue::from(model.job_title));
        insert_data.insert("language_preference".to_string(), SqlValue::from(model.language_preference));
        insert_data.insert("last_failed_login_at".to_string(), SqlValue::from(model.last_failed_login_at));
        insert_data.insert("last_name".to_string(), SqlValue::from(model.last_name));
        insert_data.insert("last_success_login_at".to_string(), SqlValue::from(model.last_success_login_at));
        insert_data.insert("manager_id".to_string(), SqlValue::from(model.manager_id));
        insert_data.insert("mobile_number".to_string(), SqlValue::from(model.mobile_number));
        insert_data.insert("notification_preference".to_string(), model.notification_preference.clone().map(SqlValue::Enum).unwrap_or(SqlValue::Null));
        insert_data.insert("password_hash".to_string(), SqlValue::from(model.password_hash));
        insert_data.insert("phone_number".to_string(), SqlValue::from(model.phone_number));
        insert_data.insert("phone_verified".to_string(), SqlValue::from(model.phone_verified));
        insert_data.insert("profile_photo_updated_at".to_string(), SqlValue::from(model.profile_photo_updated_at));
        insert_data.insert("profile_photo_url".to_string(), SqlValue::from(model.profile_photo_url));
        insert_data.insert("totp_enabled".to_string(), SqlValue::from(model.totp_enabled));
        insert_data.insert("totp_secret".to_string(), SqlValue::from(model.totp_secret));
        insert_data.insert("updated_at".to_string(), SqlValue::from(model.updated_at));
        insert_data.insert("username".to_string(), SqlValue::from(model.username));
        
        let query_builder = QueryBuilder::new(DatabaseBackend::MySQL)
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