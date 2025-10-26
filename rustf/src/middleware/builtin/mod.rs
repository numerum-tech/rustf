//! Built-in middleware implementations
//!
//! This module provides common middleware that are frequently needed in web applications.
//! These serve as examples for third-party middleware authors and provide immediate utility.

pub mod cors;
pub mod csp;
pub mod logging;
pub mod rate_limit;
pub mod security_headers;
pub mod session;
pub mod validation;

// Re-export middleware for convenience
pub use cors::{CorsConfig, CorsMiddleware};
pub use csp::{CspConfig, CspMiddleware};
pub use logging::LoggingMiddleware;
pub use rate_limit::RateLimitMiddleware;
pub use security_headers::SecurityHeadersMiddleware;
pub use session::SessionMiddleware;
pub use validation::{ValidationConfig, ValidationMiddleware};

// Note: AuthMiddleware can be created using the session middleware
// or implemented as a custom middleware in your application
