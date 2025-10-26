//! Template helper system for view rendering
//!
//! This module provides a system for registering and using custom helper
//! functions in templates. Helpers can perform formatting, calculations,
//! and other data transformations during view rendering.

use crate::error::{Error, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for helper function results
pub type HelperResult = Result<Value>;

/// Trait for template helper functions
///
/// Implement this trait to create custom helper functions that can be
/// used in templates for data transformation and formatting.
pub trait Helper: Send + Sync {
    /// Execute the helper function with the given arguments
    ///
    /// # Arguments
    /// * `args` - Array of arguments passed to the helper
    /// * `context` - Optional context data from the template
    ///
    /// # Returns
    /// The result of the helper function as a JSON value
    fn call(&self, args: &[Value], context: Option<&Value>) -> HelperResult;

    /// Get the helper's name (for debugging)
    fn name(&self) -> &str {
        "unnamed"
    }

    /// Get the helper's description (for documentation)
    fn description(&self) -> &str {
        "No description available"
    }

    /// Validate arguments before execution
    fn validate_args(&self, _args: &[Value]) -> Result<()> {
        Ok(())
    }
}

/// Function-based helper implementation
struct FunctionHelper<F>
where
    F: Fn(&[Value], Option<&Value>) -> HelperResult + Send + Sync,
{
    func: F,
    name: String,
    description: String,
}

impl<F> Helper for FunctionHelper<F>
where
    F: Fn(&[Value], Option<&Value>) -> HelperResult + Send + Sync,
{
    fn call(&self, args: &[Value], context: Option<&Value>) -> HelperResult {
        (self.func)(args, context)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }
}

/// Registry for template helpers
pub struct HelperRegistry {
    helpers: HashMap<String, Arc<dyn Helper>>,
}

// Manual Debug implementation since Arc<dyn Helper> doesn't implement Debug
impl std::fmt::Debug for HelperRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HelperRegistry")
            .field("helper_count", &self.helpers.len())
            .field("helper_names", &self.helpers.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl HelperRegistry {
    /// Create a new empty helper registry
    pub fn new() -> Self {
        let mut registry = Self {
            helpers: HashMap::new(),
        };

        // Register built-in helpers
        registry.register_builtin_helpers();

        registry
    }

    /// Register a helper function
    pub fn register(&mut self, name: &str, helper: impl Helper + 'static) {
        log::debug!("Registering template helper: {}", name);
        self.helpers.insert(name.to_string(), Arc::new(helper));
    }

    /// Register a function as a helper
    pub fn register_fn<F>(&mut self, name: &str, description: &str, func: F)
    where
        F: Fn(&[Value], Option<&Value>) -> HelperResult + Send + Sync + 'static,
    {
        let helper = FunctionHelper {
            func,
            name: name.to_string(),
            description: description.to_string(),
        };
        self.register(name, helper);
    }

    /// Get a helper by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Helper>> {
        self.helpers.get(name).cloned()
    }

    /// Check if a helper exists
    pub fn exists(&self, name: &str) -> bool {
        self.helpers.contains_key(name)
    }

    /// Get the number of registered helpers
    pub fn count(&self) -> usize {
        self.helpers.len()
    }

    /// List all registered helper names
    pub fn list(&self) -> Vec<String> {
        self.helpers.keys().cloned().collect()
    }

    /// Call a helper by name
    pub fn call(&self, name: &str, args: &[Value], context: Option<&Value>) -> HelperResult {
        match self.helpers.get(name) {
            Some(helper) => {
                helper.validate_args(args)?;
                helper.call(args, context)
            }
            None => Err(Error::internal(format!("Helper '{}' not found", name))),
        }
    }

    /// Register built-in helpers
    fn register_builtin_helpers(&mut self) {
        // Format currency helper
        self.register_fn(
            "format_currency",
            "Format a number as currency",
            |args, _| {
                if args.is_empty() {
                    return Ok(Value::String("$0.00".to_string()));
                }

                let amount = args[0].as_f64().unwrap_or(0.0);
                let currency = args.get(1).and_then(|v| v.as_str()).unwrap_or("USD");

                let formatted = match currency {
                    "USD" => format!("${:.2}", amount),
                    "EUR" => format!("€{:.2}", amount),
                    "GBP" => format!("£{:.2}", amount),
                    "JPY" => format!("¥{:.0}", amount),
                    _ => format!("{} {:.2}", currency, amount),
                };

                Ok(Value::String(formatted))
            },
        );

        // Truncate text helper
        self.register_fn(
            "truncate",
            "Truncate text to specified length",
            |args, _| {
                if args.len() < 2 {
                    return Err(Error::internal(
                        "truncate helper requires text and length arguments".to_string(),
                    ));
                }

                let text = args[0].as_str().unwrap_or("");
                let length = args[1].as_u64().unwrap_or(50) as usize;
                let suffix = args.get(2).and_then(|v| v.as_str()).unwrap_or("...");

                let result = if text.len() > length {
                    format!("{}{}", &text[..length.min(text.len())], suffix)
                } else {
                    text.to_string()
                };

                Ok(Value::String(result))
            },
        );

        // Pluralize helper
        self.register_fn("pluralize", "Pluralize text based on count", |args, _| {
            if args.is_empty() {
                return Err(Error::internal(
                    "pluralize helper requires at least count argument".to_string(),
                ));
            }

            let count = args[0].as_i64().unwrap_or(0);
            let singular = args.get(1).and_then(|v| v.as_str()).unwrap_or("item");

            let default_plural = format!("{}s", singular);
            let plural = args
                .get(2)
                .and_then(|v| v.as_str())
                .unwrap_or(&default_plural);

            let result = if count == 1 {
                format!("{} {}", count, singular)
            } else {
                format!("{} {}", count, plural)
            };

            Ok(Value::String(result))
        });

        // Time ago helper
        self.register_fn(
            "time_ago",
            "Format time as relative (e.g., '2 hours ago')",
            |args, _| {
                use std::time::{SystemTime, UNIX_EPOCH};

                if args.is_empty() {
                    return Ok(Value::String("just now".to_string()));
                }

                let timestamp = args[0].as_u64().unwrap_or(0);
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let diff = now.saturating_sub(timestamp);

                let result = match diff {
                    0..=59 => "just now".to_string(),
                    60..=119 => "1 minute ago".to_string(),
                    120..=3599 => format!("{} minutes ago", diff / 60),
                    3600..=7199 => "1 hour ago".to_string(),
                    7200..=86399 => format!("{} hours ago", diff / 3600),
                    86400..=172799 => "1 day ago".to_string(),
                    _ => format!("{} days ago", diff / 86400),
                };

                Ok(Value::String(result))
            },
        );

        // Format date helper
        self.register_fn(
            "format_date",
            "Format timestamp as date string",
            |args, _| {
                use chrono::{TimeZone, Utc};

                if args.is_empty() {
                    return Ok(Value::String("".to_string()));
                }

                let timestamp = args[0].as_i64().unwrap_or(0);
                let format = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .unwrap_or("%Y-%m-%d %H:%M:%S");

                let datetime = Utc
                    .timestamp_opt(timestamp, 0)
                    .single()
                    .unwrap_or_else(Utc::now);

                let formatted = datetime.format(format).to_string();
                Ok(Value::String(formatted))
            },
        );

        // URL encode helper
        self.register_fn("url_encode", "URL encode a string", |args, _| {
            if args.is_empty() {
                return Ok(Value::String("".to_string()));
            }

            let text = args[0].as_str().unwrap_or("");
            let encoded = urlencoding::encode(text);
            Ok(Value::String(encoded.into_owned()))
        });

        // URL decode helper
        self.register_fn("url_decode", "URL decode a string", |args, _| {
            if args.is_empty() {
                return Ok(Value::String("".to_string()));
            }

            let text = args[0].as_str().unwrap_or("");
            let decoded = urlencoding::decode(text).unwrap_or_else(|_| text.into());
            Ok(Value::String(decoded.into_owned()))
        });

        // JSON stringify helper
        self.register_fn("json", "Convert value to JSON string", |args, _| {
            if args.is_empty() {
                return Ok(Value::String("null".to_string()));
            }

            let json_str = serde_json::to_string(&args[0]).unwrap_or_else(|_| "null".to_string());
            Ok(Value::String(json_str))
        });

        // Default value helper
        self.register_fn(
            "default",
            "Return default if value is null/empty",
            |args, _| {
                if args.len() < 2 {
                    return Ok(Value::Null);
                }

                let value = &args[0];
                let default = &args[1];

                match value {
                    Value::Null => Ok(default.clone()),
                    Value::String(s) if s.is_empty() => Ok(default.clone()),
                    Value::Array(a) if a.is_empty() => Ok(default.clone()),
                    Value::Object(o) if o.is_empty() => Ok(default.clone()),
                    _ => Ok(value.clone()),
                }
            },
        );

        // Capitalize helper
        self.register_fn("capitalize", "Capitalize first letter", |args, _| {
            if args.is_empty() {
                return Ok(Value::String("".to_string()));
            }

            let text = args[0].as_str().unwrap_or("");
            let capitalized = text
                .chars()
                .take(1)
                .flat_map(char::to_uppercase)
                .chain(text.chars().skip(1))
                .collect::<String>();

            Ok(Value::String(capitalized))
        });

        // Slugify helper
        self.register_fn("slugify", "Convert text to URL-friendly slug", |args, _| {
            if args.is_empty() {
                return Ok(Value::String("".to_string()));
            }

            let text = args[0].as_str().unwrap_or("");
            let slug = text
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect::<String>()
                .split('-')
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("-");

            Ok(Value::String(slug))
        });

        // File size helper
        self.register_fn(
            "file_size",
            "Format bytes as human-readable size",
            |args, _| {
                if args.is_empty() {
                    return Ok(Value::String("0 B".to_string()));
                }

                let bytes = args[0].as_u64().unwrap_or(0) as f64;
                let units = ["B", "KB", "MB", "GB", "TB", "PB"];

                let mut size = bytes;
                let mut unit_index = 0;

                while size >= 1024.0 && unit_index < units.len() - 1 {
                    size /= 1024.0;
                    unit_index += 1;
                }

                let formatted = if unit_index == 0 {
                    format!("{:.0} {}", size, units[unit_index])
                } else {
                    format!("{:.2} {}", size, units[unit_index])
                };

                Ok(Value::String(formatted))
            },
        );
    }
}

impl Default for HelperRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Example custom helper implementation
pub struct EmailHelper;

impl Helper for EmailHelper {
    fn call(&self, args: &[Value], _context: Option<&Value>) -> HelperResult {
        if args.is_empty() {
            return Ok(Value::String("".to_string()));
        }

        let email = args[0].as_str().unwrap_or("");

        // Simple email obfuscation
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() == 2 {
            let username = parts[0];
            let domain = parts[1];
            let obfuscated = format!(
                "{}...@{}",
                &username.chars().take(2).collect::<String>(),
                domain
            );
            Ok(Value::String(obfuscated))
        } else {
            Ok(Value::String(email.to_string()))
        }
    }

    fn name(&self) -> &str {
        "obfuscate_email"
    }

    fn description(&self) -> &str {
        "Obfuscate email address for display"
    }

    fn validate_args(&self, args: &[Value]) -> Result<()> {
        if args.is_empty() {
            return Err(Error::internal(
                "obfuscate_email helper requires an email argument".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_format_currency() {
        let registry = HelperRegistry::new();

        let result = registry
            .call("format_currency", &[json!(42.50), json!("USD")], None)
            .unwrap();
        assert_eq!(result, json!("$42.50"));

        let result = registry
            .call("format_currency", &[json!(42.50), json!("EUR")], None)
            .unwrap();
        assert_eq!(result, json!("€42.50"));
    }

    #[test]
    fn test_truncate() {
        let registry = HelperRegistry::new();

        let result = registry
            .call("truncate", &[json!("Hello, World!"), json!(5)], None)
            .unwrap();
        assert_eq!(result, json!("Hello..."));

        let result = registry
            .call("truncate", &[json!("Short"), json!(10)], None)
            .unwrap();
        assert_eq!(result, json!("Short"));
    }

    #[test]
    fn test_pluralize() {
        let registry = HelperRegistry::new();

        let result = registry
            .call("pluralize", &[json!(1), json!("item")], None)
            .unwrap();
        assert_eq!(result, json!("1 item"));

        let result = registry
            .call("pluralize", &[json!(5), json!("item")], None)
            .unwrap();
        assert_eq!(result, json!("5 items"));

        let result = registry
            .call(
                "pluralize",
                &[json!(2), json!("person"), json!("people")],
                None,
            )
            .unwrap();
        assert_eq!(result, json!("2 people"));
    }

    #[test]
    fn test_slugify() {
        let registry = HelperRegistry::new();

        let result = registry
            .call("slugify", &[json!("Hello World!")], None)
            .unwrap();
        assert_eq!(result, json!("hello-world"));

        let result = registry
            .call("slugify", &[json!("This & That")], None)
            .unwrap();
        assert_eq!(result, json!("this-that"));
    }

    #[test]
    fn test_file_size() {
        let registry = HelperRegistry::new();

        let result = registry.call("file_size", &[json!(1024)], None).unwrap();
        assert_eq!(result, json!("1.00 KB"));

        let result = registry.call("file_size", &[json!(1048576)], None).unwrap();
        assert_eq!(result, json!("1.00 MB"));

        let result = registry.call("file_size", &[json!(500)], None).unwrap();
        assert_eq!(result, json!("500 B"));
    }
}
