//! Error context and chaining utilities
//!
//! Provides tools for building error chains with context information,
//! making it easier to trace errors through the application stack.

use super::Error;
use std::fmt;

/// Trait for adding context to errors
pub trait ErrorContext<T> {
    /// Add context to the error
    fn context<C>(self, context: C) -> Result<T, Error>
    where
        C: Into<String>;

    /// Add context with lazy evaluation
    fn with_context<C, F>(self, f: F) -> Result<T, Error>
    where
        C: Into<String>,
        F: FnOnce() -> C;
}

impl<T> ErrorContext<T> for Result<T, Error> {
    fn context<C>(self, context: C) -> Result<T, Error>
    where
        C: Into<String>,
    {
        self.map_err(|e| e.with_context(context))
    }

    fn with_context<C, F>(self, f: F) -> Result<T, Error>
    where
        C: Into<String>,
        F: FnOnce() -> C,
    {
        self.map_err(|e| e.with_context(f()))
    }
}

/// Helper for building error chains
pub struct ErrorChain<'a> {
    error: &'a Error,
    chain: Vec<String>,
}

impl<'a> ErrorChain<'a> {
    /// Create a new error chain
    pub fn new(error: &'a Error) -> Self {
        let mut chain = Vec::new();
        Self::build_chain(error, &mut chain);
        Self { error, chain }
    }

    fn build_chain(error: &Error, chain: &mut Vec<String>) {
        chain.push(error.to_string());

        // If this is a context error, recurse to get the source
        if let Error::WithContext { source, .. } = error {
            Self::build_chain(source, chain);
        }
    }

    /// Get the full error chain as a vector
    pub fn chain(&self) -> &[String] {
        &self.chain
    }

    /// Get the root cause of the error
    pub fn root_cause(&self) -> &Error {
        let mut current = self.error;
        while let Error::WithContext { source, .. } = current {
            current = source;
        }
        current
    }

    /// Format the error chain for logging
    pub fn format_for_log(&self) -> String {
        self.chain.join(" -> ")
    }

    /// Format the error chain for display
    pub fn format_for_display(&self) -> String {
        if self.chain.len() == 1 {
            self.chain[0].clone()
        } else {
            format!(
                "{}\n\nCaused by:\n{}",
                self.chain[0],
                self.chain[1..]
                    .iter()
                    .enumerate()
                    .map(|(i, msg)| format!("  {}. {}", i + 1, msg))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }
    }
}

impl<'a> fmt::Display for ErrorChain<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_for_display())
    }
}

/// Extension trait for Option types
pub trait OptionExt<T> {
    /// Convert None to an error with context
    fn context<C>(self, context: C) -> Result<T, Error>
    where
        C: Into<String>;

    /// Convert None to an error with lazy context
    fn with_context<C, F>(self, f: F) -> Result<T, Error>
    where
        C: Into<String>,
        F: FnOnce() -> C;
}

impl<T> OptionExt<T> for Option<T> {
    fn context<C>(self, context: C) -> Result<T, Error>
    where
        C: Into<String>,
    {
        self.ok_or_else(|| Error::internal(context))
    }

    fn with_context<C, F>(self, f: F) -> Result<T, Error>
    where
        C: Into<String>,
        F: FnOnce() -> C,
    {
        self.ok_or_else(|| Error::internal(f()))
    }
}

/// Macro for easy error context chaining
#[macro_export]
macro_rules! context {
    ($result:expr, $($arg:tt)*) => {
        $result.context(format!($($arg)*))
    };
}

/// Macro for creating errors with context
#[macro_export]
macro_rules! error_with_context {
    ($error:expr, $($arg:tt)*) => {
        $error.with_context(format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context_chaining() {
        let base_error = Error::database_connection("Connection refused");
        let with_context = base_error
            .with_context("Failed to connect to database")
            .with_context("Cannot initialize application");

        if let Error::WithContext { message, source } = with_context {
            assert_eq!(message, "Cannot initialize application");
            if let Error::WithContext { message, source } = source.as_ref() {
                assert_eq!(message, "Failed to connect to database");
                assert!(matches!(source.as_ref(), Error::DatabaseConnection(_)));
            }
        } else {
            panic!("Expected WithContext error");
        }
    }

    #[test]
    fn test_error_chain_formatting() {
        let error = Error::database_connection("Connection refused")
            .with_context("Failed to connect to database")
            .with_context("Cannot initialize application");

        let chain = ErrorChain::new(&error);

        assert_eq!(chain.chain().len(), 3);
        assert!(chain.format_for_log().contains("->"));

        let root = chain.root_cause();
        assert!(matches!(root, Error::DatabaseConnection(_)));
    }

    #[test]
    fn test_option_context() {
        let none_value: Option<i32> = None;
        let result = none_value.context("Value was missing");

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, Error::Internal(_)));
            assert_eq!(e.to_string(), "Internal error: Value was missing");
        }
    }

    #[test]
    fn test_result_context() {
        let error = Error::validation("Invalid input");
        let result: Result<(), Error> = Err(error);

        let with_context = result.context("Failed to process request");

        assert!(with_context.is_err());
        if let Err(e) = with_context {
            assert!(matches!(e, Error::WithContext { .. }));

            let chain = ErrorChain::new(&e);
            assert_eq!(chain.chain().len(), 2);
        }
    }
}
