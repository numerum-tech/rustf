//! Shared code module system for RustF
//!
//! This module provides a framework for organizing shared code across the application,
//! including both business logic services and utility functions. The system supports
//! auto-discovery and provides type-safe access to shared modules throughout the application.
//!
//! # Global Module Access
//!
//! The MODULE system provides Total.js-style global access to registered modules:
//!
//! ```rust,ignore
//! // Type-based access (recommended)
//! let email = MODULE::<EmailService>()?;
//!
//! // Name-based access
//! let service = MODULE::get("EmailService")?;
//!
//! // Check if module exists
//! if MODULE::exists::<EmailService>() {
//!     // Module is available
//! }
//! ```

use crate::error::{Error, Result as RustfResult};
use anyhow::Result;
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// Trait that all shared modules must implement
///
/// This provides a common interface for both services (stateful business logic)
/// and utilities (stateless functions) to be registered and accessed uniformly.
#[async_trait]
pub trait SharedModule: Send + Sync {
    /// Returns the name of this shared module
    fn name(&self) -> &'static str;

    /// Returns the module type (service, util, helper, etc.)
    fn module_type(&self) -> SharedModuleType;

    /// Initialize the module (called once during app startup)
    ///
    /// This is useful for modules that need to set up connections,
    /// load configuration, or perform other initialization tasks.
    async fn initialize(&self) -> Result<()> {
        Ok(())
    }

    /// Shutdown the module (called during app shutdown)
    ///
    /// This allows modules to clean up resources, close connections, etc.
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    /// Return self as Any for type casting
    fn as_any(&self) -> &dyn Any;
}

/// Types of shared modules supported by the framework
#[derive(Debug, Clone, PartialEq)]
pub enum SharedModuleType {
    /// Business logic services (stateful, with side effects)
    Service,
    /// Pure utility functions (stateless, no side effects)
    Util,
    /// Template and view helpers
    Helper,
    /// Custom trait definitions and interfaces
    Trait,
}

impl std::fmt::Display for SharedModuleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SharedModuleType::Service => write!(f, "service"),
            SharedModuleType::Util => write!(f, "util"),
            SharedModuleType::Helper => write!(f, "helper"),
            SharedModuleType::Trait => write!(f, "trait"),
        }
    }
}

/// Registry for managing shared modules
///
/// The SharedRegistry maintains a type-safe registry of shared modules that can be
/// accessed throughout the application. It supports both registration during app
/// startup and runtime access with compile-time type safety.
pub struct SharedRegistry {
    modules: HashMap<TypeId, Arc<dyn SharedModule>>,
    modules_by_name: HashMap<String, Arc<dyn SharedModule>>,
}

impl SharedRegistry {
    /// Create a new empty shared registry
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            modules_by_name: HashMap::new(),
        }
    }

    /// Register a shared module with the registry
    ///
    /// # Example
    /// ```rust,ignore
    /// registry.register(EmailService::new());
    /// registry.register(ValidationUtils);
    /// ```
    pub fn register<T: SharedModule + 'static>(&mut self, module: T) {
        let module = Arc::new(module);
        let type_id = TypeId::of::<T>();
        let name = module.name().to_string();

        log::info!("Registering shared {}: {}", module.module_type(), name);

        self.modules
            .insert(type_id, module.clone() as Arc<dyn SharedModule>);
        self.modules_by_name
            .insert(name, module as Arc<dyn SharedModule>);
    }

    /// Get a shared module by type
    ///
    /// # Example
    /// ```rust,ignore
    /// let email_service = registry.get::<EmailService>()
    ///     .ok_or_else(|| Error::internal_error("EmailService not registered"))?;
    /// ```
    pub fn get<T: SharedModule + 'static>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        self.modules
            .get(&type_id)
            .and_then(|module| module.as_ref().as_any().downcast_ref::<T>())
    }

    /// Get a shared module by name (useful for dynamic access)
    pub fn get_by_name(&self, name: &str) -> Option<Arc<dyn SharedModule>> {
        self.modules_by_name.get(name).cloned()
    }

    /// Initialize all registered modules
    ///
    /// This should be called during application startup to allow modules
    /// to perform any necessary initialization.
    pub async fn initialize_all(&self) -> Result<()> {
        for (name, module) in &self.modules_by_name {
            log::debug!("Initializing shared module: {}", name);
            module
                .initialize()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to initialize module '{}': {}", name, e))?;
        }
        Ok(())
    }

    /// Shutdown all registered modules
    ///
    /// This should be called during application shutdown to allow modules
    /// to clean up resources properly.
    pub async fn shutdown_all(&self) -> Result<()> {
        for (name, module) in &self.modules_by_name {
            log::debug!("Shutting down shared module: {}", name);
            if let Err(e) = module.shutdown().await {
                log::warn!("Error shutting down module '{}': {}", name, e);
            }
        }
        Ok(())
    }

    /// List all registered modules
    pub fn list_modules(&self) -> Vec<(&str, SharedModuleType)> {
        self.modules_by_name
            .values()
            .map(|module| (module.name(), module.module_type()))
            .collect()
    }

    /// Check if a module is registered by type
    pub fn contains<T: SharedModule + 'static>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        self.modules.contains_key(&type_id)
    }

    /// Check if a module is registered by name
    pub fn contains_name(&self, name: &str) -> bool {
        self.modules_by_name.contains_key(name)
    }
}

impl Default for SharedRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience macro for implementing SharedModule trait for utility structs
///
/// # Example
/// ```rust,ignore
/// use rustf::impl_shared_util;
///
/// pub struct ValidationUtils;
/// impl_shared_util!(ValidationUtils);
/// ```
#[macro_export]
macro_rules! impl_shared_util {
    ($struct_name:ident) => {
        #[async_trait::async_trait]
        impl $crate::shared::SharedModule for $struct_name {
            fn name(&self) -> &'static str {
                stringify!($struct_name)
            }

            fn module_type(&self) -> $crate::shared::SharedModuleType {
                $crate::shared::SharedModuleType::Util
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }
    };
}

/// Convenience macro for implementing SharedModule trait for service structs
///
/// # Example
/// ```rust,ignore
/// use rustf::impl_shared_service;
///
/// pub struct EmailService;
/// impl_shared_service!(EmailService);
/// ```
#[macro_export]
macro_rules! impl_shared_service {
    ($struct_name:ident) => {
        #[async_trait::async_trait]
        impl $crate::shared::SharedModule for $struct_name {
            fn name(&self) -> &'static str {
                stringify!($struct_name)
            }

            fn module_type(&self) -> $crate::shared::SharedModuleType {
                $crate::shared::SharedModuleType::Service
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }
    };
}

/// Convenience macro for implementing SharedModule trait for helper structs
///
/// # Example
/// ```rust,ignore
/// use rustf::impl_shared_helper;
///
/// pub struct FormHelpers;
/// impl_shared_helper!(FormHelpers);
/// ```
#[macro_export]
macro_rules! impl_shared_helper {
    ($struct_name:ident) => {
        #[async_trait::async_trait]
        impl $crate::shared::SharedModule for $struct_name {
            fn name(&self) -> &'static str {
                stringify!($struct_name)
            }

            fn module_type(&self) -> $crate::shared::SharedModuleType {
                $crate::shared::SharedModuleType::Helper
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test utility struct
    pub struct TestUtils;
    impl_shared_util!(TestUtils);

    impl TestUtils {
        pub fn add_numbers(a: i32, b: i32) -> i32 {
            a + b
        }
    }

    // Test service struct
    pub struct TestService;
    impl_shared_service!(TestService);

    impl TestService {
        pub async fn do_something(&self) -> Result<String> {
            Ok("Service result".to_string())
        }
    }

    #[tokio::test]
    async fn test_shared_registry() {
        let mut registry = SharedRegistry::new();

        // Register modules
        registry.register(TestUtils);
        registry.register(TestService);

        // Test type-based access
        let _utils = registry.get::<TestUtils>().unwrap();
        assert_eq!(TestUtils::add_numbers(2, 3), 5);

        let service = registry.get::<TestService>().unwrap();
        let result = service.do_something().await.unwrap();
        assert_eq!(result, "Service result");

        // Test name-based access
        assert!(registry.contains_name("TestUtils"));
        assert!(registry.contains_name("TestService"));

        // Test module listing
        let modules = registry.list_modules();
        assert_eq!(modules.len(), 2);

        // Test initialization/shutdown
        registry.initialize_all().await.unwrap();
        registry.shutdown_all().await.unwrap();
    }
}

// ============================================================================
// Global MODULE System - Total.js Style Access
// ============================================================================

/// Global module registry instance
static MODULE_REGISTRY: OnceCell<Arc<SharedRegistry>> = OnceCell::new();

/// Global MODULE accessor for Total.js-style module access
///
/// Provides global access to registered modules without needing Context or
/// dependency injection. This follows the same pattern as APP and CONF.
///
/// # Examples
/// ```rust,ignore
/// // Type-based access (recommended)
/// let email = MODULE::<EmailService>()?;
/// email.send_notification("user@example.com").await?;
///
/// // Check if module exists
/// if MODULE::exists::<EmailService>() {
///     // Module is available
/// }
///
/// // Name-based access (for dynamic scenarios)
/// let service = MODULE::get("EmailService")?;
/// ```
pub struct MODULE;

impl MODULE {
    /// Initialize the global module registry
    ///
    /// This should be called once during application startup by the framework.
    /// It connects the SharedRegistry to the global MODULE system.
    ///
    /// # Arguments
    /// * `registry` - The SharedRegistry containing all registered modules
    ///
    /// # Returns
    /// * `Ok(())` if initialization succeeded
    /// * `Err` if the registry was already initialized
    pub fn init(registry: Arc<SharedRegistry>) -> RustfResult<()> {
        MODULE_REGISTRY
            .set(registry)
            .map_err(|_| Error::internal("MODULE registry has already been initialized"))
    }

    /// Get a module by type (recommended approach)
    ///
    /// This is the primary way to access modules - it's type-safe and efficient.
    /// Returns a reference to the singleton instance of the module.
    ///
    /// # Type Parameters
    /// * `T` - The module type to retrieve
    ///
    /// # Returns
    /// * `Ok(&T)` if the module is registered
    /// * `Err` if the module is not found or not initialized
    ///
    /// # Examples
    /// ```rust,ignore
    /// let email_service = MODULE::<EmailService>()?;
    /// let cache_service = MODULE::<CacheService>()?;
    /// ```
    pub fn get_typed<T: SharedModule + 'static>() -> RustfResult<&'static T> {
        let registry = MODULE_REGISTRY.get().ok_or_else(|| {
            Error::internal(
                "MODULE registry not initialized. Ensure modules are registered during app startup",
            )
        })?;

        registry.get::<T>().ok_or_else(|| {
            Error::internal(format!(
                "Module of type {} not registered",
                std::any::type_name::<T>()
            ))
        })
    }

    /// Get a module by name (for dynamic access)
    ///
    /// Use this when you need to access modules dynamically by name.
    /// Note: Type-based access via `MODULE::<T>()` is preferred when possible.
    ///
    /// # Arguments
    /// * `name` - The module name to retrieve
    ///
    /// # Returns
    /// * `Some(Arc<dyn SharedModule>)` if the module exists
    /// * `None` if the module is not found
    ///
    /// # Examples
    /// ```rust,ignore
    /// if let Some(module) = MODULE::get("EmailService") {
    ///     // Use the module dynamically
    /// }
    /// ```
    pub fn get(name: &str) -> Option<Arc<dyn SharedModule>> {
        MODULE_REGISTRY.get()?.get_by_name(name)
    }

    /// Check if a module is registered by type
    ///
    /// # Type Parameters
    /// * `T` - The module type to check
    ///
    /// # Returns
    /// * `true` if the module is registered
    /// * `false` otherwise
    ///
    /// # Examples
    /// ```rust,ignore
    /// if MODULE::exists::<EmailService>() {
    ///     let email = MODULE::<EmailService>()?;
    ///     // Use email service
    /// }
    /// ```
    pub fn exists<T: SharedModule + 'static>() -> bool {
        MODULE_REGISTRY.get().and_then(|r| r.get::<T>()).is_some()
    }

    /// Check if a module is registered by name
    ///
    /// # Arguments
    /// * `name` - The module name to check
    ///
    /// # Returns
    /// * `true` if the module is registered
    /// * `false` otherwise
    ///
    /// # Examples
    /// ```rust,ignore
    /// if MODULE::exists_by_name("EmailService") {
    ///     let service = MODULE::get("EmailService").unwrap();
    /// }
    /// ```
    pub fn exists_by_name(name: &str) -> bool {
        MODULE_REGISTRY
            .get()
            .map(|r| r.contains_name(name))
            .unwrap_or(false)
    }

    /// Check if the MODULE system is initialized
    ///
    /// # Returns
    /// * `true` if MODULE::init() has been called successfully
    /// * `false` otherwise
    ///
    /// # Examples
    /// ```rust,ignore
    /// if MODULE::is_initialized() {
    ///     // Modules are available
    /// }
    /// ```
    pub fn is_initialized() -> bool {
        MODULE_REGISTRY.get().is_some()
    }

    /// List all registered module names
    ///
    /// Useful for debugging or dynamic module discovery.
    ///
    /// # Returns
    /// * Vector of module names and their types
    ///
    /// # Examples
    /// ```rust,ignore
    /// let modules = MODULE::list();
    /// for (name, module_type) in modules {
    ///     println!("Module: {} ({})", name, module_type);
    /// }
    /// ```
    pub fn list() -> Vec<(String, String)> {
        MODULE_REGISTRY
            .get()
            .map(|r| {
                r.list_modules()
                    .into_iter()
                    .map(|(name, module_type)| (name.to_string(), module_type.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the underlying registry (for advanced use cases)
    ///
    /// This provides direct access to the SharedRegistry for scenarios
    /// that need more control or access to registry-specific methods.
    ///
    /// # Returns
    /// * `Some(&Arc<SharedRegistry>)` if initialized
    /// * `None` otherwise
    pub fn get_registry() -> Option<&'static Arc<SharedRegistry>> {
        MODULE_REGISTRY.get()
    }
}
