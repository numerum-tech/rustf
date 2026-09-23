use thiserror::Error;

pub mod context;
pub mod logging;
pub mod pages;
pub mod retry;

pub type Result<T> = std::result::Result<T, Error>;

// Re-export error page types for easy access
pub use pages::{CheckStatus, ErrorPages, HealthCheck, HealthCheckResult};

// Re-export logging types for easy access
pub use logging::{ErrorLogger, LogConfig, LogEntry, LogLevel, LogOutput, RequestContext};

// Re-export retry logic
pub use retry::{with_retry, RetryPolicy, RetryableError};

// Re-export context helpers
pub use context::{ErrorChain, ErrorContext};

/// Main error type for RustF framework
#[derive(Error, Debug)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] hyper::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Template error: {0}")]
    Template(String),

    #[error("Route not found: {0}")]
    RouteNotFound(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),

    // Database-specific errors
    #[error("Database connection error: {0}")]
    DatabaseConnection(String),

    #[error("Database query error: {0}")]
    DatabaseQuery(String),

    #[error("Database transaction error: {0}")]
    DatabaseTransaction(String),

    #[error("Database migration error: {0}")]
    DatabaseMigration(String),

    #[error("Database pool error: {0}")]
    DatabasePool(String),

    // Network and external service errors
    #[error("Network error: {0}")]
    Network(String),

    #[error("External service error: {service}: {message}")]
    ExternalService { service: String, message: String },

    #[error("Timeout error: {0}")]
    Timeout(String),

    // Authentication and authorization
    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    #[cfg(feature = "redis")]
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[cfg(feature = "redis")]
    #[error("Redis pool error: {0}")]
    RedisPool(String),

    /// A WebSocket connection failed: a protocol violation, a closed socket,
    /// or an oversized frame. Carries a message rather than the tungstenite
    /// error so that the dependency stays out of RustF's public API.
    #[cfg(feature = "websocket")]
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    // Error with context chain
    #[error("{message}")]
    WithContext {
        message: String,
        #[source]
        source: Box<Error>,
    },
}

// Redis-specific error conversions
#[cfg(feature = "redis")]
impl From<deadpool_redis::ConfigError> for Error {
    fn from(err: deadpool_redis::ConfigError) -> Self {
        Self::RedisPool(format!("Config error: {}", err))
    }
}

#[cfg(feature = "redis")]
impl From<deadpool_redis::CreatePoolError> for Error {
    fn from(err: deadpool_redis::CreatePoolError) -> Self {
        Self::RedisPool(format!("Pool creation error: {}", err))
    }
}

#[cfg(feature = "redis")]
impl From<deadpool_redis::PoolError> for Error {
    fn from(err: deadpool_redis::PoolError) -> Self {
        Self::RedisPool(format!("Pool error: {}", err))
    }
}

impl Error {
    pub fn template(msg: impl Into<String>) -> Self {
        Self::Template(msg.into())
    }

    pub fn session(msg: impl Into<String>) -> Self {
        Self::Session(msg.into())
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    // Database error constructors
    pub fn database_connection(msg: impl Into<String>) -> Self {
        Self::DatabaseConnection(msg.into())
    }

    pub fn database_query(msg: impl Into<String>) -> Self {
        Self::DatabaseQuery(msg.into())
    }

    pub fn database_transaction(msg: impl Into<String>) -> Self {
        Self::DatabaseTransaction(msg.into())
    }

    pub fn database_migration(msg: impl Into<String>) -> Self {
        Self::DatabaseMigration(msg.into())
    }

    pub fn database_pool(msg: impl Into<String>) -> Self {
        Self::DatabasePool(msg.into())
    }

    // Network error constructors
    pub fn network(msg: impl Into<String>) -> Self {
        Self::Network(msg.into())
    }

    pub fn external_service(service: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ExternalService {
            service: service.into(),
            message: message.into(),
        }
    }

    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::Timeout(msg.into())
    }

    // Auth error constructors
    pub fn authentication(msg: impl Into<String>) -> Self {
        Self::Authentication(msg.into())
    }

    pub fn authorization(msg: impl Into<String>) -> Self {
        Self::Authorization(msg.into())
    }

    pub fn rate_limit(msg: impl Into<String>) -> Self {
        Self::RateLimit(msg.into())
    }

    // Add context to an error
    pub fn with_context(self, context: impl Into<String>) -> Self {
        Self::WithContext {
            message: context.into(),
            source: Box::new(self),
        }
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Network(_)
                | Error::DatabaseConnection(_)
                | Error::Timeout(_)
                | Error::ExternalService { .. }
                | Error::DatabasePool(_)
        )
    }

    /// Get error code for API responses
    pub fn error_code(&self) -> &'static str {
        match self {
            Error::Http(_) => "E_HTTP",
            Error::Json(_) => "E_JSON",
            Error::Template(_) => "E_TEMPLATE",
            Error::RouteNotFound(_) => "E_ROUTE_NOT_FOUND",
            Error::ModelNotFound(_) => "E_MODEL_NOT_FOUND",
            Error::Session(_) => "E_SESSION",
            Error::Validation(_) => "E_VALIDATION",
            Error::InvalidInput(_) => "E_INVALID_INPUT",
            Error::Io(_) => "E_IO",
            Error::Internal(_) => "E_INTERNAL",
            Error::DatabaseConnection(_) => "E_DB_CONNECTION",
            Error::DatabaseQuery(_) => "E_DB_QUERY",
            Error::DatabaseTransaction(_) => "E_DB_TRANSACTION",
            Error::DatabaseMigration(_) => "E_DB_MIGRATION",
            Error::DatabasePool(_) => "E_DB_POOL",
            Error::Network(_) => "E_NETWORK",
            Error::ExternalService { .. } => "E_EXTERNAL_SERVICE",
            Error::Timeout(_) => "E_TIMEOUT",
            Error::Authentication(_) => "E_AUTH",
            Error::Authorization(_) => "E_AUTHZ",
            Error::RateLimit(_) => "E_RATE_LIMIT",
            #[cfg(feature = "redis")]
            Error::Redis(_) => "E_REDIS",
            #[cfg(feature = "redis")]
            Error::RedisPool(_) => "E_REDIS_POOL",
            #[cfg(feature = "websocket")]
            Error::WebSocket(_) => "E_WEBSOCKET",
            Error::WithContext { source, .. } => source.error_code(),
        }
    }

    /// Get HTTP status code for the error
    pub fn status_code(&self) -> u16 {
        match self {
            Error::Validation(_) | Error::InvalidInput(_) => 400,
            Error::Authentication(_) => 401,
            Error::Authorization(_) => 403,
            Error::RouteNotFound(_) | Error::ModelNotFound(_) => 404,
            Error::RateLimit(_) => 429,
            Error::Timeout(_) => 408,
            Error::WithContext { source, .. } => source.status_code(),
            _ => 500,
        }
    }

    /// The text this error may safely show the client.
    ///
    /// A 4xx describes what the *caller* got wrong, so its message is part of
    /// the answer. Everything else collapses to the canonical reason phrase:
    /// a 5xx message names internals (a connection string, a file path, a
    /// driver's complaint) and must not leave the process, and even at 4xx a
    /// `Timeout`, `RateLimit` or `ModelNotFound` message describes the server
    /// rather than the request. The full chain goes to the log instead.
    pub fn client_message(&self) -> &str {
        match self {
            Error::Validation(msg)
            | Error::InvalidInput(msg)
            | Error::Authentication(msg)
            | Error::Authorization(msg) => msg,
            // Delegate rather than return the context string: `with_context`
            // is where callers record *internal* detail ("while loading the
            // billing profile from the replica"), so only the wrapped error
            // decides what the client sees.
            Error::WithContext { source, .. } => source.client_message(),
            _ => reason_phrase(self.status_code()),
        }
    }
}

/// The canonical reason phrase for the statuses [`Error::status_code`] can
/// produce. Used as the response body wherever the error's own message is not
/// safe to disclose.
fn reason_phrase(status: u16) -> &'static str {
    match status {
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        408 => "Request Timeout",
        429 => "Too Many Requests",
        _ => "Internal Server Error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_message_discloses_only_what_the_caller_caused() {
        // A 4xx describes the request, so the message is the answer.
        assert_eq!(
            Error::validation("email is not a valid address").client_message(),
            "email is not a valid address"
        );
        assert_eq!(
            Error::authentication("Session expired").client_message(),
            "Session expired"
        );
        assert_eq!(
            Error::authorization("administrators only").client_message(),
            "administrators only"
        );

        // A 5xx describes the server and must not leave the process.
        let internal = Error::internal("postgres://admin:hunter2@10.0.0.7/billing is down");
        assert_eq!(internal.status_code(), 500);
        assert_eq!(internal.client_message(), "Internal Server Error");

        // 4xx variants whose message still names internals fall back to the
        // reason phrase.
        assert_eq!(
            Error::ModelNotFound("internal_billing_ledger".into()).client_message(),
            "Not Found"
        );
        assert_eq!(
            Error::rate_limit("10.0.0.7 exceeded 100/min").client_message(),
            "Too Many Requests"
        );
        assert_eq!(
            Error::Timeout("replica-3 did not answer in 30s".into()).client_message(),
            "Request Timeout"
        );

        // `with_context` records internal detail; the wrapped error still
        // decides what the client reads.
        let wrapped = Error::authentication("Session expired")
            .with_context("while verifying the token against /etc/rustf/keys/admin.pem");
        assert_eq!(wrapped.status_code(), 401);
        assert_eq!(wrapped.client_message(), "Session expired");

        let wrapped_internal =
            Error::internal("shard 4 offline").with_context("while loading the billing profile");
        assert_eq!(wrapped_internal.client_message(), "Internal Server Error");
    }
}
