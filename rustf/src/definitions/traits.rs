//! Simple traits for extending RustF framework functionality
//!
//! These traits define the contracts for user-defined extensions
//! that can be auto-discovered from the definitions folder.

use crate::config::SessionConfig;
use crate::error::Result;
use crate::session::SessionStorage;
use std::sync::Arc;

/// Trait for registering custom view helpers
///
/// Implement this trait in `definitions/helpers.rs` to add
/// custom template helper functions.
pub trait Helpers {
    /// Register helper functions with the registry
    fn register(&self, registry: &mut super::HelperRegistry);
}

/// Trait for registering custom validators
///
/// Implement this trait in `definitions/validators.rs` to add
/// custom validation logic.
pub trait Validators {
    /// Register validators with the registry
    fn register(&self, registry: &mut super::ValidatorRegistry);
}

/// Factory function signature for custom session storage
///
/// Implement a function with this signature in `definitions/session_storage.rs`
/// to provide custom session storage backend.
pub type SessionStorageFactory = fn(&SessionConfig) -> Result<Arc<dyn SessionStorage>>;

/// Factory function signature for custom cache storage
///
/// Implement a function with this signature in `definitions/cache_storage.rs`
/// to provide custom cache backend.
pub type CacheStorageFactory<T> = fn() -> Result<Arc<dyn crate::cache::Cache<T>>>;
