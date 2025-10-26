pub mod ast;
pub mod engine;
/// Total.js template engine implementation for RustF
///
/// This module provides a full-featured Total.js-compatible template engine
/// with support for all major Total.js template features including:
/// - Variable interpolation with dot notation
/// - Control flow (if/else, foreach loops)
/// - Sections and layouts
/// - Helper functions
/// - Localization
/// - Import directives
/// - CSRF and security features
pub mod lexer;
pub mod parser;
pub mod renderer;
pub mod resource_translation;
pub mod translation;

#[cfg(feature = "embedded-views")]
pub mod embedded;

pub use engine::TotalJsEngine;

#[cfg(feature = "embedded-views")]
pub use embedded::EmbeddedTotalJsEngine;

// Re-export commonly used types
pub use ast::{Node, Template};
pub use lexer::{Token, TokenKind};
