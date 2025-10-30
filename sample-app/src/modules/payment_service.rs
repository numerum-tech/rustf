//! Payment Service Service
//!
//! This module provides Payment Service functionality
//!
//! As a shared module (service), it:
//! - Implements the SharedModule trait
//! - Can be registered as a singleton via MODULE::register()
//! - Can have multiple instances with different configurations
//! - Should handle stateful business logic
//!
//! # Usage
//! ```rust,ignore
//! use rustf::prelude::*;
//!
//! // Register the service with a unique name
//! MODULE::init()?;
//! MODULE::register("payment_service", PaymentService::new())?;
//!
//! // Access it elsewhere
//! let service = MODULE::get("payment_service")?;
//! ```

use rustf::prelude::*;
use rustf::impl_shared_service;
use anyhow::Result;

/// Payment Service Service - implements SharedModule for singleton management
#[derive(Debug, Clone)]
pub struct PaymentService {
    // Add your service configuration/dependencies here
    // config: ServiceConfig,
    // db: Arc<Database>,
    // cache: Arc<Cache>,
}

impl_shared_service!(PaymentService);

impl PaymentService {
    /// Create a new instance of PaymentService
    pub fn new() -> Self {
        Self {
            // Initialize your service here
        }
    }

    /// Example method: Process data
    ///
    /// # Arguments
    /// * `data` - Input data to process
    ///
    /// # Example
    /// ```rust,ignore
    /// let service = MODULE::get("payment_service")?;
    /// let result = service.process_data(data).await?;
    /// ```
    pub async fn process_data(&self, data: serde_json::Value) -> Result<serde_json::Value> {
        // TODO: Implement your business logic here

        // Example validation
        if data.get("required_field").is_none() {
            return Err(anyhow::anyhow!("Missing required field"));
        }

        // Process the data
        let processed = json!({
            "processed": true,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "original": data,
        });

        Ok(processed)
    }

    /// Example method: Validate input
    pub fn validate_input(&self, input: &str) -> Result<bool> {
        // TODO: Add your validation logic
        if input.is_empty() {
            return Err(anyhow::anyhow!("Input cannot be empty"));
        }

        if input.len() < 3 {
            return Err(anyhow::anyhow!("Input must be at least 3 characters"));
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_input() {
        let service = PaymentService::new();

        assert!(service.validate_input("hello").is_ok());
        assert!(service.validate_input("").is_err());
        assert!(service.validate_input("ab").is_err());
    }

    #[tokio::test]
    async fn test_process_data() {
        let service = PaymentService::new();

        let data = json!({
            "required_field": "value",
            "other": "data"
        });

        let result = service.process_data(data).await;
        assert!(result.is_ok());
    }
}

