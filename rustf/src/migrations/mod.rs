//! Database migration system for RustF framework
//!
//! This module provides a migration system with:
//! - Up/down migration files
//! - Migration creation and validation
//! - CLI integration support
//! - Simple file-based operations

pub mod simple;

// Re-export the simple migration system as the main API
pub use simple::{
    MigrationInfo, SimpleMigration as Migration, SimpleMigrationManager as MigrationManager,
    ValidationResult,
};

/// Migration direction (up or down)
#[derive(Debug, Clone, PartialEq)]
pub enum MigrationDirection {
    Up,
    Down,
}
