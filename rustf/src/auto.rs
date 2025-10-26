use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};

use crate::definitions::Definitions;
use crate::events::EventEmitter;
use crate::middleware::MiddlewareRegistry;
use crate::models::ModelRegistry;
use crate::routing::Route;
use crate::shared::SharedRegistry;
use crate::workers::WorkerManager;
use crate::Result;

/// Type alias for async worker installer function
/// Takes a WorkerManager and returns a future that completes when workers are registered
pub type AsyncWorkerInstaller = fn(Arc<WorkerManager>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>;

#[derive(Clone)]
#[derive(Default)]
pub struct AutoDiscoveryHooks {
    pub controllers: Option<fn() -> Vec<Route>>,
    pub models: Option<fn(&mut ModelRegistry)>,
    pub shared: Option<fn(&mut SharedRegistry)>,
    pub middleware: Option<fn(&mut MiddlewareRegistry)>,
    pub events: Option<fn(&mut EventEmitter)>,
    pub definitions: Option<fn(&mut Definitions)>,
    pub workers: Option<AsyncWorkerInstaller>,
}


static AUTO_HOOKS: OnceLock<AutoDiscoveryHooks> = OnceLock::new();

pub fn register_hooks(hooks: AutoDiscoveryHooks) {
    if AUTO_HOOKS.set(hooks).is_err() {
        log::warn!(
            "Auto-discovery hooks have already been registered; ignoring duplicate registration"
        );
    }
}

pub fn hooks() -> Option<&'static AutoDiscoveryHooks> {
    AUTO_HOOKS.get()
}
