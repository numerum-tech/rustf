//! Validation system for data validation
//!
//! This module provides a system for registering and using validators
//! for data validation in models, forms, and API requests.

use crate::error::{Error, Result};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Result type for validators
pub type ValidationResult = Result<()>;

/// Trait for validators
///
/// Implement this trait to create custom validators that can be
/// used for data validation throughout the application.
pub trait Validator: Send + Sync {
    /// Validate a value
    ///
    /// # Arguments
    /// * `value` - The value to validate
    /// * `options` - Optional validation options
    ///
    /// # Returns
    /// Ok(()) if valid, Err with validation message if invalid
    fn validate(&self, value: &Value, options: Option<&Value>) -> ValidationResult;

    /// Get the validator's name
    fn name(&self) -> &str {
        "unnamed"
    }

    /// Get the validator's description
    fn description(&self) -> &str {
        "No description available"
    }
}

/// Function-based validator implementation
struct FunctionValidator<F>
where
    F: Fn(&Value, Option<&Value>) -> ValidationResult + Send + Sync,
{
    func: F,
    name: String,
    description: String,
}

impl<F> Validator for FunctionValidator<F>
where
    F: Fn(&Value, Option<&Value>) -> ValidationResult + Send + Sync,
{
    fn validate(&self, value: &Value, options: Option<&Value>) -> ValidationResult {
        (self.func)(value, options)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }
}

/// Registry for validators
pub struct ValidatorRegistry {
    validators: HashMap<String, Arc<dyn Validator>>,
}

// Manual Debug implementation since Arc<dyn Validator> doesn't implement Debug
impl std::fmt::Debug for ValidatorRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidatorRegistry")
            .field("validator_count", &self.validators.len())
            .field(
                "validator_names",
                &self.validators.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl ValidatorRegistry {
    /// Create a new validator registry
    pub fn new() -> Self {
        let mut registry = Self {
            validators: HashMap::new(),
        };

        // Register built-in validators
        registry.register_builtin_validators();

        registry
    }

    /// Register a validator
    pub fn register(&mut self, name: &str, validator: impl Validator + 'static) {
        log::debug!("Registering validator: {}", name);
        self.validators
            .insert(name.to_string(), Arc::new(validator));
    }

    /// Register a function as a validator
    pub fn register_fn<F>(&mut self, name: &str, description: &str, func: F)
    where
        F: Fn(&Value, Option<&Value>) -> ValidationResult + Send + Sync + 'static,
    {
        let validator = FunctionValidator {
            func,
            name: name.to_string(),
            description: description.to_string(),
        };
        self.register(name, validator);
    }

    /// Get a validator by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Validator>> {
        self.validators.get(name).cloned()
    }

    /// Check if a validator exists
    pub fn exists(&self, name: &str) -> bool {
        self.validators.contains_key(name)
    }

    /// Get the number of registered validators
    pub fn count(&self) -> usize {
        self.validators.len()
    }

    /// List all registered validator names
    pub fn list(&self) -> Vec<String> {
        self.validators.keys().cloned().collect()
    }

    /// Validate a value using a named validator
    pub fn validate(
        &self,
        validator_name: &str,
        value: &Value,
        options: Option<&Value>,
    ) -> ValidationResult {
        match self.validators.get(validator_name) {
            Some(validator) => validator.validate(value, options),
            None => Err(Error::internal(format!(
                "Validator '{}' not found",
                validator_name
            ))),
        }
    }

    /// Register built-in validators
    fn register_builtin_validators(&mut self) {
        // Required validator
        self.register_fn(
            "required",
            "Value must not be null or empty",
            |value, _| match value {
                Value::Null => Err(Error::validation("Value is required")),
                Value::String(s) if s.is_empty() => Err(Error::validation("Value is required")),
                Value::Array(a) if a.is_empty() => Err(Error::validation("Value is required")),
                _ => Ok(()),
            },
        );

        // Email validator
        self.register("email", EmailValidator);

        // URL validator
        self.register("url", UrlValidator);

        // Phone validator
        self.register("phone", PhoneValidator);

        // Credit card validator
        self.register("credit_card", CreditCardValidator);

        // Min length validator
        self.register_fn(
            "min_length",
            "Minimum length validation",
            |value, options| {
                let min = options
                    .and_then(|o| o.get("min"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;

                match value {
                    Value::String(s) if s.len() < min => Err(Error::validation(format!(
                        "Must be at least {} characters",
                        min
                    ))),
                    Value::Array(a) if a.len() < min => Err(Error::validation(format!(
                        "Must have at least {} items",
                        min
                    ))),
                    _ => Ok(()),
                }
            },
        );

        // Max length validator
        self.register_fn(
            "max_length",
            "Maximum length validation",
            |value, options| {
                let max = options
                    .and_then(|o| o.get("max"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(u64::MAX) as usize;

                match value {
                    Value::String(s) if s.len() > max => Err(Error::validation(format!(
                        "Must be at most {} characters",
                        max
                    ))),
                    Value::Array(a) if a.len() > max => Err(Error::validation(format!(
                        "Must have at most {} items",
                        max
                    ))),
                    _ => Ok(()),
                }
            },
        );

        // Range validator
        self.register_fn("range", "Numeric range validation", |value, options| {
            let min = options
                .and_then(|o| o.get("min"))
                .and_then(|v| v.as_f64())
                .unwrap_or(f64::MIN);
            let max = options
                .and_then(|o| o.get("max"))
                .and_then(|v| v.as_f64())
                .unwrap_or(f64::MAX);

            if let Some(num) = value.as_f64() {
                if num < min || num > max {
                    return Err(Error::validation(format!(
                        "Must be between {} and {}",
                        min, max
                    )));
                }
            }
            Ok(())
        });

        // Pattern validator
        self.register_fn(
            "pattern",
            "Regular expression validation",
            |value, options| {
                let pattern = options
                    .and_then(|o| o.get("pattern"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(".*");

                if let Ok(regex) = Regex::new(pattern) {
                    if let Some(text) = value.as_str() {
                        if !regex.is_match(text) {
                            return Err(Error::validation("Value does not match required pattern"));
                        }
                    }
                }
                Ok(())
            },
        );

        // Alpha validator
        self.register_fn("alpha", "Alphabetic characters only", |value, _| {
            if let Some(text) = value.as_str() {
                if !text.chars().all(|c| c.is_alphabetic()) {
                    return Err(Error::validation("Must contain only alphabetic characters"));
                }
            }
            Ok(())
        });

        // Alphanumeric validator
        self.register_fn(
            "alphanumeric",
            "Alphanumeric characters only",
            |value, _| {
                if let Some(text) = value.as_str() {
                    if !text.chars().all(|c| c.is_alphanumeric()) {
                        return Err(Error::validation(
                            "Must contain only alphanumeric characters",
                        ));
                    }
                }
                Ok(())
            },
        );

        // Numeric validator
        self.register_fn(
            "numeric",
            "Numeric value validation",
            |value, _| match value {
                Value::Number(_) => Ok(()),
                Value::String(s) => {
                    if s.parse::<f64>().is_ok() {
                        Ok(())
                    } else {
                        Err(Error::validation("Must be a numeric value"))
                    }
                }
                _ => Err(Error::validation("Must be a numeric value")),
            },
        );

        // Boolean validator
        self.register_fn(
            "boolean",
            "Boolean value validation",
            |value, _| match value {
                Value::Bool(_) => Ok(()),
                Value::String(s) => {
                    let lower = s.to_lowercase();
                    if lower == "true" || lower == "false" || lower == "1" || lower == "0" {
                        Ok(())
                    } else {
                        Err(Error::validation("Must be a boolean value"))
                    }
                }
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        if i == 0 || i == 1 {
                            return Ok(());
                        }
                    }
                    Err(Error::validation("Must be a boolean value"))
                }
                _ => Err(Error::validation("Must be a boolean value")),
            },
        );

        // UUID validator
        self.register_fn("uuid", "UUID format validation", |value, _| {
            if let Some(text) = value.as_str() {
                let uuid_regex =
                    Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
                        .unwrap();
                if !uuid_regex.is_match(&text.to_lowercase()) {
                    return Err(Error::validation("Must be a valid UUID"));
                }
            }
            Ok(())
        });
    }
}

impl Default for ValidatorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Email validator implementation
pub struct EmailValidator;

impl Validator for EmailValidator {
    fn validate(&self, value: &Value, _options: Option<&Value>) -> ValidationResult {
        if let Some(email) = value.as_str() {
            // Simple email regex (not fully RFC compliant but good enough for most cases)
            let email_regex = Regex::new(
                r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$"
            ).unwrap();

            if !email_regex.is_match(email) {
                return Err(Error::validation("Invalid email address"));
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "email"
    }

    fn description(&self) -> &str {
        "Validates email address format"
    }
}

/// URL validator implementation
pub struct UrlValidator;

impl Validator for UrlValidator {
    fn validate(&self, value: &Value, _options: Option<&Value>) -> ValidationResult {
        if let Some(url) = value.as_str() {
            if url::Url::parse(url).is_err() {
                return Err(Error::validation("Invalid URL format"));
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "url"
    }

    fn description(&self) -> &str {
        "Validates URL format"
    }
}

/// Phone number validator
pub struct PhoneValidator;

impl Validator for PhoneValidator {
    fn validate(&self, value: &Value, _options: Option<&Value>) -> ValidationResult {
        if let Some(phone) = value.as_str() {
            // Remove common formatting characters
            let cleaned: String = phone
                .chars()
                .filter(|c| c.is_numeric() || *c == '+')
                .collect();

            // Check for minimum length (7 digits for local numbers)
            if cleaned.len() < 7 {
                return Err(Error::validation("Phone number too short"));
            }

            // Check for maximum length (15 digits for international)
            if cleaned.len() > 15 {
                return Err(Error::validation("Phone number too long"));
            }

            // If starts with +, must be followed by country code
            if cleaned.starts_with('+') && cleaned.len() < 8 {
                return Err(Error::validation("Invalid international phone number"));
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "phone"
    }

    fn description(&self) -> &str {
        "Validates phone number format"
    }
}

/// Credit card validator using Luhn algorithm
pub struct CreditCardValidator;

impl CreditCardValidator {
    /// Validate credit card number using Luhn algorithm
    fn luhn_check(number: &str) -> bool {
        let digits: Vec<u32> = number
            .chars()
            .filter(|c| c.is_numeric())
            .filter_map(|c| c.to_digit(10))
            .collect();

        if digits.len() < 13 || digits.len() > 19 {
            return false;
        }

        let mut sum = 0;
        let mut alternate = false;

        for digit in digits.iter().rev() {
            let mut n = *digit;
            if alternate {
                n *= 2;
                if n > 9 {
                    n -= 9;
                }
            }
            sum += n;
            alternate = !alternate;
        }

        sum % 10 == 0
    }
}

impl Validator for CreditCardValidator {
    fn validate(&self, value: &Value, _options: Option<&Value>) -> ValidationResult {
        if let Some(card_number) = value.as_str() {
            if !Self::luhn_check(card_number) {
                return Err(Error::validation("Invalid credit card number"));
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "credit_card"
    }

    fn description(&self) -> &str {
        "Validates credit card number using Luhn algorithm"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_required_validator() {
        let registry = ValidatorRegistry::new();

        assert!(registry.validate("required", &json!(null), None).is_err());
        assert!(registry.validate("required", &json!(""), None).is_err());
        assert!(registry.validate("required", &json!([]), None).is_err());
        assert!(registry.validate("required", &json!("value"), None).is_ok());
        assert!(registry.validate("required", &json!(123), None).is_ok());
    }

    #[test]
    fn test_email_validator() {
        let registry = ValidatorRegistry::new();

        assert!(registry
            .validate("email", &json!("test@example.com"), None)
            .is_ok());
        assert!(registry
            .validate("email", &json!("user.name+tag@example.co.uk"), None)
            .is_ok());
        assert!(registry
            .validate("email", &json!("invalid.email"), None)
            .is_err());
        assert!(registry
            .validate("email", &json!("@example.com"), None)
            .is_err());
        assert!(registry.validate("email", &json!("test@"), None).is_err());
    }

    #[test]
    fn test_url_validator() {
        let registry = ValidatorRegistry::new();

        assert!(registry
            .validate("url", &json!("https://example.com"), None)
            .is_ok());
        assert!(registry
            .validate("url", &json!("http://localhost:8080"), None)
            .is_ok());
        assert!(registry
            .validate("url", &json!("ftp://files.example.com"), None)
            .is_ok());
        assert!(registry.validate("url", &json!("not-a-url"), None).is_err());
        assert!(registry
            .validate("url", &json!("//example.com"), None)
            .is_err());
    }

    #[test]
    fn test_phone_validator() {
        let registry = ValidatorRegistry::new();

        assert!(registry.validate("phone", &json!("1234567"), None).is_ok());
        assert!(registry
            .validate("phone", &json!("+1234567890"), None)
            .is_ok());
        assert!(registry
            .validate("phone", &json!("(555) 123-4567"), None)
            .is_ok());
        assert!(registry.validate("phone", &json!("123"), None).is_err());
        assert!(registry
            .validate("phone", &json!("1234567890123456"), None)
            .is_err());
    }

    #[test]
    fn test_credit_card_validator() {
        let registry = ValidatorRegistry::new();

        // Valid test credit card numbers
        assert!(registry
            .validate("credit_card", &json!("4532015112830366"), None)
            .is_ok()); // Visa
        assert!(registry
            .validate("credit_card", &json!("5425233430109903"), None)
            .is_ok()); // Mastercard
        assert!(registry
            .validate("credit_card", &json!("374245455400126"), None)
            .is_ok()); // Amex

        // Invalid numbers
        assert!(registry
            .validate("credit_card", &json!("1234567890123456"), None)
            .is_err());
        // Note: 0000000000000000 passes Luhn check (sum of zeros = 0, divisible by 10)
        // but we could add additional validation to reject obviously fake numbers
        assert!(registry
            .validate("credit_card", &json!("0000000000000000"), None)
            .is_ok());
    }

    #[test]
    fn test_range_validator() {
        let registry = ValidatorRegistry::new();
        let options = json!({"min": 0, "max": 100});

        assert!(registry
            .validate("range", &json!(50), Some(&options))
            .is_ok());
        assert!(registry
            .validate("range", &json!(0), Some(&options))
            .is_ok());
        assert!(registry
            .validate("range", &json!(100), Some(&options))
            .is_ok());
        assert!(registry
            .validate("range", &json!(-1), Some(&options))
            .is_err());
        assert!(registry
            .validate("range", &json!(101), Some(&options))
            .is_err());
    }
}
