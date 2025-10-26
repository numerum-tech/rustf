//! RustF Multi-Database Query Builder - Legacy Compatibility Module
//!
//! This file maintains backward compatibility by re-exporting all the types
//! from the new modular query_builder structure. The actual implementation
//! has been moved to separate modules for better organization:
//!
//! - dialects/: Database-specific SQL generation
//! - core.rs: Main query building logic
//! - schema.rs: Schema/DDL building
//! - database.rs: Database connection management
//!
//! All existing code should continue working without changes.

// Import the modular structure
#[path = "query_builder_modules/mod.rs"]
mod query_builder_modules;

// Re-export everything from the modular query_builder for backward compatibility
pub use query_builder_modules::*;
