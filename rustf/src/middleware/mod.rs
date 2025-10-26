//! RustF Middleware System
//!
//! This module provides a dual-phase middleware system that separates request
//! processing (inbound) from response processing (outbound), solving Rust's
//! lifetime challenges while maintaining full middleware capabilities.
//!
//! # Design Principles
//! - **Dual-phase architecture**: Separate inbound and outbound processing
//! - **AI-friendly**: Clear, predictable patterns
//! - **Performance-first**: Only process responses when needed
//! - **Type-safe**: Leverages Rust's type system for safety
//!
//! # Example
//!
//! ```rust,ignore
//! use rustf::prelude::*;
//! use rustf::middleware::{InboundMiddleware, OutboundMiddleware, InboundAction};
//!
//! struct TimingMiddleware;
//!
//! impl InboundMiddleware for TimingMiddleware {
//!     fn process_request(&self, ctx: &mut Context) -> Result<InboundAction> {
//!         ctx.set("start_time", Instant::now());
//!         Ok(InboundAction::Capture) // We want to process the response
//!     }
//! }
//!
//! impl OutboundMiddleware for TimingMiddleware {
//!     fn process_response(&self, ctx: &Context, response: &mut Response) -> Result<()> {
//!         let duration = ctx.get::<Instant>("start_time")?.elapsed();
//!         response.add_header("X-Response-Time", format!("{}ms", duration.as_millis()));
//!         Ok(())
//!     }
//! }
//! ```

pub mod builtin;
pub mod traits;

// Re-export the dual-phase traits
pub use traits::MiddlewareInstance as DualPhaseMiddlewareInstance;
pub use traits::{
    DualPhaseMiddleware, InboundAction, InboundMiddleware, MiddlewareBuilder, OutboundMiddleware,
};

// Keep MiddlewareResult for backward compatibility in app.rs
use crate::http::Response;

/// Result of middleware execution (used by app.rs)
pub enum MiddlewareResult {
    /// Continue to the next middleware or route handler
    Continue,

    /// Stop the middleware chain and return the response immediately
    Stop(Response),
}

impl std::fmt::Debug for MiddlewareResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Continue => write!(f, "Continue"),
            Self::Stop(_) => write!(f, "Stop(Response)"),
        }
    }
}

/// Registry for storing and managing dual-phase middleware instances
#[derive(Default)]
pub struct MiddlewareRegistry {
    pub(crate) middleware: Vec<DualPhaseMiddlewareInstance>,
    sorted: bool,
}

impl MiddlewareRegistry {
    /// Create a new middleware registry
    pub fn new() -> Self {
        Self {
            middleware: Vec::new(),
            sorted: true, // Empty is considered sorted
        }
    }

    /// Register an inbound-only middleware
    pub fn register_inbound<M: InboundMiddleware>(&mut self, name: &str, middleware: M) {
        self.middleware
            .push(DualPhaseMiddlewareInstance::inbound(name, middleware));
        self.sorted = false;
    }

    /// Register an outbound-only middleware
    pub fn register_outbound<M: OutboundMiddleware>(&mut self, name: &str, middleware: M) {
        self.middleware
            .push(DualPhaseMiddlewareInstance::outbound(name, middleware));
        self.sorted = false;
    }

    /// Register a dual-phase middleware
    pub fn register_dual<M>(&mut self, name: &str, middleware: M)
    where
        M: InboundMiddleware + OutboundMiddleware + Clone + 'static,
    {
        self.middleware
            .push(DualPhaseMiddlewareInstance::dual(name, middleware));
        self.sorted = false;
    }

    /// Compatibility method for old code - converts to dual phase
    pub fn register<M>(&mut self, name: &str, middleware: M)
    where
        M: InboundMiddleware + OutboundMiddleware + Clone + 'static,
    {
        self.register_dual(name, middleware);
    }

    /// Get all middleware instances sorted by priority
    pub fn get_sorted(&self) -> Vec<&DualPhaseMiddlewareInstance> {
        let mut sorted_refs: Vec<&DualPhaseMiddlewareInstance> = self.middleware.iter().collect();
        sorted_refs.sort_by_key(|m| m.priority);
        sorted_refs
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.middleware.is_empty()
    }

    /// Get count of registered middleware
    pub fn len(&self) -> usize {
        self.middleware.len()
    }
}
