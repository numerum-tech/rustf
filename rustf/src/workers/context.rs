//! Worker execution context

use crate::config::AppConfig;
use crate::error::{Error, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Context provided to worker handlers
#[derive(Clone)]
pub struct WorkerContext {
    worker_name: String,
    run_id: String,
    config: Arc<AppConfig>,
    message_tx: Option<mpsc::UnboundedSender<Value>>,
    state: Arc<RwLock<HashMap<String, Value>>>,
    environment: String,
    data: Option<Value>,
}

impl WorkerContext {
    pub fn new(
        worker_name: String,
        run_id: String,
        config: Arc<AppConfig>,
        message_tx: Option<mpsc::UnboundedSender<Value>>,
    ) -> Self {
        Self {
            worker_name,
            run_id,
            config,
            message_tx,
            state: Arc::new(RwLock::new(HashMap::new())),
            environment: std::env::var("RUSTF_ENV")
                .or_else(|_| std::env::var("NODE_ENV"))
                .unwrap_or_else(|_| "development".to_string()),
            data: None,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn worker_name(&self) -> &str {
        &self.worker_name
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn config(&self) -> &Arc<AppConfig> {
        &self.config
    }

    pub fn payload(&self) -> Option<&Value> {
        self.data.as_ref()
    }

    /// Send a message to the caller (if supported)
    pub fn emit(&self, message: Value) -> Result<()> {
        if let Some(tx) = &self.message_tx {
            tx.send(message)
                .map_err(|_| Error::internal("Failed to emit worker message"))
        } else {
            Ok(())
        }
    }

    pub async fn set_state(&self, key: impl Into<String>, value: Value) -> Result<()> {
        self.state.write().await.insert(key.into(), value);
        Ok(())
    }

    pub async fn get_state(&self, key: &str) -> Option<Value> {
        self.state.read().await.get(key).cloned()
    }

    pub async fn clear_state(&self) -> Result<()> {
        self.state.write().await.clear();
        Ok(())
    }

    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }

    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    pub fn log(&self, level: log::Level, message: &str) {
        log::log!(
            level,
            "[Worker:{} run:{}] {}",
            self.worker_name,
            self.run_id,
            message
        );
    }

    pub fn info(&self, message: &str) {
        self.log(log::Level::Info, message);
    }

    pub fn debug(&self, message: &str) {
        self.log(log::Level::Debug, message);
    }

    pub fn warn(&self, message: &str) {
        self.log(log::Level::Warn, message);
    }

    pub fn error(&self, message: &str) {
        self.log(log::Level::Error, message);
    }

    pub async fn sleep(&self, duration: std::time::Duration) {
        tokio::time::sleep(duration).await;
    }
}
