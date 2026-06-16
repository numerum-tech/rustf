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
}
