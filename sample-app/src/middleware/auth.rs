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
        // AJAX / htmx responses are partials, so render them without the page
        // layout (chrome). Applies to every XHR request, not just protected
        // ones, so handlers don't each have to remember to strip the layout.
        if ctx.is_xhr() {
            ctx.layout("");
        }

        let protected = PROTECTED_PREFIXES
            .iter()
            .any(|prefix| ctx.path().starts_with(prefix));

        if protected && ctx.require_auth().is_err() {
            ctx.flash_error("Please sign in to manage tasks.")?;

            if ctx.is_xhr() {
                // A 302 to an XHR/htmx request is followed transparently by the
                // browser, so the login page would land inside the partial's
                // target element. Ask the client to do a real, top-level
                // browser navigation instead — htmx honors `HX-Redirect`.
                // Caveat: `HX-Redirect` is htmx-specific; a non-htmx XHR client
                // must read the header and navigate itself.
                ctx.html("")?;
                ctx.add_header("HX-Redirect", "/login");
            } else {
                ctx.redirect("/login")?;
            }

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
