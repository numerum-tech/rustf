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
pub type BeforeFn =
    for<'a> fn(&'a mut Context) -> Pin<Box<dyn Future<Output = Result<BeforeAction>> + Send + 'a>>;

/// Per-route WebSocket settings, set by the `[..]` suffix in a `SOCKET` route.
///
/// ```ignore
/// routes![
///     SOCKET "/chat"  => chat,                        // defaults
///     SOCKET "/feed"  => feed, [json],                // JSON-only
///     SOCKET "/bulk"  => bulk, [json, max = 1 << 20], // and a 1 MiB cap
/// ]
/// ```
///
/// Mirrors Total.js's `ROUTE('SOCKET /chat/', fn, ['json'], 1024)`, with the
/// size given in bytes rather than KiB.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SocketOptions {
    /// Reject any text message that is not valid JSON, closing with `1003`.
    ///
    /// Off by default: messages arrive as they were sent, and a handler that
    /// wants JSON calls `msg.parse::<T>()` itself.
    pub json: bool,
    /// Largest message this route accepts, in bytes. `None` falls back to
    /// `[websocket] max_message_size` from the configuration.
    pub max_message_size: Option<usize>,
}

pub struct Route {
    pub method: String,
    pub path: String,
    pub handler: RouteHandler,
    pub xhr_only: bool,
    /// Optional controller-level pre-handler hook. Wired by the
    /// `routes![before: ..., ...]` macro arm; `None` for routes built
    /// with the bare `routes![...]` arm.
    pub before: Option<BeforeFn>,
    /// `Some` when this is a `SOCKET` route. The route still matches `GET`
    /// (a handshake *is* a GET); this is what tells the dispatcher to answer
    /// a non-upgrade request with `426` instead of running the handler.
    pub socket: Option<SocketOptions>,
}

impl Route {
    pub fn new(method: &str, path: &str, handler: RouteHandler) -> Self {
        Self {
            method: method.to_string(),
            path: path.to_string(),
            handler,
            xhr_only: false,
            before: None,
            socket: None,
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
            socket: None,
        }
    }

    /// A WebSocket route — what `routes![SOCKET "/chat" => chat]` builds.
    ///
    /// Registered as `GET`, because the opening handshake is a GET. What
    /// changes is the dispatch: a `GET` that is not a handshake gets `426
    /// Upgrade Required` and the handler never runs.
    ///
    /// Requires the `websocket` feature (on by default).
    #[cfg(feature = "websocket")]
    pub fn websocket(path: &str, handler: RouteHandler) -> Self {
        Self {
            method: "GET".to_string(),
            path: path.to_string(),
            handler,
            xhr_only: false,
            before: None,
            socket: Some(SocketOptions::default()),
        }
    }

    /// A WebSocket route with explicit settings — what
    /// `routes![SOCKET "/chat" => chat, [json, max = 1024]]` builds.
    #[cfg(feature = "websocket")]
    pub fn websocket_with(path: &str, handler: RouteHandler, socket: SocketOptions) -> Self {
        Self {
            socket: Some(socket),
            ..Self::websocket(path, handler)
        }
    }
}

// Utility macro for creating routes
//
// Implemented as an incremental tt-muncher rather than a flat
// `$(...),*` repetition. That is what makes the Total.js-shaped flag suffix
// possible: an `expr` fragment may only be followed by `=>`, `,` or `;`, so
// `=> $handler [json]` cannot be parsed at all, and `=> $handler, [json]`
// inside a comma-separated repetition is locally ambiguous. Munching one
// route at a time sidesteps both.
//
// Arm order is load-bearing throughout: the `@`-prefixed internal arms must
// precede the two public ones (or `$($rest:tt)*` would swallow them), the
// flagged `SOCKET` arms must precede the unflagged ones, and both must
// precede the catch-all `$method:ident` arm.
#[macro_export]
macro_rules! routes {
    // ---------------------------------------------------------------------
    // Internal: per-route WebSocket flags, `[json, max = 1 << 20]`.
    // ---------------------------------------------------------------------
    (@sockflag $options:ident,) => {};

    (@sockflag $options:ident, json) => {
        $options.json = true;
    };
    (@sockflag $options:ident, json, $($rest:tt)*) => {
        $options.json = true;
        $crate::routes!(@sockflag $options, $($rest)*);
    };

    (@sockflag $options:ident, text) => {
        $options.json = false;
    };
    (@sockflag $options:ident, text, $($rest:tt)*) => {
        $options.json = false;
        $crate::routes!(@sockflag $options, $($rest)*);
    };

    (@sockflag $options:ident, max = $size:expr) => {
        $options.max_message_size = Some($size);
    };
    (@sockflag $options:ident, max = $size:expr, $($rest:tt)*) => {
        $options.max_message_size = Some($size);
        $crate::routes!(@sockflag $options, $($rest)*);
    };

    // ---------------------------------------------------------------------
    // Internal: build one route.
    // ---------------------------------------------------------------------
    (@socket $path:expr, $handler:expr, [$($flags:tt)*]) => {{
        #[allow(unused_mut)]
        let mut __options = $crate::routing::SocketOptions::default();
        $crate::routes!(@sockflag __options, $($flags)*);
        $crate::routing::Route::websocket_with(
            $path,
            |ctx| Box::pin(async { $handler(ctx).await }),
            __options
        )
    }};

    (@xhr $path:expr, $handler:expr) => {
        $crate::routing::Route::xhr(
            $path,
            |ctx| Box::pin(async { $handler(ctx).await })
        )
    };

    (@plain $method:ident, $path:expr, $handler:expr) => {
        $crate::routing::Route::new(
            stringify!($method),
            $path,
            |ctx| Box::pin(async { $handler(ctx).await })
        )
    };

    // ---------------------------------------------------------------------
    // Internal: consume the route list one entry at a time.
    // ---------------------------------------------------------------------
    (@munch $out:ident,) => {};

    // SOCKET with a flag suffix.
    (@munch $out:ident, SOCKET $path:literal => $handler:expr, [$($flags:tt)*], $($rest:tt)*) => {
        $out.push($crate::routes!(@socket $path, $handler, [$($flags)*]));
        $crate::routes!(@munch $out, $($rest)*);
    };
    (@munch $out:ident, SOCKET $path:literal => $handler:expr, [$($flags:tt)*]) => {
        $out.push($crate::routes!(@socket $path, $handler, [$($flags)*]));
    };

    // SOCKET without flags.
    (@munch $out:ident, SOCKET $path:literal => $handler:expr, $($rest:tt)*) => {
        $out.push($crate::routes!(@socket $path, $handler, []));
        $crate::routes!(@munch $out, $($rest)*);
    };
    (@munch $out:ident, SOCKET $path:literal => $handler:expr) => {
        $out.push($crate::routes!(@socket $path, $handler, []));
    };

    // XHR.
    (@munch $out:ident, XHR $path:literal => $handler:expr, $($rest:tt)*) => {
        $out.push($crate::routes!(@xhr $path, $handler));
        $crate::routes!(@munch $out, $($rest)*);
    };
    (@munch $out:ident, XHR $path:literal => $handler:expr) => {
        $out.push($crate::routes!(@xhr $path, $handler));
    };

    // Any other method: GET, POST, PUT, PATCH, DELETE, ...
    (@munch $out:ident, $method:ident $path:literal => $handler:expr, $($rest:tt)*) => {
        $out.push($crate::routes!(@plain $method, $path, $handler));
        $crate::routes!(@munch $out, $($rest)*);
    };
    (@munch $out:ident, $method:ident $path:literal => $handler:expr) => {
        $out.push($crate::routes!(@plain $method, $path, $handler));
    };

    // ---------------------------------------------------------------------
    // Public: with an optional `before:` clause at the top.
    //
    // The `before` expression must resolve to an `async fn(&mut Context)
    // -> rustf::Result<BeforeAction>`. Inner `async fn` items defined inside
    // the calling scope (e.g. inside `install()`) coerce here just like a free
    // function would.
    // ---------------------------------------------------------------------
    (before: $before:expr, $($rest:tt)*) => {{
        // Wrap the user-provided `async fn` in a non-capturing closure
        // that boxes its future, matching the BeforeFn signature.
        let __before_fn: $crate::routing::BeforeFn =
            |ctx| Box::pin(async move { $before(ctx).await });
        #[allow(unused_mut)]
        let mut __routes: Vec<$crate::routing::Route> = Vec::new();
        $crate::routes!(@munch __routes, $($rest)*);
        for __r in &mut __routes {
            __r.before = Some(__before_fn);
        }
        __routes
    }};

    // Public: bare arm, no before hook.
    ($($rest:tt)*) => {{
        #[allow(unused_mut)]
        let mut __routes: Vec<$crate::routing::Route> = Vec::new();
        $crate::routes!(@munch __routes, $($rest)*);
        __routes
    }};
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
