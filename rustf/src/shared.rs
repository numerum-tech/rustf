//! Shared code module system for RustF
//!
//! This module provides a framework for organizing shared code across the application,
//! including both business logic services and utility functions. The system supports
//! explicit registration and provides access to shared modules throughout the application.
//!
//! # Named Module Registration
//!
//! The MODULE system uses named registration, allowing multiple instances of the same
//! type with different configurations:
//!
//! ```rust,ignore
//! // Explicitly register modules with unique names
//! MODULE::init();
//! MODULE::register("email-primary", EmailService::new("primary@example.com"))?;
//! MODULE::register("email-backup", EmailService::new("backup@example.com"))?;
//!
//! // Name-based access
//! let primary = MODULE::get("email-primary")?;
//! let backup = MODULE::get("email-backup")?;
//!
//! // Check if module exists
//! if MODULE::exists("email-primary") {
//!     // Module is available
//! }
//! ```

use crate::error::{Error, Result as RustfResult};
use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
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

/// Thread-safe mutable module registry using DashMap for concurrent access
///
/// The ModuleRegistry provides a mutable, concurrent-friendly registry for shared modules
/// using **named registration**. This allows multiple instances of the same type to be
/// registered with different unique identifiers.
///
/// # Named Registration Pattern
///
/// Modules are registered with unique string identifiers:
/// - Each registration must have a unique name
/// - Multiple instances of the same type can coexist
/// - Type safety is enforced at compilation time (only SharedModule implementers can register)
///
/// # Example
/// ```rust,ignore
/// use rustf::shared::MODULE;
///
/// // Initialize the module system
/// MODULE::init();
///
/// // Register multiple instances of the same type
/// MODULE::register("email-primary", EmailService::new("primary@example.com"))?;
/// MODULE::register("email-backup", EmailService::new("backup@example.com"))?;
///
/// // Access by name
/// let primary = MODULE::get("email-primary")?;
/// let backup = MODULE::get("email-backup")?;
/// ```
pub struct ModuleRegistry {
    // Named registration: String key -> Module instance
    modules: DashMap<String, Arc<dyn SharedModule>>,
}

impl ModuleRegistry {
    /// Create a new empty module registry
    pub fn new() -> Self {
        Self {
            modules: DashMap::new(),
        }
    }

    /// Register a shared module with the registry using a unique name
    ///
    /// This is the primary API for registering modules. Only modules that implement
    /// the SharedModule trait can be registered, enforcing type safety at compile time.
    ///
    /// Multiple instances of the same type can be registered with different names,
    /// allowing for scenarios like multiple email services with different configurations.
    ///
    /// # Type Parameters
    /// * `T` - The module type to register (must implement SharedModule + 'static)
    ///
    /// # Arguments
    /// * `name` - Unique identifier for this module instance
    /// * `module` - The module instance to register
    ///
    /// # Returns
    /// * `Ok(())` if registration succeeded
    /// * `Err` if a module with the same name is already registered
    ///
    /// # Example
    /// ```rust,ignore
    /// MODULE::register("email-service-primary", EmailService::new("primary@example.com"))?;
    /// MODULE::register("email-service-backup", EmailService::new("backup@example.com"))?;
    /// ```
    pub fn register<T: SharedModule + 'static>(&self, name: &str, module: T) -> RustfResult<()> {
        if self.modules.contains_key(name) {
            return Err(Error::internal(format!(
                "Module with name '{}' is already registered",
                name
            )));
        }

        let module = Arc::new(module);
        log::info!("Registering shared {}: {}", module.module_type(), name);

        self.modules
            .insert(name.to_string(), module as Arc<dyn SharedModule>);

        Ok(())
    }

    /// Get a shared module by name
    ///
    /// # Arguments
    /// * `name` - The module name to retrieve
    ///
    /// # Returns
    /// * `Ok(Arc<dyn SharedModule>)` if the module is registered
    /// * `Err` if the module is not found
    pub fn get(&self, name: &str) -> RustfResult<Arc<dyn SharedModule>> {
        self.modules
            .get(name)
            .map(|entry| entry.clone())
            .ok_or_else(|| Error::internal(format!("Module '{}' not found", name)))
    }

    /// Try to get a shared module by name (returns Option)
    ///
    /// # Arguments
    /// * `name` - The module name to retrieve
    ///
    /// # Returns
    /// * `Some(Arc<dyn SharedModule>)` if the module is registered
    /// * `None` if the module is not found
    pub fn get_opt(&self, name: &str) -> Option<Arc<dyn SharedModule>> {
        self.modules.get(name).map(|entry| entry.clone())
    }

    /// Shutdown all registered modules
    ///
    /// This should be called during application shutdown to allow modules
    /// to clean up resources properly.
    ///
    /// # Returns
    /// * `Ok(())` when complete (ignores individual module shutdown errors)
    pub async fn shutdown_all(&self) -> Result<()> {
        for entry in self.modules.iter() {
            let name = entry.key().clone();
            let module = entry.value().clone();
            log::debug!("Shutting down shared module: {}", name);
            if let Err(e) = module.shutdown().await {
                log::warn!("Error shutting down module '{}': {}", name, e);
            }
        }
        Ok(())
    }

    /// List all registered modules
    ///
    /// # Returns
    /// Vector of (module_name, module_type) tuples
    pub fn list_modules(&self) -> Vec<(String, SharedModuleType)> {
        self.modules
            .iter()
            .map(|entry| {
                let name = entry.key().clone();
                let module_type = entry.value().module_type();
                (name, module_type)
            })
            .collect()
    }

    /// Check if a module is registered by name
    ///
    /// # Arguments
    /// * `name` - The module name to check
    ///
    /// # Returns
    /// * `true` if the module is registered
    /// * `false` otherwise
    pub fn contains(&self, name: &str) -> bool {
        self.modules.contains_key(name)
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry for managing shared modules (for backward compatibility with auto-discovery)
///
/// This struct is maintained for backward compatibility with the framework's
/// auto-discovery system. It uses immutable registration during app initialization.
/// For explicit module registration after initialization, use ModuleRegistry and MODULE::register().
#[derive(Default)]
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
    /// This is used during app initialization via auto-discovery.
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
    pub struct TestService {
        pub config: String,
    }
    impl_shared_service!(TestService);

    impl TestService {
        pub fn new(config: String) -> Self {
            Self { config }
        }

        pub async fn do_something(&self) -> Result<String> {
            Ok(format!("Service result: {}", self.config))
        }
    }

    #[tokio::test]
    async fn test_module_registry_named_registration() {
        let registry = ModuleRegistry::new();

        // Register multiple instances of the same type
        let service1 = TestService::new("primary".to_string());
        let service2 = TestService::new("backup".to_string());

        registry.register("service-primary", service1).unwrap();
        registry.register("service-backup", service2).unwrap();

        // Test name-based access
        let primary = registry.get("service-primary").unwrap();
        assert!(registry.contains("service-primary"));
        assert!(registry.contains("service-backup"));

        // Test module listing
        let modules = registry.list_modules();
        assert_eq!(modules.len(), 2);

        // Test shutdown (developers handle initialization explicitly)
        registry.shutdown_all().await.unwrap();
    }

    #[tokio::test]
    async fn test_module_registry_duplicate_name() {
        let registry = ModuleRegistry::new();

        let service = TestService::new("config".to_string());
        registry.register("test-service", service).unwrap();

        // Attempt to register with the same name should fail
        let service2 = TestService::new("config2".to_string());
        let result = registry.register("test-service", service2);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("already registered"));
    }

    #[tokio::test]
    async fn test_module_registry_not_found() {
        let registry = ModuleRegistry::new();

        let result = registry.get("non-existent");
        assert!(result.is_err());

        let opt = registry.get_opt("non-existent");
        assert!(opt.is_none());
    }
}

// ============================================================================
// Global MODULE System - Total.js Style Access
// ============================================================================

/// Global module registry instance (using new ModuleRegistry with named registration)
static MODULE_REGISTRY: OnceCell<ModuleRegistry> = OnceCell::new();

/// Global MODULE accessor for Total.js-style module access with named registration
///
/// Provides global access to registered modules without needing Context or
/// dependency injection. This follows the same pattern as APP and CONF.
///
/// # Named Registration Pattern
///
/// Modules are registered with unique string identifiers, allowing multiple instances
/// of the same type:
///
/// # Examples
/// ```rust,ignore
/// // Initialize the module system
/// MODULE::init();
///
/// // Register modules with unique names
/// MODULE::register("email-primary", EmailService::new("primary@example.com"))?;
/// MODULE::register("email-backup", EmailService::new("backup@example.com"))?;
///
/// // Name-based access
/// let primary = MODULE::get("email-primary")?;
/// let backup = MODULE::get("email-backup")?;
///
/// // Check if module exists
/// if MODULE::exists("email-primary") {
///     // Module is available
/// }
/// ```
pub struct MODULE;

impl MODULE {
    /// Initialize the global module registry
    ///
    /// This should be called once during application startup by the framework.
    /// After initialization, modules can be registered using MODULE::register().
    ///
    /// # Returns
    /// * `Ok(())` if initialization succeeded
    /// * `Err` if the registry was already initialized
    pub fn init() -> RustfResult<()> {
        MODULE_REGISTRY
            .set(ModuleRegistry::new())
            .map_err(|_| Error::internal("MODULE registry has already been initialized"))
    }

    /// Register a shared module with a unique name
    ///
    /// This is the primary API for developers to register modules explicitly.
    /// Only modules that implement SharedModule trait can be registered,
    /// enforcing type safety at compile time.
    ///
    /// Multiple instances of the same type can be registered with different names.
    ///
    /// # Type Parameters
    /// * `T` - The module type to register (must implement SharedModule + 'static)
    ///
    /// # Arguments
    /// * `name` - Unique identifier for this module instance
    /// * `module` - The module instance to register
    ///
    /// # Returns
    /// * `Ok(())` if registration succeeded
    /// * `Err` if not initialized or name already registered
    ///
    /// # Examples
    /// ```rust,ignore
    /// MODULE::register("email-primary", EmailService::new("primary@example.com"))?;
    /// MODULE::register("cache", CacheService::new())?;
    /// ```
    pub fn register<T: SharedModule + 'static>(name: &str, module: T) -> RustfResult<()> {
        let registry = MODULE_REGISTRY.get().ok_or_else(|| {
            Error::internal(
                "MODULE registry not initialized. Call MODULE::init() during app startup",
            )
        })?;

        registry.register(name, module)
    }

    /// Get a shared module by name
    ///
    /// This is the primary way to access registered modules by their unique name.
    /// Returns an Arc<dyn SharedModule> which can be cloned cheaply.
    ///
    /// # Arguments
    /// * `name` - The module name to retrieve
    ///
    /// # Returns
    /// * `Ok(Arc<dyn SharedModule>)` if the module is registered
    /// * `Err` if the module is not found or registry not initialized
    ///
    /// # Examples
    /// ```rust,ignore
    /// let primary = MODULE::get("email-primary")?;
    /// let cache = MODULE::get("cache")?;
    /// ```
    pub fn get(name: &str) -> RustfResult<Arc<dyn SharedModule>> {
        let registry = MODULE_REGISTRY.get().ok_or_else(|| {
            Error::internal(
                "MODULE registry not initialized. Call MODULE::init() during app startup",
            )
        })?;

        registry.get(name)
    }

    /// Try to get a shared module by name (returns Option)
    ///
    /// # Arguments
    /// * `name` - The module name to retrieve
    ///
    /// # Returns
    /// * `Some(Arc<dyn SharedModule>)` if the module is registered
    /// * `None` if the module is not found or registry not initialized
    pub fn get_opt(name: &str) -> Option<Arc<dyn SharedModule>> {
        MODULE_REGISTRY.get()?.get_opt(name)
    }

    /// Check if a module is registered by name
    ///
    /// # Arguments
    /// * `name` - The module name to check
    ///
    /// # Returns
    /// * `true` if the module is registered
    /// * `false` otherwise (or if registry not initialized)
    ///
    /// # Examples
    /// ```rust,ignore
    /// if MODULE::exists("email-primary") {
    ///     let email = MODULE::get("email-primary")?;
    /// }
    /// ```
    pub fn exists(name: &str) -> bool {
        MODULE_REGISTRY
            .get()
            .map(|r| r.contains(name))
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
    /// * Vector of (module_name, module_type) tuples
    /// * Empty vector if registry not initialized
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
                    .map(|(name, module_type)| (name, module_type.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the underlying registry (for advanced use cases)
    ///
    /// This provides direct access to the ModuleRegistry for scenarios
    /// that need more control or access to registry-specific methods.
    ///
    /// # Returns
    /// * `Some(&'static ModuleRegistry)` if initialized
    /// * `None` otherwise
    pub fn get_registry() -> Option<&'static ModuleRegistry> {
        MODULE_REGISTRY.get()
    }
}
