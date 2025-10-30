//! Email service module demonstrating explicit MODULE::register() with named registration
//!
//! This service shows how to implement SharedModule and register it explicitly
//! with a unique name, allowing multiple instances with different configurations.

use anyhow::Result;
use rustf::impl_shared_service;
use rustf::prelude::*;

/// EmailService with different configurations (e.g., multiple sender accounts)
#[derive(Clone)]
pub struct EmailService {
    sender_email: String,
    sender_name: String,
    smtp_host: String,
}

impl_shared_service!(EmailService);

impl EmailService {
    /// Create a new email service with given configuration
    pub fn new(sender_email: String, sender_name: String, smtp_host: String) -> Self {
        Self {
            sender_email,
            sender_name,
            smtp_host,
        }
    }

    /// Send a test email
    pub async fn send_test(&self) -> Result<String> {
        Ok(format!(
            "Email sent from {} <{}> via {}",
            self.sender_name, self.sender_email, self.smtp_host
        ))
    }

    /// Get sender info
    pub fn sender_info(&self) -> String {
        format!("{} <{}>", self.sender_name, self.sender_email)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_email_service_creation() {
        let email = EmailService::new(
            "noreply@example.com".to_string(),
            "Example App".to_string(),
            "smtp.example.com".to_string(),
        );

        assert_eq!(email.sender_email, "noreply@example.com");
        assert_eq!(email.name(), "EmailService");
        assert_eq!(email.module_type(), SharedModuleType::Service);
    }

    #[tokio::test]
    async fn test_email_service_send() {
        let email = EmailService::new(
            "noreply@example.com".to_string(),
            "Example App".to_string(),
            "smtp.example.com".to_string(),
        );

        let result = email.send_test().await.unwrap();
        assert!(result.contains("Example App"));
    }
}
