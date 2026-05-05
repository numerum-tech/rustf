pub mod macros;
pub mod router;
pub mod trie;

use crate::context::Context;
use crate::error::Result;
pub use router::Router;
use std::future::Future;
use std::pin::Pin;

// Type alias for route handlers - modifies Context in place
pub type RouteHandler =
    for<'a> fn(&'a mut Context) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

/// Decision returned by a controller-level `before` hook.
///
/// `Continue` lets the matched route handler run as usual.
/// `Stop` skips the handler entirely; the framework returns whatever
/// response the `before` body has already written to `ctx.res` (typically
/// via `ctx.redirect`, `ctx.throw403`, `ctx.json`, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeforeAction {
    Continue,
    Stop,
}

/// Type alias for controller-level `before` hooks. Same shape as
/// `RouteHandler` but returns `Result<BeforeAction>` so the hook can
/// short-circuit the handler.
pub type BeforeFn = for<'a> fn(
    &'a mut Context,
) -> Pin<Box<dyn Future<Output = Result<BeforeAction>> + Send + 'a>>;

pub struct Route {
    pub method: String,
    pub path: String,
    pub handler: RouteHandler,
    pub xhr_only: bool,
    /// Optional controller-level pre-handler hook. Wired by the
    /// `routes![before: ..., ...]` macro arm; `None` for routes built
    /// with the bare `routes![...]` arm.
    pub before: Option<BeforeFn>,
}

impl Route {
    pub fn new(method: &str, path: &str, handler: RouteHandler) -> Self {
        Self {
            method: method.to_string(),
            path: path.to_string(),
            handler,
            xhr_only: false,
            before: None,
        }
    }

    pub fn get(path: &str, handler: RouteHandler) -> Self {
        Self::new("GET", path, handler)
    }

    pub fn post(path: &str, handler: RouteHandler) -> Self {
        Self::new("POST", path, handler)
    }

    pub fn put(path: &str, handler: RouteHandler) -> Self {
        Self::new("PUT", path, handler)
    }

    pub fn delete(path: &str, handler: RouteHandler) -> Self {
        Self::new("DELETE", path, handler)
    }

    pub fn xhr(path: &str, handler: RouteHandler) -> Self {
        Self {
            method: "XHR".to_string(),
            path: path.to_string(),
            handler,
            xhr_only: true,
            before: None,
        }
    }
}

// Utility macro for creating routes
#[macro_export]
macro_rules! routes {
    // With an optional `before:` clause at the top.
    //
    // The `before:` line is matched first by the parser. If absent, the
    // bare arm below applies — fully backward-compatible.
    //
    // The `before` expression must resolve to an `async fn(&mut Context)
    // -> rustf::Result<BeforeAction>`. Inner `async fn` items defined
    // inside the calling scope (e.g. inside `install()`) coerce here just
    // like a free function would.
    (before: $before:expr, $($method:ident $path:literal => $handler:expr),* $(,)?) => {{
        // Wrap the user-provided `async fn` in a non-capturing closure
        // that boxes its future, matching the BeforeFn signature.
        let __before_fn: $crate::routing::BeforeFn =
            |ctx| Box::pin(async move { $before(ctx).await });
        let mut __routes: Vec<$crate::routing::Route> = vec![
            $(
                routes!(@route $method, $path, $handler)
            ),*
        ];
        for __r in &mut __routes {
            __r.before = Some(__before_fn);
        }
        __routes
    }};

    // Bare arm — no before hook. Identical to the original macro.
    ($($method:ident $path:literal => $handler:expr),* $(,)?) => {
        vec![
            $(
                routes!(@route $method, $path, $handler)
            ),*
        ]
    };

    (@route XHR, $path:expr, $handler:expr) => {
        $crate::routing::Route::xhr(
            $path,
            |ctx| Box::pin(async { $handler(ctx).await })
        )
    };

    (@route $method:ident, $path:expr, $handler:expr) => {
        $crate::routing::Route::new(
            stringify!($method),
            $path,
            |ctx| Box::pin(async { $handler(ctx).await })
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;

    // Stub handler used to populate routes in macro tests.
    async fn dummy(_ctx: &mut Context) -> Result<()> {
        Ok(())
    }

    async fn dummy_before(_ctx: &mut Context) -> Result<BeforeAction> {
        Ok(BeforeAction::Continue)
    }

    #[test]
    fn routes_macro_without_before_leaves_field_none() {
        let routes = routes![
            GET "/" => dummy,
            POST "/items" => dummy,
        ];
        assert_eq!(routes.len(), 2);
        for r in &routes {
            assert!(
                r.before.is_none(),
                "bare routes! arm must leave Route::before as None"
            );
        }
    }

    #[test]
    fn routes_macro_with_before_sets_field_on_every_route() {
        let routes = routes![
            before: dummy_before,
            GET "/" => dummy,
            POST "/items" => dummy,
            DELETE "/items/{id}" => dummy,
        ];
        assert_eq!(routes.len(), 3);
        for r in &routes {
            assert!(
                r.before.is_some(),
                "routes! with `before:` clause must set Route::before on every route"
            );
        }
    }
}
