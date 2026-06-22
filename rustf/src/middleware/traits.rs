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

    /// Optional: Get the execution priority (lower numbers execute earlier
    /// in registration order and later in outbound reverse order).
    fn priority(&self) -> i32 {
        0
    }
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
        let priority = middleware.priority();
        Self {
            name: name.to_string(),
            priority,
            inbound: None,
            outbound: Some(Box::new(middleware)),
        }
    }

    /// Create a dual-phase middleware instance
    pub fn dual<M>(name: &str, middleware: M) -> Self
    where
        M: InboundMiddleware + OutboundMiddleware + Clone + 'static,
    {
        let priority = InboundMiddleware::priority(&middleware);
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
    priority: Option<i32>,
}

impl MiddlewareBuilder {
    /// Create a new middleware builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            priority: None,
        }
    }

    /// Set the priority for this middleware
    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Build an inbound-only middleware
    pub fn inbound<M: InboundMiddleware>(self, middleware: M) -> MiddlewareInstance {
        let mut instance = MiddlewareInstance::inbound(&self.name, middleware);
        if let Some(priority) = self.priority {
            instance.priority = priority;
        }
        instance
    }

    /// Build an outbound-only middleware
    pub fn outbound<M: OutboundMiddleware>(self, middleware: M) -> MiddlewareInstance {
        let mut instance = MiddlewareInstance::outbound(&self.name, middleware);
        if let Some(priority) = self.priority {
            instance.priority = priority;
        }
        instance
    }

    /// Build a dual-phase middleware
    pub fn dual<M>(self, middleware: M) -> MiddlewareInstance
    where
        M: InboundMiddleware + OutboundMiddleware + Clone + 'static,
    {
        let mut instance = MiddlewareInstance::dual(&self.name, middleware);
        if let Some(priority) = self.priority {
            instance.priority = priority;
        }
        instance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;
    use crate::views::ViewEngine;
    use std::sync::Arc;

    #[derive(Clone)]
    struct TestDualMiddleware;

    #[async_trait]
    impl InboundMiddleware for TestDualMiddleware {
        async fn process_request(&self, _ctx: &mut Context) -> Result<InboundAction> {
            Ok(InboundAction::Continue)
        }

        fn priority(&self) -> i32 {
            -10
        }
    }

    #[async_trait]
    impl OutboundMiddleware for TestDualMiddleware {
        async fn process_response(&self, _ctx: &mut Context) -> Result<()> {
            Ok(())
        }

        fn priority(&self) -> i32 {
            -10
        }
    }

    #[test]
    fn middleware_builder_priority_overrides_default() {
        let instance = MiddlewareBuilder::new("test")
            .priority(42)
            .dual(TestDualMiddleware);

        assert_eq!(instance.priority, 42);
    }

    #[tokio::test]
    async fn outbound_instance_uses_outbound_priority() {
        #[derive(Clone)]
        struct TestOutbound;

        #[async_trait]
        impl OutboundMiddleware for TestOutbound {
            async fn process_response(&self, _ctx: &mut Context) -> Result<()> {
                Ok(())
            }

            fn priority(&self) -> i32 {
                7
            }
        }

        let instance = MiddlewareInstance::outbound("outbound", TestOutbound);
        assert_eq!(instance.priority, 7);

        let views = Arc::new(ViewEngine::new());
        let mut ctx = Context::new(crate::http::Request::default(), views);
        instance
            .outbound
            .as_ref()
            .unwrap()
            .process_response(&mut ctx)
            .await
            .unwrap();
    }
}
