//! Worker registry for global access and batch registration

use once_cell::sync::OnceCell;
use std::sync::Arc;

use super::manager::WorkerManager;
use super::types::WorkerDefinition;
use crate::error::{Error, Result};

/// Global worker registry instance
pub static WORKER_REGISTRY: OnceCell<Arc<WorkerRegistry>> = OnceCell::new();

/// Initialize the global worker registry and API
pub fn initialize_global(manager: Arc<WorkerManager>) -> Result<()> {
    super::api::initialize_global_manager(manager.clone())?;
    let registry = Arc::new(WorkerRegistry::new(manager));
    WORKER_REGISTRY
        .set(registry)
        .map_err(|_| Error::internal("Worker registry already initialized"))
}

/// Obtain the global worker registry
pub fn get_registry() -> Result<&'static Arc<WorkerRegistry>> {
    WORKER_REGISTRY
        .get()
        .ok_or_else(|| Error::internal("Worker registry not initialized"))
}

/// Registry facade used for auto-discovery / batch registration
pub struct WorkerRegistry {
    manager: Arc<WorkerManager>,
}

impl WorkerRegistry {
    pub fn new(manager: Arc<WorkerManager>) -> Self {
        Self { manager }
    }

    pub fn manager(&self) -> &Arc<WorkerManager> {
        &self.manager
    }

    /// Register a batch of workers using the provided builder closure
    pub async fn register_from<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(&mut WorkerRegistryBuilder) -> Result<()>,
    {
        let mut builder = WorkerRegistryBuilder::new(self.manager.clone());
        f(&mut builder)?;
        builder.build().await
    }
}

/// Collects worker registrations prior to installing them in the manager
pub struct WorkerRegistryBuilder {
    manager: Arc<WorkerManager>,
    registrations: Vec<WorkerDefinition>,
}

impl WorkerRegistryBuilder {
    fn new(manager: Arc<WorkerManager>) -> Self {
        Self {
            manager,
            registrations: Vec::new(),
        }
    }

    /// Define a worker that can be executed later
    pub fn define<F, Fut>(&mut self, name: impl Into<String>, handler: F) -> &mut Self
    where
        F: Fn(super::context::WorkerContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.registrations
            .push(WorkerDefinition::new(name, handler));
        self
    }

    async fn build(self) -> Result<()> {
        for definition in self.registrations {
            self.manager.register_definition(definition).await?;
        }
        Ok(())
    }
}
