//! Event Emitter System for RustF
//!
//! Provides a Total.js-inspired event system that allows developers to hook into
//! the application lifecycle with async event handlers. This enables decoupled,
//! extensible application initialization and lifecycle management.

pub mod builtin;

use futures::future::join_all;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};

/// Event handler function type
pub type EventHandlerFn = Box<
    dyn Fn(EventContext) -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send>> + Send + Sync,
>;

/// Priority for event handler execution (lower values execute first)
pub type Priority = i32;

/// Unique identifier for event handlers
pub type HandlerId = usize;

/// Configuration for event emitter performance and behavior
#[derive(Debug, Clone)]
pub struct EventEmitterConfig {
    /// Enable parallel execution of handlers within the same priority group
    pub parallel_execution: bool,

    /// Maximum time to wait for handlers to complete (prevents runaway handlers)
    pub handler_timeout: Duration,

    /// Enable debug logging for event execution (can impact performance)
    pub debug_logging: bool,

    /// Maximum number of concurrent handlers per priority group (0 = unlimited)
    pub max_concurrent_handlers: usize,
}

impl Default for EventEmitterConfig {
    fn default() -> Self {
        Self {
            parallel_execution: true,
            handler_timeout: Duration::from_secs(30),
            debug_logging: cfg!(debug_assertions),
            max_concurrent_handlers: 0, // Unlimited
        }
    }
}

impl EventEmitterConfig {
    /// Create a new configuration with parallel execution enabled
    pub fn parallel() -> Self {
        Self {
            parallel_execution: true,
            ..Default::default()
        }
    }

    /// Create a new configuration with sequential execution (for debugging)
    pub fn sequential() -> Self {
        Self {
            parallel_execution: false,
            debug_logging: true,
            ..Default::default()
        }
    }

    /// Set handler timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.handler_timeout = timeout;
        self
    }

    /// Enable or disable debug logging
    pub fn with_debug_logging(mut self, enabled: bool) -> Self {
        self.debug_logging = enabled;
        self
    }

    /// Set maximum concurrent handlers per priority group
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent_handlers = max;
        self
    }
}

/// Standard application lifecycle events
pub mod events {
    /// Core lifecycle events
    pub const READY: &str = "ready"; // Framework fully initialized
    pub const STARTUP: &str = "startup"; // Before server starts listening
    pub const SHUTDOWN: &str = "shutdown"; // Graceful shutdown initiated
    pub const ERROR: &str = "error"; // Application-level errors

    /// Initialization events
    pub const CONFIG_LOADED: &str = "config.loaded"; // Configuration loaded
    pub const DATABASE_READY: &str = "database.ready"; // Database connected
    pub const MODULES_READY: &str = "modules.ready"; // Shared modules initialized
    pub const MIDDLEWARE_READY: &str = "middleware.ready"; // Middleware configured
    pub const ROUTES_READY: &str = "routes.ready"; // Routes registered

    /// Request lifecycle events
    pub const REQUEST_START: &str = "request.start"; // New request received
    pub const REQUEST_END: &str = "request.end"; // Request completed
    pub const REQUEST_ERROR: &str = "request.error"; // Request processing error

    /// Database events
    pub const DB_SEED: &str = "db.seed"; // Database seeding needed
    pub const DB_MIGRATE: &str = "db.migrate"; // Database migration needed
    pub const DB_CONNECTED: &str = "db.connected"; // Database connection established
    pub const DB_DISCONNECTED: &str = "db.disconnected"; // Database connection lost
}

/// Context provided to event handlers
#[derive(Clone)]
pub struct EventContext {
    /// Event name that triggered this handler
    pub event: String,

    /// Optional event data
    pub data: Option<Value>,

    /// Application configuration
    pub config: Arc<crate::config::AppConfig>,

    /// Current environment (development, production, etc.)
    pub environment: String,

    /// Reference to the event emitter (for emitting other events)
    emitter: Option<Arc<RwLock<EventEmitter>>>,
}

impl EventContext {
    /// Create a new event context
    pub fn new(event: String, config: Arc<crate::config::AppConfig>) -> Self {
        let environment = std::env::var("NODE_ENV")
            .or_else(|_| std::env::var("RUST_ENV"))
            .or_else(|_| std::env::var("APP_ENV"))
            .unwrap_or_else(|_| "development".to_string());

        Self {
            event,
            data: None,
            config,
            environment,
            emitter: None,
        }
    }

    /// Set event data
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Set the event emitter reference
    #[allow(dead_code)]
    pub(crate) fn with_emitter(mut self, emitter: Arc<RwLock<EventEmitter>>) -> Self {
        self.emitter = Some(emitter);
        self
    }

    /// Get the current environment
    pub fn env(&self) -> &str {
        &self.environment
    }

    /// Check if running in development
    pub fn is_development(&self) -> bool {
        self.environment == "development" || self.environment == "dev"
    }

    /// Check if running in production
    pub fn is_production(&self) -> bool {
        self.environment == "production" || self.environment == "prod"
    }

    /// Emit another event from within a handler
    pub async fn emit(&self, event: &str, data: Option<Value>) -> crate::Result<()> {
        if let Some(emitter) = &self.emitter {
            let emitter = emitter.read().await;
            emitter
                .emit_internal(event, data, self.config.clone())
                .await
        } else {
            Ok(())
        }
    }
}

/// Event handler registration
struct EventHandler {
    id: HandlerId,
    _priority: Priority,
    handler: EventHandlerFn,
    once: bool,
}

/// Event emitter for managing application lifecycle events
pub struct EventEmitter {
    handlers: HashMap<String, BTreeMap<Priority, Vec<EventHandler>>>,
    next_id: HandlerId,
    config: EventEmitterConfig,
    executed_once: Arc<Mutex<HashSet<HandlerId>>>,
}

impl EventEmitter {
    /// Create a new event emitter with default configuration
    pub fn new() -> Self {
        Self::with_config(EventEmitterConfig::default())
    }

    /// Create a new event emitter with custom configuration
    pub fn with_config(config: EventEmitterConfig) -> Self {
        Self {
            handlers: HashMap::new(),
            next_id: 1,
            config,
            executed_once: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Get the current configuration
    pub fn config(&self) -> &EventEmitterConfig {
        &self.config
    }

    /// Update the configuration
    pub fn set_config(&mut self, config: EventEmitterConfig) {
        self.config = config;
    }

    /// Register an event handler with default priority (0)
    pub fn on<F>(&mut self, event: &str, handler: F) -> HandlerId
    where
        F: Fn(EventContext) -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        self.on_priority(event, 0, handler)
    }

    /// Register an event handler with specific priority
    pub fn on_priority<F>(&mut self, event: &str, priority: Priority, handler: F) -> HandlerId
    where
        F: Fn(EventContext) -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        let id = self.next_id;
        self.next_id += 1;

        let handler = EventHandler {
            id,
            _priority: priority,
            handler: Box::new(handler),
            once: false,
        };

        self.handlers
            .entry(event.to_string())
            .or_default()
            .entry(priority)
            .or_default()
            .push(handler);

        log::debug!(
            "Registered event handler for '{}' with priority {} (id: {})",
            event,
            priority,
            id
        );

        id
    }

    /// Register a one-time event handler
    pub fn once<F>(&mut self, event: &str, handler: F) -> HandlerId
    where
        F: Fn(EventContext) -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        self.once_priority(event, 0, handler)
    }

    /// Register a one-time event handler with priority
    pub fn once_priority<F>(&mut self, event: &str, priority: Priority, handler: F) -> HandlerId
    where
        F: Fn(EventContext) -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        let id = self.next_id;
        self.next_id += 1;

        let handler = EventHandler {
            id,
            _priority: priority,
            handler: Box::new(handler),
            once: true,
        };

        self.handlers
            .entry(event.to_string())
            .or_default()
            .entry(priority)
            .or_default()
            .push(handler);

        log::debug!(
            "Registered one-time event handler for '{}' with priority {} (id: {})",
            event,
            priority,
            id
        );

        id
    }

    /// Remove an event handler by ID
    pub fn off(&mut self, event: &str, handler_id: HandlerId) -> bool {
        if let Some(priorities) = self.handlers.get_mut(event) {
            for handlers in priorities.values_mut() {
                if let Some(pos) = handlers.iter().position(|h| h.id == handler_id) {
                    handlers.remove(pos);
                    log::debug!("Removed event handler {} for '{}'", handler_id, event);
                    return true;
                }
            }
        }
        false
    }

    /// Remove all handlers for an event
    pub fn clear(&mut self, event: &str) {
        if self.handlers.remove(event).is_some() {
            log::debug!("Cleared all handlers for event '{}'", event);
        }
    }

    /// Emit an event and execute all registered handlers
    pub async fn emit(
        &self,
        event: &str,
        data: Option<Value>,
        config: Arc<crate::config::AppConfig>,
    ) -> crate::Result<()> {
        self.emit_internal(event, data, config).await
    }

    /// Internal emit that doesn't require mutable self
    async fn emit_internal(
        &self,
        event: &str,
        data: Option<Value>,
        config: Arc<crate::config::AppConfig>,
    ) -> crate::Result<()> {
        // Fast path: early exit if no handlers registered
        if !self.has_handlers(event) {
            if self.config.debug_logging {
                log::debug!("No handlers registered for event '{}'", event);
            }
            return Ok(());
        }

        if self.config.debug_logging {
            log::info!(
                "Emitting event: '{}' (parallel: {})",
                event,
                self.config.parallel_execution
            );
        }

        let priorities = self
            .handlers
            .get(event)
            .expect("Event should exist due to has_handlers check");
        let mut total_executed = 0;
        let mut all_errors = Vec::new();

        // Execute handlers in priority order (lower priority numbers first)
        for (priority, handlers) in priorities.iter() {
            if handlers.is_empty() {
                continue;
            }

            if self.config.debug_logging {
                log::debug!(
                    "Executing {} handler(s) for '{}' at priority {} (parallel: {})",
                    handlers.len(),
                    event,
                    priority,
                    self.config.parallel_execution
                );
            }

            let (executed, errors) = if self.config.parallel_execution && handlers.len() > 1 {
                // Parallel execution within this priority group
                self.execute_handlers_parallel(handlers, event, &data, config.clone())
                    .await?
            } else {
                // Sequential execution (either forced or single handler)
                self.execute_handlers_sequential(handlers, event, &data, config.clone())
                    .await?
            };

            total_executed += executed;
            all_errors.extend(errors);
        }

        if self.config.debug_logging {
            log::info!(
                "Event '{}' executed {} handler(s) with {} error(s)",
                event,
                total_executed,
                all_errors.len()
            );
        }

        // Return first error if any occurred
        if let Some(err) = all_errors.into_iter().next() {
            return Err(crate::error::Error::internal(err.to_string()));
        }

        Ok(())
    }

    /// Execute handlers in parallel within the same priority group
    async fn execute_handlers_parallel(
        &self,
        handlers: &[EventHandler],
        event: &str,
        data: &Option<Value>,
        config: Arc<crate::config::AppConfig>,
    ) -> crate::Result<(usize, Vec<String>)> {
        // Filter out already-executed once handlers
        let executed_once = self.executed_once.lock().await;
        let handlers_to_execute: Vec<_> = handlers
            .iter()
            .filter(|h| !h.once || !executed_once.contains(&h.id))
            .collect();
        drop(executed_once);

        // Create futures for all handlers
        let handler_futures: Vec<_> = handlers_to_execute
            .iter()
            .enumerate()
            .map(|(idx, handler)| {
                let event_name = event.to_string();
                let data_clone = data.clone();
                let config_clone = config.clone();
                let handler_id = handler.id;
                let handler_once = handler.once;
                let debug_logging = self.config.debug_logging;

                async move {
                    let mut ctx = EventContext::new(event_name, config_clone);
                    if let Some(data) = data_clone {
                        ctx = ctx.with_data(data);
                    }

                    if debug_logging {
                        log::debug!(
                            "Executing handler {} for '{}' (parallel batch)",
                            handler_id,
                            ctx.event
                        );
                    }

                    let start = std::time::Instant::now();
                    let result = (handler.handler)(ctx).await;
                    let duration = start.elapsed();

                    if debug_logging && duration > Duration::from_millis(10) {
                        log::debug!("Handler {} completed in {:?}", handler_id, duration);
                    }

                    (idx, handler_id, handler_once, result)
                }
            })
            .collect();

        // Execute with timeout protection
        let results = tokio::time::timeout(self.config.handler_timeout, join_all(handler_futures))
            .await
            .map_err(|_| {
                crate::error::Error::internal(format!(
                    "Event '{}' handlers timed out after {:?}",
                    event, self.config.handler_timeout
                ))
            })?;

        // Process results
        let mut executed = 0;
        let mut errors = Vec::new();

        for (idx, handler_id, handler_once, result) in results {
            match result {
                Ok(()) => {
                    executed += 1;
                    // Track once handlers
                    if handler_once {
                        let mut executed_once = self.executed_once.lock().await;
                        executed_once.insert(handler_id);
                    }
                }
                Err(e) => {
                    let error_msg = format!(
                        "Error in handler {} (idx {}) for '{}': {}",
                        handler_id, idx, event, e
                    );
                    log::error!("{}", error_msg);
                    errors.push(error_msg);
                }
            }
        }

        Ok((executed, errors))
    }

    /// Execute handlers sequentially (legacy behavior or single handler)
    async fn execute_handlers_sequential(
        &self,
        handlers: &[EventHandler],
        event: &str,
        data: &Option<Value>,
        config: Arc<crate::config::AppConfig>,
    ) -> crate::Result<(usize, Vec<String>)> {
        let mut executed = 0;
        let mut errors = Vec::new();

        for handler in handlers {
            // Skip already-executed once handlers
            if handler.once {
                let executed_once = self.executed_once.lock().await;
                if executed_once.contains(&handler.id) {
                    continue;
                }
            }
            let mut ctx = EventContext::new(event.to_string(), config.clone());
            if let Some(ref data) = data {
                ctx = ctx.with_data(data.clone());
            }

            if self.config.debug_logging {
                log::debug!(
                    "Executing handler {} for '{}' (sequential)",
                    handler.id,
                    event
                );
            }

            let start = std::time::Instant::now();

            // Apply timeout protection for individual handlers
            let result =
                tokio::time::timeout(self.config.handler_timeout, (handler.handler)(ctx)).await;

            match result {
                Ok(Ok(())) => {
                    executed += 1;
                    // Track once handlers
                    if handler.once {
                        let mut executed_once = self.executed_once.lock().await;
                        executed_once.insert(handler.id);
                    }
                    if self.config.debug_logging {
                        let duration = start.elapsed();
                        if duration > Duration::from_millis(10) {
                            log::debug!("Handler {} completed in {:?}", handler.id, duration);
                        }
                    }
                }
                Ok(Err(e)) => {
                    let error_msg =
                        format!("Error in handler {} for '{}': {}", handler.id, event, e);
                    log::error!("{}", error_msg);
                    errors.push(error_msg);
                }
                Err(_) => {
                    let error_msg = format!(
                        "Handler {} for '{}' timed out after {:?}",
                        handler.id, event, self.config.handler_timeout
                    );
                    log::error!("{}", error_msg);
                    errors.push(error_msg);
                }
            }
        }

        Ok((executed, errors))
    }

    /// Check if there are any handlers for an event
    pub fn has_handlers(&self, event: &str) -> bool {
        self.handlers
            .get(event)
            .map(|p| p.values().any(|h| !h.is_empty()))
            .unwrap_or(false)
    }

    /// Get the count of handlers for an event
    pub fn handler_count(&self, event: &str) -> usize {
        self.handlers
            .get(event)
            .map(|p| p.values().map(|h| h.len()).sum())
            .unwrap_or(0)
    }

    /// List all registered events
    pub fn list_events(&self) -> Vec<&str> {
        self.handlers.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function for creating async event handlers
///
/// # Example
/// ```rust,ignore
/// app.on("ready", handler!(async |ctx| {
///     println!("Application ready!");
///     Ok(())
/// }));
/// ```
#[macro_export]
macro_rules! handler {
    ($handler:expr) => {
        |ctx| Box::pin($handler(ctx))
    };
}

// Note: auto_events! macro is now implemented in rustf-macros crate
// and re-exported through lib.rs. The old broken implementation has been removed.

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_emitter() {
        let mut emitter = EventEmitter::new();
        let config = Arc::new(crate::config::AppConfig::default());

        // Test basic event registration and emission
        let counter = Arc::new(RwLock::new(0));
        let counter_clone = counter.clone();

        emitter.on("test", move |_ctx| {
            let counter = counter_clone.clone();
            Box::pin(async move {
                let mut count = counter.write().await;
                *count += 1;
                Ok(())
            })
        });

        emitter.emit("test", None, config.clone()).await.unwrap();
        assert_eq!(*counter.read().await, 1);

        // Test priority ordering
        let order = Arc::new(RwLock::new(Vec::new()));
        let order_clone = order.clone();

        emitter.on_priority("priority", 10, move |_ctx| {
            let order = order_clone.clone();
            Box::pin(async move {
                order.write().await.push("second");
                Ok(())
            })
        });

        let order_clone = order.clone();
        emitter.on_priority("priority", -10, move |_ctx| {
            let order = order_clone.clone();
            Box::pin(async move {
                order.write().await.push("first");
                Ok(())
            })
        });

        emitter
            .emit("priority", None, config.clone())
            .await
            .unwrap();
        let result = order.read().await.clone();
        assert_eq!(result, vec!["first", "second"]);

        // Test once handler
        let once_counter = Arc::new(RwLock::new(0));
        let once_clone = once_counter.clone();

        emitter.once("once_test", move |_ctx| {
            let counter = once_clone.clone();
            Box::pin(async move {
                let mut count = counter.write().await;
                *count += 1;
                Ok(())
            })
        });

        emitter
            .emit("once_test", None, config.clone())
            .await
            .unwrap();
        emitter
            .emit("once_test", None, config.clone())
            .await
            .unwrap();
        // Should only execute once
        assert_eq!(*once_counter.read().await, 1);
    }

    #[tokio::test]
    async fn test_event_context() {
        let config = Arc::new(crate::config::AppConfig::default());
        let ctx = EventContext::new("test".to_string(), config);

        // Test environment detection
        std::env::set_var("NODE_ENV", "production");
        let ctx_prod = EventContext::new(
            "test".to_string(),
            Arc::new(crate::config::AppConfig::default()),
        );
        assert!(ctx_prod.is_production());
        assert!(!ctx_prod.is_development());

        std::env::remove_var("NODE_ENV");
    }
}
