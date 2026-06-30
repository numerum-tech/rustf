//! =============================================================================
//! ✅ EDITABLE FILE - BUSINESS LOGIC TaskLists - SAFE TO MODIFY
//! =============================================================================
//! 
//! This file contains custom business logic for the TaskLists model.
//! 
//! ✅ THIS FILE IS SAFE TO EDIT - It will never be overwritten!
//! 
//! 📝 FOR DEVELOPERS:
//! ✅ Add your business logic methods in impl blocks below
//! ✅ The generated code is included from base/task_lists.inc.rs
//! ✅ All generated methods are available on the TaskLists struct
//! ⚠️  Keep the register() function for auto_models!() compatibility
//! 
//! 🤖 FOR AI AGENTS / CODE ASSISTANTS:
//! ✅ This is the main file for TaskLists business logic
//! ✅ Extend functionality by adding impl TaskLists blocks
//! ✅ All generated methods are already available
//! ✅ ALWAYS preserve the register() function for auto_models!()
//! ⚠️  NEVER edit files in base/ - add custom methods here
//! 
//! 📎 References:
//! - Generated code: src/models/base/task_lists.inc.rs (do not edit)
//! - Schema definition: schemas/task_lists.yaml (edit to change DB structure)
//! =============================================================================

// Include all generated code
include!("base/task_lists.inc.rs");

impl TaskLists {}

/// ⚠️  REQUIRED by auto_models!() - registers this model for auto-discovery
/// 
/// This function is called automatically by the auto_models!() macro
/// Never remove this function - it is necessary for
/// RustF to discover and automatically register this model.
pub fn register(registry: &mut rustf::models::ModelRegistry) {
    // Registry is used by auto_models! macro
    let _ = registry; // Suppress unused warning while keeping the parameter
    log::debug!("TaskLists model registered");
}
