//! Global WORKER API mirroring Total.js behaviour

use std::sync::Arc;
use std::time::Duration;

use once_cell::sync::OnceCell;
use serde_json::Value;

use super::manager::{WorkerHandle, WorkerManager};
use super::types::{WorkerDefinition, WorkerStats};
use crate::error::{Error, Result};

static GLOBAL_MANAGER: OnceCell<Arc<WorkerManager>> = OnceCell::new();

pub fn initialize_global_manager(manager: Arc<WorkerManager>) -> Result<()> {
    GLOBAL_MANAGER
        .set(manager)
        .map_err(|_| Error::internal("Worker manager already initialised"))
}

fn manager() -> Result<&'static Arc<WorkerManager>> {
    GLOBAL_MANAGER
        .get()
        .ok_or_else(|| Error::internal("Worker manager not initialised"))
}

pub struct WORKER;

impl WORKER {
    /// Register a worker definition that can be invoked later.
    pub async fn register<F, Fut>(name: impl Into<String>, handler: F) -> Result<()>
    where
        F: Fn(super::context::WorkerContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        let definition = WorkerDefinition::new(name, handler);
        manager()?.register_definition(definition).await
    }

    /// Invoke a worker asynchronously and obtain a handle for the running instance.
    pub async fn call(
        worker_name: &str,
        timeout: Option<Duration>,
        payload: Option<Value>,
    ) -> Result<WorkerHandle> {
        manager()?.call(worker_name, timeout, payload).await
    }

    /// Convenience helper that runs the worker and waits for completion.
    pub async fn run(worker_name: &str, payload: Option<Value>) -> Result<()> {
        let handle = WORKER::call(worker_name, None, payload).await?;
        handle.await_result().await
    }

    /// Cancel a running worker by run identifier.
    pub async fn cancel(run_id: &str) -> Result<()> {
        manager()?.cancel(run_id).await
    }

    /// List registered worker names.
    pub async fn definitions() -> Result<Vec<String>> {
        Ok(manager()?.definitions().await)
    }

    /// List currently running worker run identifiers.
    pub async fn running() -> Result<Vec<String>> {
        Ok(manager()?.running().await)
    }

    /// List run ids for the given worker name.
    pub async fn running_for(worker_name: &str) -> Result<Vec<String>> {
        Ok(manager()?.running_for(worker_name).await)
    }

    /// Retrieve aggregated statistics for a worker definition.
    pub async fn stats(worker_name: &str) -> Option<WorkerStats> {
        match manager() {
            Ok(manager) => manager.stats(worker_name).await,
            Err(_) => None,
        }
    }

    /// Suspend all running workers.
    pub async fn shutdown() -> Result<()> {
        manager()?.shutdown_all().await
    }
}
