//! Application definitions
//!
//! Customises framework behaviour through the definitions system:
//! template helpers, validators, and the session-storage factory.
//!
//! NOTE: The definitions system covers helpers + validators + a session-
//! storage factory. It does NOT include a generic interceptor/provider
//! system — if you need one, reach for middleware, events, or shared
//! modules instead.

use rustf::definitions::Definitions;
use serde_json::Value;

/// Install function called by auto-discovery.
pub fn install(defs: &mut Definitions) {
    register_helpers(defs);

    // Uncomment to add more customisations:
    // register_validators(defs);
    // defs.set_session_storage_factory(my_session_factory);
}

/// Register template helpers (callable from views).
fn register_helpers(defs: &mut Definitions) {
    // Closure-based helper — goes through HelperRegistry::register_fn,
    // which takes (name, description, closure).
    defs.helpers.register_fn(
        "format_number",
        "Format a number with thousands separators (e.g. 1234 -> 1,234)",
        |args, _ctx| {
            if let Some(Value::Number(n)) = args.first() {
                if let Some(num) = n.as_u64() {
                    let formatted = format!("{}", num)
                        .chars()
                        .rev()
                        .enumerate()
                        .map(|(i, c)| {
                            if i > 0 && i % 3 == 0 {
                                format!(",{}", c)
                            } else {
                                c.to_string()
                            }
                        })
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect::<String>();
                    return Ok(Value::String(formatted));
                }
            }
            Ok(args.first().cloned().unwrap_or(Value::Null))
        },
    );

    // App-specific helper.
    defs.helpers.register_fn(
        "app_version",
        "Return the current app version string",
        |_args, _ctx| Ok(Value::String("1.0.0".to_string())),
    );
}

// Example: Custom validators (uncomment to use). Same register_fn shape:
// (name, description, closure) — goes through ValidatorRegistry::register_fn.
/*
fn register_validators(defs: &mut Definitions) {
    defs.validators.register_fn(
        "strong_password",
        "Require 8+ chars with upper, lower, and a digit",
        |value, _options| {
            if let Some(password) = value.as_str() {
                if password.len() < 8 {
                    return Err(rustf::Error::validation("Password must be at least 8 characters"));
                }
                if !password.chars().any(|c| c.is_uppercase()) {
                    return Err(rustf::Error::validation("Password must contain at least one uppercase letter"));
                }
                if !password.chars().any(|c| c.is_lowercase()) {
                    return Err(rustf::Error::validation("Password must contain at least one lowercase letter"));
                }
                if !password.chars().any(|c| c.is_numeric()) {
                    return Err(rustf::Error::validation("Password must contain at least one number"));
                }
            }
            Ok(())
        },
    );
}
*/
