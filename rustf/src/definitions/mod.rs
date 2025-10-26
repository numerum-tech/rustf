//! RustF Definitions System - Simplified
//!
//! This module provides a simple, convention-based system for extending
//! the framework with custom helpers, validators, and storage backends.
//!
//! # Convention
//!
//! Place your extensions in the `definitions/` folder:
//! - `helpers.rs` - Custom template helpers
//! - `validators.rs` - Custom validators
//! - `session_storage.rs` - Custom session storage
//! - `cache_storage.rs` - Custom cache storage
//!
//! # Example
//!
//! ```rust
//! // In definitions/helpers.rs
//! use rustf::definitions::{Helpers, HelperRegistry};
//!
//! pub struct AppHelpers;
//!
//! impl Helpers for AppHelpers {
//!     fn register(&self, registry: &mut HelperRegistry) {
//!         registry.register("format_money", |v| format!("${:.2}", v));
//!     }
//! }
//! ```

pub mod helpers;
pub mod traits;
pub mod validators;

pub use helpers::{Helper, HelperRegistry};
pub use traits::*;
pub use validators::{Validator, ValidatorRegistry};

use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global definitions instance
static GLOBAL_DEFINITIONS: Lazy<Arc<RwLock<Definitions>>> =
    Lazy::new(|| Arc::new(RwLock::new(Definitions::new())));

/// Container for all user-defined extensions
pub struct Definitions {
    /// Registry of template helper functions
    pub helpers: HelperRegistry,

    /// Registry of validators for data validation
    pub validators: ValidatorRegistry,

    /// Factory function for custom session storage
    pub session_storage_factory: Option<traits::SessionStorageFactory>,

    /// Factory function for custom cache storage (future use)
    pub cache_storage_factory: Option<Box<dyn std::any::Any + Send + Sync>>,
}

impl Definitions {
    /// Create a new empty definitions container
    pub fn new() -> Self {
        Self {
            helpers: HelperRegistry::new(),
            validators: ValidatorRegistry::new(),
            session_storage_factory: None,
            cache_storage_factory: None,
        }
    }

    /// Register a helper function
    pub fn register_helper(&mut self, name: &str, helper: impl Helper + 'static) {
        self.helpers.register(name, helper);
    }

    /// Register a validator
    pub fn register_validator(&mut self, name: &str, validator: impl Validator + 'static) {
        self.validators.register(name, validator);
    }

    /// Check if a helper exists
    pub fn has_helper(&self, name: &str) -> bool {
        self.helpers.exists(name)
    }

    /// Check if a validator exists
    pub fn has_validator(&self, name: &str) -> bool {
        self.validators.exists(name)
    }

    /// Set the session storage factory
    pub fn set_session_storage_factory(&mut self, factory: traits::SessionStorageFactory) {
        log::debug!("Registering custom session storage factory");
        self.session_storage_factory = Some(factory);
    }

    /// Get the session storage factory
    pub fn get_session_storage_factory(&self) -> Option<traits::SessionStorageFactory> {
        self.session_storage_factory
    }

    /// Check if a custom session storage factory is registered
    pub fn has_session_storage_factory(&self) -> bool {
        self.session_storage_factory.is_some()
    }

    /// Initialize all registered components
    pub async fn initialize(&self) -> crate::error::Result<()> {
        log::info!("Initializing RustF definitions");

        log::debug!("Registered {} helpers", self.helpers.count());
        log::debug!("Registered {} validators", self.validators.count());
        if self.session_storage_factory.is_some() {
            log::debug!("Custom session storage factory registered");
        }

        Ok(())
    }
}

impl Default for Definitions {
    fn default() -> Self {
        Self::new()
    }
}

/// Installation function type for definitions modules
///
/// This is the signature for functions that install definitions.
pub type InstallFn = fn(&mut Definitions);

/// Load definitions from user-defined modules
///
/// This function is called during application startup to register
/// all custom definitions from the application's definitions folder.
pub async fn load(definitions: Definitions) -> crate::error::Result<()> {
    log::info!("Loading user definitions");

    // Store in global instance
    *GLOBAL_DEFINITIONS.write().await = definitions;

    // Initialize
    GLOBAL_DEFINITIONS.read().await.initialize().await?;

    Ok(())
}

/// Get the global definitions instance
pub async fn get() -> Arc<RwLock<Definitions>> {
    GLOBAL_DEFINITIONS.clone()
}

/// Get mutable access to the global definitions instance
pub async fn get_mut() -> tokio::sync::RwLockWriteGuard<'static, Definitions> {
    GLOBAL_DEFINITIONS.write().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_definitions_creation() {
        let defs = Definitions::new();
        // HelperRegistry and ValidatorRegistry register built-in components on creation
        assert!(defs.helpers.count() > 0);
        assert!(defs.validators.count() > 0);
    }

    #[tokio::test]
    async fn test_global_definitions() {
        let defs = get().await;
        // Should be able to access global definitions
        assert!(defs.read().await.helpers.count() >= 0);
    }
}
