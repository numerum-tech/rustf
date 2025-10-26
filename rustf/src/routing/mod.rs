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

pub struct Route {
    pub method: String,
    pub path: String,
    pub handler: RouteHandler,
    pub xhr_only: bool,
}

impl Route {
    pub fn new(method: &str, path: &str, handler: RouteHandler) -> Self {
        Self {
            method: method.to_string(),
            path: path.to_string(),
            handler,
            xhr_only: false,
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
        }
    }
}

// Utility macro for creating routes
#[macro_export]
macro_rules! routes {
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
