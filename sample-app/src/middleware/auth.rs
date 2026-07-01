//! Authentication gate.
//!
//! Protects the task-management area by path prefix. Any request whose path
//! starts with one of [`PROTECTED_PREFIXES`] must carry an authenticated
//! session (`ctx.require_auth()`); otherwise it is flashed and redirected to
//! `/login` before any controller runs.
//!
//! This is a cross-cutting concern spanning multiple controllers, so it lives
//! as middleware rather than a per-controller `before` hook — it replaces the
//! duplicated auth guards that used to sit in `tasks` and `task_lists`.

use async_trait::async_trait;
use rustf::middleware::{InboundAction, InboundMiddleware, MiddlewareRegistry};
use rustf::prelude::*;

/// Path prefixes that require an authenticated session.
const PROTECTED_PREFIXES: &[&str] = &["/task_lists", "/tasks"];

#[derive(Clone)]
pub struct AuthMiddleware;

#[async_trait]
impl InboundMiddleware for AuthMiddleware {
    async fn process_request(&self, ctx: &mut Context) -> rustf::Result<InboundAction> {
        let protected = PROTECTED_PREFIXES
            .iter()
            .any(|prefix| ctx.path().starts_with(prefix));

        if protected && ctx.require_auth().is_err() {
            ctx.flash_error("Please sign in to manage tasks.")?;
            ctx.redirect("/login")?;
            return Ok(InboundAction::Stop);
        }

        Ok(InboundAction::Continue)
    }

    fn name(&self) -> &'static str {
        "auth"
    }

    fn priority(&self) -> i32 {
        // Run after the built-in session middleware (-500) has populated the
        // session, but ahead of the controllers.
        -100
    }
}

/// Required by auto-discovery.
pub fn install(registry: &mut MiddlewareRegistry) {
    registry.register_inbound("auth", AuthMiddleware);
}
