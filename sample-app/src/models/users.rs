//! =============================================================================
//! ✅ EDITABLE FILE - BUSINESS LOGIC Users - SAFE TO MODIFY
//! =============================================================================
//! 
//! This file contains custom business logic for the Users model.
//! 
//! ✅ THIS FILE IS SAFE TO EDIT - It will never be overwritten!
//! 
//! 📝 FOR DEVELOPERS:
//! ✅ Add your business logic methods in impl blocks below
//! ✅ The generated code is included from base/users.inc.rs
//! ✅ All generated methods are available on the Users struct
//! ⚠️  Keep the register() function for auto_models!() compatibility
//! 
//! 🤖 FOR AI AGENTS / CODE ASSISTANTS:
//! ✅ This is the main file for Users business logic
//! ✅ Extend functionality by adding impl Users blocks
//! ✅ All generated methods are already available
//! ✅ ALWAYS preserve the register() function for auto_models!()
//! ⚠️  NEVER edit files in base/ - add custom methods here
//! 
//! 📎 References:
//! - Generated code: src/models/base/users.inc.rs (do not edit)
//! - Schema definition: schemas/users.yaml (edit to change DB structure)
//! =============================================================================

// Include all generated code
include!("base/users.inc.rs");

// =========================================================================
// ✅ CUSTOM BUSINESS LOGIC - Add your methods here
// =========================================================================

impl Users {
    // 📝 ADD YOUR CUSTOM BUSINESS METHODS HERE
    // 
    // 📚 BUILDER PATTERN USAGE:
    // ========================
    // 
    // Creating a new record with the builder:
    // ----------------------------------------
    // let new_record = Users::builder()
    //     .field1("value1")              // Required fields
    //     .field2(42)                     
    //     .optional_field(Some("value")) // Optional fields
    //     .save(&pool)                    // Save to database
    //     .await?;
    //
    // The builder validates required fields and saves directly to the database.
    // All fields have type-safe setter methods generated automatically.
    //
    // 📚 QUERY METHODS:
    // =================
    //
    // Find by ID:
    // -----------
    // let record = Users::get_by_id(id).await?;
    // 
    // Query with filters:
    // -------------------
    // let results = Users::query()?
    //     .where_eq("status", "active")
    //     .where_like("name", "%search%")
    //     .where_in("role", vec!["admin", "user"])
    //     .where_not_null("email")
    //     .order_by("created_at", OrderDirection::Desc)
    //     .limit(10)
    //     .get_all()
    //     .await?;
    //
    // Count records:
    // --------------
    // let total = Users::count().await?;
    // let active_count = Users::query()?
    //     .where_eq("is_active", true)
    //     .count()
    //     .await?;
    //
    // 📚 UPDATE OPERATIONS:
    // =====================
    //
    // Smart updates (only changed fields):
    // -------------------------------------
    // let mut record = Users::get_by_id(id).await?.unwrap();
    // record.set_field1("new_value");    // Mark field as changed
    // record.set_field2(123);             // Mark another field as changed
    // record.update(&pool).await?;       // Only updates changed fields!
    //
    // The update() method automatically tracks changes and generates
    // optimized UPDATE queries with only the modified fields.
    //
    // 📚 GETTERS AND SETTERS:
    // =======================
    //
    // All fields have generated getters and setters:
    // -----------------------------------------------
    // // Getters return appropriate types:
    // let value1 = record.field1();      // Returns &str for String fields
    // let value2 = record.field2();      // Returns i32 for integer fields  
    // let opt = record.optional_field(); // Returns Option<&str> for Option<String>
    //
    // // Setters automatically track changes:
    // record.set_field1("value");        // Marks field1 as changed
    // record.set_field2(42);              // Marks field2 as changed
    //
    // // Check if fields have been modified:
    // if record.has_changes() {
    //     record.update(&pool).await?;
    // }
    //
    // 📚 CUSTOM BUSINESS LOGIC EXAMPLES:
    // ==================================
    //
    // Example: Find by unique field
    // ------------------------------
    // pub async fn find_by_email(email: &str) -> rustf::Result<Option<Self>> {
    //     Self::query()?
    //         .where_eq("email", email)
    //         .get_first()
    //         .await
    // }
    //
    // Example: Complex business query
    // --------------------------------
    // pub async fn find_active_admins() -> rustf::Result<Vec<Self>> {
    //     Self::query()?
    //         .where_eq("is_active", true)
    //         .where_eq("role", "admin")
    //         .where_not_null("verified_at")
    //         .order_by("last_login_at", OrderDirection::Desc)
    //         .get_all()
    //         .await
    // }
    //
    // Example: Business validation
    // -----------------------------
    // pub async fn activate(&mut self, pool: &Pool<Postgres>) -> rustf::Result<()> {
    //     self.set_is_active(true);
    //     self.set_activated_at(Some(Utc::now()));
    //     self.update(pool).await
    // }
    //
    // Example: Authentication
    // ------------------------
    // pub async fn verify_password(&self, password: &str) -> bool {
    //     // Assuming password is hashed with bcrypt
    //     bcrypt::verify(password, &self.password_hash).unwrap_or(false)
    // }
}

/// ⚠️  REQUIRED by auto_models!() - registers this model for auto-discovery
/// 
/// This function is called automatically by the auto_models!() macro
/// Never remove this function - it is necessary for
/// RustF to discover and automatically register this model.
pub fn register(registry: &mut rustf::models::ModelRegistry) {
    // Registry is used by auto_models! macro
    let _ = registry; // Suppress unused warning while keeping the parameter
    log::debug!("Users model registered");
}