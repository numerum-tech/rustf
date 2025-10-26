//! Worker manager for on-demand worker execution

use crate::config::AppConfig;
use crate::error::{Error, Result};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::task::JoinHandle;

use super::context::WorkerContext;
use super::types::{WorkerDefinition, WorkerHandler, WorkerId, WorkerStats};

#[derive(Clone)]
pub struct WorkerManager {
    inner: Arc<WorkerManagerInner>,
}

struct WorkerManagerInner {
    definitions: RwLock<HashMap<String, WorkerHandler>>,
    stats: RwLock<HashMap<String, WorkerStats>>,
    active_runs: RwLock<HashMap<WorkerId, ActiveRun>>,
    definition_runs: RwLock<HashMap<String, HashSet<WorkerId>>>,
    config: Arc<AppConfig>,
}

struct ActiveRun {
    definition: String,
    handle: JoinHandle<()>,
}

impl WorkerManager {
    pub fn new() -> Result<Self> {
        Self::with_config(Arc::new(AppConfig::default()))
    }

    pub fn with_config(config: Arc<AppConfig>) -> Result<Self> {
        Ok(Self {
            inner: Arc::new(WorkerManagerInner {
                definitions: RwLock::new(HashMap::new()),
                stats: RwLock::new(HashMap::new()),
                active_runs: RwLock::new(HashMap::new()),
                definition_runs: RwLock::new(HashMap::new()),
                config,
            }),
        })
    }

    pub async fn register_definition(&self, definition: WorkerDefinition) -> Result<()> {
        let mut definitions = self.inner.definitions.write().await;
        definitions.insert(definition.name.clone(), definition.handler.clone());
        drop(definitions);

        let mut stats = self.inner.stats.write().await;
        stats.entry(definition.name).or_default();
        Ok(())
    }

    pub async fn definitions(&self) -> Vec<String> {
        self.inner
            .definitions
            .read()
            .await
            .keys()
            .cloned()
            .collect()
    }

    pub async fn call(
        &self,
        worker_name: &str,
        timeout: Option<Duration>,
        payload: Option<Value>,
    ) -> Result<WorkerHandle> {
        let handler = {
            let definitions = self.inner.definitions.read().await;
            definitions
                .get(worker_name)
                .cloned()
                .ok_or_else(|| Error::InvalidInput(format!("Worker '{}' not registered", worker_name)))?
        };

        let run_id = uuid::Uuid::new_v4().to_string();
        let (result_tx, result_rx) = oneshot::channel();
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        let context = WorkerContext::new(
            worker_name.to_string(),
            run_id.clone(),
            Arc::clone(&self.inner.config),
            Some(message_tx.clone()),
        )
        .with_data(payload.unwrap_or(Value::Null));

        let manager = self.clone();
        let name = worker_name.to_string();
        let run_id_clone = run_id.clone();
        let handler_clone = handler.clone();

        let handle = tokio::spawn(async move {
            let started = Instant::now();

            let outcome = match timeout {
                Some(duration) => match tokio::time::timeout(duration, (handler_clone)(context)).await {
                    Ok(result) => result,
                    Err(_) => Err(Error::timeout(format!(
                        "Worker '{}' timed out after {:?}",
                        name, duration
                    ))),
                },
                None => (handler_clone)(context).await,
            };

            let duration_ms = started.elapsed().as_millis() as u64;
            manager
                .finish_run(&name, &run_id_clone, duration_ms, &outcome)
                .await;
            let _ = result_tx.send(outcome);
            drop(message_tx);
        });

        {
            let mut active = self.inner.active_runs.write().await;
            active.insert(
                run_id.clone(),
                ActiveRun {
                    definition: worker_name.to_string(),
                    handle,
                },
            );
        }

        {
            let mut map = self.inner.definition_runs.write().await;
            map.entry(worker_name.to_string())
                .or_default()
                .insert(run_id.clone());
        }

        Ok(WorkerHandle {
            id: run_id,
            name: worker_name.to_string(),
            result: result_rx,
            messages: message_rx,
            manager: self.clone(),
        })
    }

    async fn finish_run(
        &self,
        worker_name: &str,
        run_id: &str,
        duration_ms: u64,
        outcome: &Result<()>,
    ) {
        {
            let mut active = self.inner.active_runs.write().await;
            active.remove(run_id);
        }

        {
            let mut map = self.inner.definition_runs.write().await;
            if let Some(set) = map.get_mut(worker_name) {
                set.remove(run_id);
                if set.is_empty() {
                    map.remove(worker_name);
                }
            }
        }

        let mut stats = self.inner.stats.write().await;
        let entry = stats.entry(worker_name.to_string()).or_default();
        match outcome {
            Ok(()) => entry.record_success(duration_ms),
            Err(_) => entry.record_error(Some(duration_ms)),
        }
    }

    pub async fn cancel(&self, run_id: &str) -> Result<()> {
        let active_run = {
            let mut active = self.inner.active_runs.write().await;
            active.remove(run_id)
        };

        if let Some(active_run) = active_run {
            active_run.handle.abort();
            let mut map = self.inner.definition_runs.write().await;
            if let Some(set) = map.get_mut(&active_run.definition) {
                set.remove(run_id);
            }
            Ok(())
        } else {
            Err(Error::InvalidInput(format!("Worker run '{}' not found", run_id)))
        }
    }

    pub async fn stats(&self, worker_name: &str) -> Option<WorkerStats> {
        self.inner.stats.read().await.get(worker_name).cloned()
    }

    pub async fn running(&self) -> Vec<WorkerId> {
        self.inner
            .active_runs
            .read()
            .await
            .keys()
            .cloned()
            .collect()
    }

    pub async fn running_for(&self, worker_name: &str) -> Vec<WorkerId> {
        self.inner
            .definition_runs
            .read()
            .await
            .get(worker_name)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub async fn shutdown_all(&self) -> Result<()> {
        let mut active = self.inner.active_runs.write().await;
        for (_, run) in active.drain() {
            run.handle.abort();
        }
        self.inner.definition_runs.write().await.clear();
        Ok(())
    }
}

/// Handle returned by `WORKER::call`
pub struct WorkerHandle {
    pub(crate) id: WorkerId,
    pub(crate) name: String,
    pub(crate) result: oneshot::Receiver<Result<()>>,
    pub(crate) messages: mpsc::UnboundedReceiver<Value>,
    pub(crate) manager: WorkerManager,
}

impl WorkerHandle {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn worker_name(&self) -> &str {
        &self.name
    }

    pub async fn cancel(&self) -> Result<()> {
        self.manager.cancel(&self.id).await
    }

    pub async fn recv(&mut self) -> Option<Value> {
        self.messages.recv().await
    }

    pub async fn await_result(self) -> Result<()> {
        match self.result.await {
            Ok(outcome) => outcome,
            Err(_) => Err(Error::internal("Worker run dropped before completion")),
        }
    }
}
