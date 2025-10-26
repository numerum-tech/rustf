//! Worker System for RustF
//!
//! Provides a lightweight worker manager with a Total.js-inspired global API.
//! Workers are registered once and can be started on demand from anywhere in
//! the application. Each execution runs to completion and stops automatically,
//! while the messaging channel enables coordination between the worker and the
//! main application.
//!
//! # Example Usage
//!
//! ```rust,ignore
//! // During startup
//! WORKER::register("send-email", |ctx| async move {
//!     ctx.info("Sending email...");
//!     Ok(())
//! }).await?;
//!
//! // Later in the application
//! let mut handle = WORKER::call("send-email", None, Some(json!({"to":"user@example.com"}))).await?;
//! handle.await_result().await?;
//! ```

pub mod api;
pub mod context;
pub mod manager;
pub mod registry;
pub mod types;

// Re-export main types for public API
pub use api::WORKER;
pub use context::WorkerContext;
pub use manager::{WorkerHandle, WorkerManager};
pub use registry::{WorkerRegistry, WORKER_REGISTRY};
pub use types::{WorkerDefinition, WorkerHandler, WorkerId, WorkerStats, WorkerStatus};

use crate::error::Result;
use std::sync::Arc;

/// Initialize the worker system
///
/// This function is called during RustF application startup to initialize
/// the global worker registry and manager.
pub fn initialize() -> Result<()> {
    // This will be called from app.rs with_workers() after the manager is created
    // The actual manager is passed through the app
    Ok(())
}

/// Initialize with a specific manager
pub fn initialize_with_manager(manager: Arc<WorkerManager>) -> Result<()> {
    // Initialize global registry
    registry::initialize_global(manager)?;
    Ok(())
}

/// Shutdown the worker system gracefully
///
/// This function is called during RustF application shutdown to stop all
/// workers and clean up resources.
pub async fn shutdown(manager: Arc<WorkerManager>) -> Result<()> {
    manager.shutdown_all().await?;
    Ok(())
}
