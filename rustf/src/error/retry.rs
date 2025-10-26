//! Retry logic for transient errors
//!
//! Provides configurable retry policies and utilities for handling
//! transient failures in network operations, database connections, etc.

use super::{Error, Result};
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

/// Trait for errors that can be retried
pub trait RetryableError {
    /// Check if the error is retryable
    fn is_retryable(&self) -> bool;

    /// Get suggested delay before retry (if any)
    fn retry_after(&self) -> Option<Duration> {
        None
    }
}

impl RetryableError for Error {
    fn is_retryable(&self) -> bool {
        self.is_retryable()
    }

    fn retry_after(&self) -> Option<Duration> {
        // Could be enhanced to parse Retry-After headers for HTTP errors
        match self {
            Error::RateLimit(_) => Some(Duration::from_secs(60)),
            Error::Timeout(_) => Some(Duration::from_secs(5)),
            _ => None,
        }
    }
}

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Backoff multiplier (e.g., 2.0 for exponential backoff)
    pub backoff_multiplier: f32,
    /// Add random jitter to delays
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Create a policy with fixed delay
    pub fn fixed(attempts: u32, delay: Duration) -> Self {
        Self {
            max_attempts: attempts,
            initial_delay: delay,
            max_delay: delay,
            backoff_multiplier: 1.0,
            jitter: false,
        }
    }

    /// Create a policy with exponential backoff
    pub fn exponential(attempts: u32) -> Self {
        Self {
            max_attempts: attempts,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }

    /// Create a policy with linear backoff
    pub fn linear(attempts: u32, increment: Duration) -> Self {
        Self {
            max_attempts: attempts,
            initial_delay: increment,
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 1.0,
            jitter: false,
        }
    }

    /// Calculate delay for a given attempt
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let mut delay = self.initial_delay;

        if self.backoff_multiplier > 1.0 {
            let multiplier = self.backoff_multiplier.powi(attempt as i32 - 1);
            delay = Duration::from_secs_f32(delay.as_secs_f32() * multiplier);
        } else if attempt > 1 {
            delay = Duration::from_secs_f32(self.initial_delay.as_secs_f32() * attempt as f32);
        }

        // Cap at max delay
        if delay > self.max_delay {
            delay = self.max_delay;
        }

        // Add jitter if enabled
        if self.jitter {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let jitter_factor = rng.gen_range(0.5..1.5);
            delay = Duration::from_secs_f32(delay.as_secs_f32() * jitter_factor);
        }

        delay
    }
}

/// Execute an async operation with retry logic
pub async fn with_retry<F, Fut, T>(policy: RetryPolicy, mut operation: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut last_error = None;

    for attempt in 1..=policy.max_attempts {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                // Check if error is retryable
                if !error.is_retryable() || attempt == policy.max_attempts {
                    return Err(error);
                }

                // Calculate delay
                let delay = if let Some(retry_after) = error.retry_after() {
                    retry_after
                } else {
                    policy.calculate_delay(attempt)
                };

                log::warn!(
                    "Retry attempt {}/{} after {:?} for error: {}",
                    attempt,
                    policy.max_attempts,
                    delay,
                    error
                );

                last_error = Some(error);
                sleep(delay).await;
            }
        }
    }

    Err(last_error.unwrap_or_else(|| Error::internal("Retry failed with no error")))
}

/// Builder for retry operations
pub struct RetryBuilder {
    policy: RetryPolicy,
}

impl RetryBuilder {
    /// Create a new retry builder
    pub fn new() -> Self {
        Self {
            policy: RetryPolicy::default(),
        }
    }

    /// Set maximum number of attempts
    pub fn max_attempts(mut self, attempts: u32) -> Self {
        self.policy.max_attempts = attempts;
        self
    }

    /// Set initial delay
    pub fn initial_delay(mut self, delay: Duration) -> Self {
        self.policy.initial_delay = delay;
        self
    }

    /// Set maximum delay
    pub fn max_delay(mut self, delay: Duration) -> Self {
        self.policy.max_delay = delay;
        self
    }

    /// Set backoff multiplier
    pub fn backoff_multiplier(mut self, multiplier: f32) -> Self {
        self.policy.backoff_multiplier = multiplier;
        self
    }

    /// Enable or disable jitter
    pub fn jitter(mut self, enabled: bool) -> Self {
        self.policy.jitter = enabled;
        self
    }

    /// Execute the operation with the configured policy
    pub async fn execute<F, Fut, T>(self, operation: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        with_retry(self.policy, operation).await
    }
}

impl Default for RetryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Circuit breaker for preventing cascading failures
pub struct CircuitBreaker {
    failure_threshold: u32,
    success_threshold: u32,
    timeout: Duration,
    half_open_max_calls: u32,

    state: std::sync::Arc<tokio::sync::RwLock<CircuitState>>,
}

#[derive(Debug, Clone)]
enum CircuitState {
    Closed {
        failure_count: u32,
    },
    Open {
        opened_at: std::time::Instant,
    },
    HalfOpen {
        success_count: u32,
        failure_count: u32,
        calls_count: u32,
    },
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(
        failure_threshold: u32,
        success_threshold: u32,
        timeout: Duration,
        half_open_max_calls: u32,
    ) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            timeout,
            half_open_max_calls,
            state: std::sync::Arc::new(tokio::sync::RwLock::new(CircuitState::Closed {
                failure_count: 0,
            })),
        }
    }

    /// Execute an operation through the circuit breaker
    pub async fn execute<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        // Check if we should allow the call
        {
            let mut state = self.state.write().await;
            match *state {
                CircuitState::Open { opened_at } => {
                    if opened_at.elapsed() < self.timeout {
                        return Err(Error::external_service(
                            "circuit_breaker",
                            "Circuit breaker is open",
                        ));
                    }
                    // Transition to half-open
                    *state = CircuitState::HalfOpen {
                        success_count: 0,
                        failure_count: 0,
                        calls_count: 0,
                    };
                }
                CircuitState::HalfOpen { calls_count, .. } => {
                    if calls_count >= self.half_open_max_calls {
                        return Err(Error::external_service(
                            "circuit_breaker",
                            "Circuit breaker is testing",
                        ));
                    }
                }
                _ => {}
            }
        }

        // Execute the operation
        let result = operation().await;

        // Update state based on result
        let mut state = self.state.write().await;
        match result {
            Ok(_) => match *state {
                CircuitState::Closed { .. } => {}
                CircuitState::HalfOpen {
                    success_count,
                    calls_count,
                    ..
                } => {
                    if success_count + 1 >= self.success_threshold {
                        *state = CircuitState::Closed { failure_count: 0 };
                    } else {
                        *state = CircuitState::HalfOpen {
                            success_count: success_count + 1,
                            failure_count: 0,
                            calls_count: calls_count + 1,
                        };
                    }
                }
                _ => {}
            },
            Err(_) => match *state {
                CircuitState::Closed { failure_count } => {
                    if failure_count + 1 >= self.failure_threshold {
                        *state = CircuitState::Open {
                            opened_at: std::time::Instant::now(),
                        };
                    } else {
                        *state = CircuitState::Closed {
                            failure_count: failure_count + 1,
                        };
                    }
                }
                CircuitState::HalfOpen { failure_count, .. } => {
                    if failure_count + 1 >= self.failure_threshold {
                        *state = CircuitState::Open {
                            opened_at: std::time::Instant::now(),
                        };
                    }
                }
                _ => {}
            },
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_delay_calculation() {
        let policy = RetryPolicy::exponential(3);

        // Test exponential backoff without jitter
        let mut policy_no_jitter = policy.clone();
        policy_no_jitter.jitter = false;

        let delay1 = policy_no_jitter.calculate_delay(1);
        let delay2 = policy_no_jitter.calculate_delay(2);
        let delay3 = policy_no_jitter.calculate_delay(3);

        assert!(delay1.as_millis() >= 99 && delay1.as_millis() <= 101);
        assert!(delay2.as_millis() >= 199 && delay2.as_millis() <= 201);
        assert!(delay3.as_millis() >= 399 && delay3.as_millis() <= 401);
    }

    #[tokio::test]
    async fn test_retry_with_success() {
        let attempt = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempt_clone = attempt.clone();
        let policy = RetryPolicy::fixed(3, Duration::from_millis(10));

        let result = with_retry(policy, || {
            let attempt_inner = attempt_clone.clone();
            async move {
                let count = attempt_inner.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if count < 1 {
                    Err(Error::network("Temporary failure"))
                } else {
                    Ok("Success")
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
        assert_eq!(attempt.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_retry_with_non_retryable_error() {
        let attempt = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempt_clone = attempt.clone();
        let policy = RetryPolicy::fixed(3, Duration::from_millis(10));

        let result: Result<()> = with_retry(policy, || {
            let attempt_inner = attempt_clone.clone();
            async move {
                attempt_inner.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Err(Error::validation("Invalid input"))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempt.load(std::sync::atomic::Ordering::SeqCst), 1); // Should not retry
    }

    #[tokio::test]
    async fn test_retry_builder() {
        let attempt = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempt_clone = attempt.clone();

        let result = RetryBuilder::new()
            .max_attempts(2)
            .initial_delay(Duration::from_millis(5))
            .jitter(false)
            .execute(|| {
                let attempt_inner = attempt_clone.clone();
                async move {
                    let count = attempt_inner.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if count == 0 {
                        Err(Error::timeout("Timeout"))
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempt.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let breaker = CircuitBreaker::new(2, 2, Duration::from_millis(100), 3);

        // First call succeeds
        let result = breaker.execute(|| async { Ok("Success") }).await;
        assert!(result.is_ok());

        // Two failures trigger open state
        for _ in 0..2 {
            let _: Result<()> = breaker
                .execute(|| async { Err(Error::network("Failed")) })
                .await;
        }

        // Circuit should be open now
        let result = breaker.execute(|| async { Ok("Should not execute") }).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Circuit breaker is open"));

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Circuit should be half-open, allowing test calls
        let result = breaker.execute(|| async { Ok("Success again") }).await;
        assert!(result.is_ok());
    }
}
