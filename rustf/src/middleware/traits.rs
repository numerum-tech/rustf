//! Inbound/Outbound Middleware Pattern for RustF
//!
//! This module implements a two-phase middleware pattern that separates
//! request processing (inbound) from response processing (outbound).
//! This approach solves Rust lifetime issues while providing full middleware capabilities.

use crate::context::Context;
use crate::error::Result;
use async_trait::async_trait;
use std::fmt::Debug;

/// Action to take after processing an inbound middleware
#[derive(Debug, Clone)]
pub enum InboundAction {
    /// Continue to the next middleware in the chain
    Continue,

    /// Stop the chain and use the response set on context
    Stop,

    /// Continue processing and ensure this middleware processes the response
    Capture,
}

/// Trait for middleware that processes incoming requests
///
/// Inbound middleware runs before the route handler and can:
/// - Modify the request context
/// - Short-circuit with an early response
/// - Register for outbound processing
#[async_trait]
pub trait InboundMiddleware: Send + Sync + 'static {
    /// Process an incoming request
    ///
    /// # Returns
    /// - `Continue`: Pass to next middleware without outbound processing
    /// - `Stop(response)`: Return response immediately, skip remaining chain
    /// - `Capture`: Continue and guarantee outbound processing
    async fn process_request(&self, ctx: &mut Context) -> Result<InboundAction>;

    /// Optional: Get the name of this middleware for debugging
    fn name(&self) -> &'static str {
        "unnamed"
    }

    /// Optional: Get the execution priority (lower numbers execute first)
    fn priority(&self) -> i32 {
        0
    }

    /// Optional: Check if this middleware should run for the given context
    fn should_run(&self, _ctx: &Context) -> bool {
        true
    }
}

/// Trait for middleware that processes outgoing responses
///
/// Outbound middleware runs after the route handler and can:
/// - Modify response headers and body via ctx.res
/// - Add cookies, compression, etc.
/// - Log response metrics
/// - Access session and request data from context
#[async_trait]
pub trait OutboundMiddleware: Send + Sync + 'static {
    /// Process an outgoing response
    ///
    /// Called in reverse order of inbound processing.
    /// The response can be accessed and modified via ctx.res.
    async fn process_response(&self, ctx: &mut Context) -> Result<()>;
}

/// Combined middleware that implements both phases
///
/// Many middleware need both phases (e.g., timing, session management).
/// This trait is automatically implemented for types that implement both
/// InboundMiddleware and OutboundMiddleware.
pub trait DualPhaseMiddleware: InboundMiddleware + OutboundMiddleware {
    /// Indicates this middleware handles both phases
    fn is_dual_phase(&self) -> bool {
        true
    }
}

// Automatic implementation for types that implement both traits
impl<T> DualPhaseMiddleware for T
where
    T: InboundMiddleware + OutboundMiddleware,
{
    fn is_dual_phase(&self) -> bool {
        true
    }
}

/// Container for a middleware instance with phase information
pub struct MiddlewareInstance {
    pub name: String,
    pub priority: i32,
    pub inbound: Option<Box<dyn InboundMiddleware>>,
    pub outbound: Option<Box<dyn OutboundMiddleware>>,
}

impl MiddlewareInstance {
    /// Create an inbound-only middleware instance
    pub fn inbound<M: InboundMiddleware>(name: &str, middleware: M) -> Self {
        let priority = middleware.priority();
        Self {
            name: name.to_string(),
            priority,
            inbound: Some(Box::new(middleware)),
            outbound: None,
        }
    }

    /// Create an outbound-only middleware instance
    pub fn outbound<M: OutboundMiddleware>(name: &str, middleware: M) -> Self {
        Self {
            name: name.to_string(),
            priority: 0,
            inbound: None,
            outbound: Some(Box::new(middleware)),
        }
    }

    /// Create a dual-phase middleware instance
    pub fn dual<M>(name: &str, middleware: M) -> Self
    where
        M: InboundMiddleware + OutboundMiddleware + Clone + 'static,
    {
        let priority = middleware.priority();
        Self {
            name: name.to_string(),
            priority,
            inbound: Some(Box::new(middleware.clone())),
            outbound: Some(Box::new(middleware)),
        }
    }

    /// Check if this middleware has an inbound phase
    pub fn has_inbound(&self) -> bool {
        self.inbound.is_some()
    }

    /// Check if this middleware has an outbound phase
    pub fn has_outbound(&self) -> bool {
        self.outbound.is_some()
    }
}

/// Builder for creating middleware with fluent API
pub struct MiddlewareBuilder {
    name: String,
    priority: i32,
}

impl MiddlewareBuilder {
    /// Create a new middleware builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            priority: 0,
        }
    }

    /// Set the priority for this middleware
    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Build an inbound-only middleware
    pub fn inbound<M: InboundMiddleware>(self, middleware: M) -> MiddlewareInstance {
        MiddlewareInstance::inbound(&self.name, middleware)
    }

    /// Build an outbound-only middleware
    pub fn outbound<M: OutboundMiddleware>(self, middleware: M) -> MiddlewareInstance {
        MiddlewareInstance::outbound(&self.name, middleware)
    }

    /// Build a dual-phase middleware
    pub fn dual<M>(self, middleware: M) -> MiddlewareInstance
    where
        M: InboundMiddleware + OutboundMiddleware + Clone + 'static,
    {
        MiddlewareInstance::dual(&self.name, middleware)
    }
}
