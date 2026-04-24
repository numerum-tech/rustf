//! Dual-phase timing middleware — records the inbound timestamp, then on
//! the outbound phase stamps the response with an `X-Response-Time` header.
//!
//! Canonical example of the `InboundMiddleware` + `OutboundMiddleware` pair
//! pattern on a single type. Returning `InboundAction::Capture` tells the
//! framework to invoke the matching outbound phase on the way out.

use async_trait::async_trait;
use rustf::middleware::{InboundAction, InboundMiddleware, OutboundMiddleware};
use rustf::prelude::*;
use std::time::Instant;

#[derive(Clone, Default)]
pub struct TimingMiddleware;

#[async_trait]
impl InboundMiddleware for TimingMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> rustf::Result<InboundAction> {
        ctx.set("timing_start", Instant::now())?;
        Ok(InboundAction::Capture)
    }

    fn priority(&self) -> i32 {
        // Negative so we start the timer before anything else does real work.
        -100
    }

    fn name(&self) -> &'static str {
        "timing"
    }
}

#[async_trait]
impl OutboundMiddleware for TimingMiddleware {
    async fn process_response(&self, ctx: &mut Context) -> rustf::Result<()> {
        if let Some(start) = ctx.get::<Instant>("timing_start") {
            let elapsed_ms = start.elapsed().as_micros() as f64 / 1000.0;
            ctx.add_header("X-Response-Time", format!("{:.3}ms", elapsed_ms));
        }
        Ok(())
    }
}

pub fn install(registry: &mut rustf::middleware::MiddlewareRegistry) {
    registry.register_dual("timing", TimingMiddleware);
}
