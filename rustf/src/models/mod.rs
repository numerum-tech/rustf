use crate::Model;
use std::collections::HashMap;
use std::sync::Arc;

pub mod base_model;
pub mod filter;
pub mod model_query;
pub mod query_builder;
/// pub mod macros;

pub struct ModelRegistry {
    models: HashMap<String, Arc<dyn Model>>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    pub fn register<M: Model + 'static>(&mut self, model: M) {
        let name = model.name().to_string();
        self.models.insert(name, Arc::new(model));
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn Model>> {
        self.models.get(name)
    }
}

// Macro for accessing models (Total.js style)
#[macro_export]
macro_rules! MODEL {
    ($name:expr) => {{
        // NOTE: This is a placeholder implementation for the MODEL! macro
        // Future enhancement: Will access the model registry from the current context
        // Currently returns None to maintain API compatibility
        log::warn!(
            "MODEL! macro called with '{}' - this is a stub implementation",
            $name
        );
        None::<std::sync::Arc<dyn $crate::Model>>
    }};
}

// Export for easier access
pub use MODEL;

// Re-export macros for model definitions
/*
 * pub use crate::{
    builder_setter, define_model, implement_crud_methods, implement_field_accessors,
    implement_wrapper,
};
*/

// Re-export base model traits and builders for easier access
pub use base_model::{BaseModel, ChangeTracking, Filter, UpdateBuilder};

// Keep DatabaseModel as alias for backward compatibility during migration
pub use base_model::BaseModel as DatabaseModel;

// Re-export query builder components
pub use query_builder::{
    DatabaseBackend, OrderDirection, QueryBuilder, QueryError, SchemaBuilder, SqlDialect, SqlValue,
};

// Re-export model query builder
pub use model_query::ModelQuery;

// Re-export filter for reusable query filters
pub use filter::ModelFilter;

// Re-export database connection wrapper
pub use query_builder::AnyDatabase;

// NULL constant and FieldUpdate enum for explicit NULL handling
/// A marker type to represent database NULL values explicitly
#[derive(Debug, Clone, Copy)]
pub struct Null;

/// Global constant for setting fields to NULL
pub const NULL: Null = Null;

/// Represents a field update that can be either a value or NULL
#[derive(Debug, Clone)]
pub enum FieldUpdate<T> {
    /// Set the field to a specific value
    Set(T),
    /// Set the field to NULL
    SetNull,
}

// Helper methods for FieldUpdate
impl<T> FieldUpdate<T> {
    /// Create a FieldUpdate with a value
    pub fn value(val: T) -> Self {
        FieldUpdate::Set(val)
    }

    /// Create a FieldUpdate with NULL
    pub fn null() -> Self {
        FieldUpdate::SetNull
    }
}

// Conversion from Option for convenience
impl<T> From<Option<T>> for FieldUpdate<T> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(v) => FieldUpdate::Set(v),
            None => FieldUpdate::SetNull,
        }
    }
}
