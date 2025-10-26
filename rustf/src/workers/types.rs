//! Core worker types and definitions

use crate::error::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use super::context::WorkerContext;

/// Unique identifier for a worker run
pub type WorkerId = String;

/// Handler function invoked when a worker starts
pub type WorkerHandler = Arc<
    dyn Fn(WorkerContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
        + Send
        + Sync,
>;

/// Definition of a worker discovered at build time or registered manually
#[derive(Clone)]
pub struct WorkerDefinition {
    pub name: String,
    pub handler: WorkerHandler,
}

impl WorkerDefinition {
    /// Create a new worker definition
    pub fn new<F, Fut>(name: impl Into<String>, handler: F) -> Self
    where
        F: Fn(WorkerContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        Self {
            name: name.into(),
            handler: Arc::new(move |ctx| Box::pin(handler(ctx))),
        }
    }
}

/// Lifecycle status for a registered worker
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorkerStatus {
    Idle,
    Running,
    Stopping,
    Stopped,
    Error(String),
}

impl WorkerStatus {
    pub fn is_running(&self) -> bool {
        matches!(self, WorkerStatus::Running)
    }
}

/// Execution statistics aggregated per worker definition
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkerStats {
    pub runs: u64,
    pub errors: u64,
    pub last_run_ms: Option<u64>,
    pub total_runtime_ms: u64,
}

impl WorkerStats {
    pub fn record_success(&mut self, duration_ms: u64) {
        self.runs += 1;
        self.last_run_ms = Some(duration_ms);
        self.total_runtime_ms = self.total_runtime_ms.saturating_add(duration_ms);
    }

    pub fn record_error(&mut self, duration_ms: Option<u64>) {
        self.errors += 1;
        if let Some(ms) = duration_ms {
            self.last_run_ms = Some(ms);
            self.total_runtime_ms = self.total_runtime_ms.saturating_add(ms);
        }
    }
}

/// Payload passed to a worker invocation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkerPayload(pub Value);

