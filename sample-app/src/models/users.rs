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

impl Users {
    pub async fn find_by_email(email: &str) -> rustf::Result<Option<Self>> {
        Self::query()?
            .where_eq("email", email)
            .get_first()
            .await
    }
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
