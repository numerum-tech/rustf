//! RustF - A simple MVC web framework inspired by Total.js
//!
//! RustF provides a convention-based MVC framework for Rust with:
//! - Total.js-style controller routing
//! - Simple template engine
//! - Session and flash message support
//! - Model loading system

// Enforce error handling best practices
#![cfg_attr(
    not(test),
    warn(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unimplemented,
        clippy::todo,
    )
)]
// Allow in tests
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used,))]

pub mod app;
pub mod auto;
pub mod cache;
pub mod config;
pub mod configuration;
pub mod context;
pub mod definitions;
pub mod error;
pub mod events;
pub mod forms;
pub mod http;
pub mod middleware;
pub mod models;
pub mod pool;
pub mod repository;
pub mod routing;
pub mod security;
pub mod session;
pub mod shared;
pub mod utils;
pub mod views;
pub mod workers;

// CLI support for command-line argument parsing
pub mod cli;

// Database module for multi-database support
pub mod database;

// Global database access (backward compatible)
pub mod db;

// Migration system for database schema management
pub mod migrations;

// Schema support through rustf-schema crate
#[cfg(feature = "schema")]
pub use rustf_schema as schema;

// Re-export main types for public API
pub use app::RustF;
pub use config::AppConfig;
pub use context::Context;
pub use error::{Error, Result};
pub use http::{Request, Response};
pub use middleware::{InboundAction, InboundMiddleware, MiddlewareResult, OutboundMiddleware};
pub use routing::{Route, RouteHandler};

// Re-export database access for backward compatibility
pub use configuration::CONF;
pub use db::DB;
pub use events::{builtin, EventContext, EventEmitter};
pub use pool::{global_request_pool, PooledRequest, RequestPool};
pub use repository::{APP, MAIN};
pub use security::{
    CsrfConfig, CsrfMiddleware, HtmlEscaper, InputValidator, PathValidator, SecurityConfig,
};
pub use session::factory::SessionStorageFactory;
#[cfg(feature = "redis")]
pub use session::redis::RedisSessionStorage;
pub use session::{Session, SessionData, SessionStorage, SessionStore, StorageStats};
pub use shared::{SharedModule, SharedModuleType, SharedRegistry, MODULE};
pub use utils::{Utils, U};

// Re-export database functions
pub use db::database_status;

// Re-export migration system
pub use migrations::{Migration, MigrationDirection, MigrationManager};

// Re-export worker system
pub use workers::{WorkerContext, WorkerManager, WORKER};

// Re-export view system
pub use views::VIEW;

#[cfg(feature = "schema")]
pub use rustf_schema::{Field, Schema, SchemaError, Table};

// Re-export commonly used external types
pub use serde::{Deserialize, Serialize};
pub use serde_json::{json, Value};

// Re-export auto-discovery macros unconditionally
pub use rustf_macros::{
    auto_controllers, auto_definitions, auto_discover, auto_events, auto_middleware, auto_models,
    auto_modules, auto_workers,
};

/// Prelude module for common imports
pub mod prelude {
    pub use crate::*;
    pub use serde::{Deserialize, Serialize};
    pub use serde_json::json;
    pub use std::collections::HashMap;

    // Global utilities for Total.js-style development
    pub use crate::configuration::CONF;
    pub use crate::repository::{APP, MAIN};
    pub use crate::shared::MODULE;
    pub use crate::utils::{Utils, U};
    pub use crate::views::VIEW;
    pub use crate::workers::WORKER;

    // Pool utilities for high-performance applications
    pub use crate::pool::{global_request_pool, PooledRequest, RequestPool};

    // Shared code system
    pub use crate::shared::{SharedModule, SharedModuleType, SharedRegistry};
    pub use crate::{impl_shared_helper, impl_shared_service, impl_shared_util};

    // Event system
    pub use crate::events::{builtin, EventContext, EventEmitter};

    // Database model traits for generated models
    pub use crate::models::{DatabaseModel, ModelQuery, OrderDirection, SqlValue};

    // Global database access
    pub use crate::db::DB;

    // Re-export auto-discovery macros for convenience
    pub use rustf_macros::{
        auto_controllers, auto_definitions, auto_discover, auto_events, auto_middleware,
        auto_models, auto_modules, auto_workers,
    };
}

// Framework traits
pub trait Model: Send + Sync {
    fn name(&self) -> &'static str;
}
