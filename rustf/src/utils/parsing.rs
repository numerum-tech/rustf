//! Safe parsing utilities for RustF framework
//!
//! This module provides safe parsing functions that return default values
//! instead of panicking when parsing fails. This is particularly useful
//! for parsing user input, configuration values, and form data.

/// Parse a string to boolean with default value
///
/// Recognizes various representations of true/false values.
/// Returns the default value if parsing fails.
///
/// # Arguments
/// * `value` - String to parse
/// * `default` - Default value if parsing fails
///
/// Recognized true values: "true", "1", "yes", "on", "y", "t"
/// Recognized false values: "false", "0", "no", "off", "n", "f"
///
/// # Example
/// ```rust,ignore
/// assert_eq!(bool("true", false), true);
/// assert_eq!(bool("yes", false), true);
/// assert_eq!(bool("1", false), true);
/// assert_eq!(bool("false", true), false);
/// assert_eq!(bool("invalid", true), true); // returns default
/// ```
pub fn bool(value: &str, default: bool) -> bool {
    let trimmed = value.trim().to_lowercase();

    match trimmed.as_str() {
        "true" | "1" | "yes" | "on" | "y" | "t" => true,
        "false" | "0" | "no" | "off" | "n" | "f" => false,
        _ => default,
    }
}

/// Parse a string to integer with default value
///
/// Attempts to parse the string as an i64 integer.
/// Returns the default value if parsing fails.
///
/// # Arguments
/// * `value` - String to parse
/// * `default` - Default value if parsing fails
///
/// # Example
/// ```rust,ignore
/// assert_eq!(int("123", 0), 123);
/// assert_eq!(int("-456", 0), -456);
/// assert_eq!(int("invalid", 42), 42); // returns default
/// ```
pub fn int(value: &str, default: i64) -> i64 {
    value.trim().parse().unwrap_or(default)
}

/// Parse a string to unsigned integer with default value
///
/// # Arguments
/// * `value` - String to parse
/// * `default` - Default value if parsing fails
///
/// # Example
/// ```rust,ignore
/// assert_eq!(parse_unsigned_integer("123", 0), 123);
/// assert_eq!(parse_unsigned_integer("invalid", 42), 42);
/// ```
pub fn parse_unsigned_integer(value: &str, default: u64) -> u64 {
    value.trim().parse().unwrap_or(default)
}

/// Parse a string to 32-bit integer with default value
///
/// # Arguments
/// * `value` - String to parse
/// * `default` - Default value if parsing fails
///
/// # Example
/// ```rust,ignore
/// assert_eq!(parse_i32("123", 0), 123);
/// assert_eq!(parse_i32("invalid", 42), 42);
/// ```
pub fn parse_i32(value: &str, default: i32) -> i32 {
    value.trim().parse().unwrap_or(default)
}

/// Parse a string to floating-point number with default value
///
/// Attempts to parse the string as an f64 floating-point number.
/// Returns the default value if parsing fails.
///
/// # Arguments
/// * `value` - String to parse
/// * `default` - Default value if parsing fails
///
/// # Example
/// ```rust,ignore
/// assert_eq!(float("123.45", 0.0), 123.45);
/// assert_eq!(float("-67.89", 0.0), -67.89);
/// assert_eq!(float("invalid", 3.14), 3.14); // returns default
/// ```
pub fn float(value: &str, default: f64) -> f64 {
    value.trim().parse().unwrap_or(default)
}

/// Parse a string to 32-bit floating-point number with default value
///
/// # Arguments
/// * `value` - String to parse
/// * `default` - Default value if parsing fails
///
/// # Example
/// ```rust,ignore
/// assert_eq!(parse_f32("123.45", 0.0), 123.45);
/// assert_eq!(parse_f32("invalid", 3.14), 3.14);
/// ```
pub fn parse_f32(value: &str, default: f32) -> f32 {
    value.trim().parse().unwrap_or(default)
}

/// Parse a string to enum variant with default value
///
/// Generic function to parse string values into enum variants.
/// Useful for parsing configuration values and form inputs.
///
/// # Arguments
/// * `value` - String to parse
/// * `default` - Default enum variant if parsing fails
///
/// # Example
/// ```rust,ignore
/// #[derive(Debug, PartialEq, Clone)]
/// enum Color { Red, Green, Blue }
///
/// impl std::str::FromStr for Color {
///     type Err = ();
///     fn from_str(s: &str) -> Result<Self, Self::Err> {
///         match s.to_lowercase().as_str() {
///             "red" => Ok(Color::Red),
///             "green" => Ok(Color::Green),
///             "blue" => Ok(Color::Blue),
///             _ => Err(()),
///         }
///     }
/// }
///
/// let color = parse_enum("red", Color::Blue);
/// assert_eq!(color, Color::Red);
/// ```
pub fn parse_enum<T>(value: &str, default: T) -> T
where
    T: std::str::FromStr + Clone,
{
    value.trim().parse().unwrap_or(default)
}

/// Parse a comma-separated string into a vector of strings
///
/// Splits the input by commas and trims whitespace from each item.
/// Empty items are filtered out.
///
/// # Arguments
/// * `value` - Comma-separated string to parse
///
/// # Example
/// ```rust,ignore
/// let items = parse_comma_separated("apple, banana, cherry");
/// assert_eq!(items, vec!["apple", "banana", "cherry"]);
///
/// let items = parse_comma_separated("one,two,,three,");
/// assert_eq!(items, vec!["one", "two", "three"]);
/// ```
pub fn parse_comma_separated(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Parse a comma-separated string into a vector of integers
///
/// # Arguments
/// * `value` - Comma-separated string of integers
/// * `default_item` - Default value for items that fail to parse
///
/// # Example
/// ```rust,ignore
/// let numbers = parse_comma_separated_integers("1, 2, 3", 0);
/// assert_eq!(numbers, vec![1, 2, 3]);
///
/// let numbers = parse_comma_separated_integers("1, invalid, 3", 0);
/// assert_eq!(numbers, vec![1, 0, 3]);
/// ```
pub fn parse_comma_separated_integers(value: &str, default_item: i64) -> Vec<i64> {
    value
        .split(',')
        .map(|s| int(s.trim(), default_item))
        .collect()
}

/// Parse a key-value pair string
///
/// Parses strings in the format "key=value" or "key:value".
///
/// # Arguments
/// * `value` - String containing key-value pair
/// * `separator` - Character used to separate key and value ("=" or ":")
///
/// # Example
/// ```rust,ignore
/// let (key, value) = parse_key_value("username=john", "=").unwrap();
/// assert_eq!(key, "username");
/// assert_eq!(value, "john");
///
/// let (key, value) = parse_key_value("port:8080", ":").unwrap();
/// assert_eq!(key, "port");
/// assert_eq!(value, "8080");
/// ```
pub fn parse_key_value(value: &str, separator: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = value.splitn(2, separator).collect();
    if parts.len() == 2 {
        Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
    } else {
        None
    }
}

/// Parse a duration string into seconds
///
/// Supports various time units: s, m, h, d (seconds, minutes, hours, days).
///
/// # Arguments
/// * `value` - Duration string (e.g., "30s", "5m", "2h", "1d")
/// * `default` - Default value in seconds if parsing fails
///
/// # Example
/// ```rust,ignore
/// assert_eq!(parse_duration("30s", 0), 30);
/// assert_eq!(parse_duration("5m", 0), 300);
/// assert_eq!(parse_duration("2h", 0), 7200);
/// assert_eq!(parse_duration("1d", 0), 86400);
/// assert_eq!(parse_duration("invalid", 60), 60);
/// ```
pub fn parse_duration(value: &str, default: u64) -> u64 {
    let trimmed = value.trim().to_lowercase();

    if let Some(num_str) = trimmed.strip_suffix('s') {
        return parse_unsigned_integer(num_str, default);
    }

    if let Some(num_str) = trimmed.strip_suffix('m') {
        let minutes = parse_unsigned_integer(num_str, 0);
        if minutes > 0 {
            return minutes * 60;
        }
    }

    if let Some(num_str) = trimmed.strip_suffix('h') {
        let hours = parse_unsigned_integer(num_str, 0);
        if hours > 0 {
            return hours * 3600;
        }
    }

    if let Some(num_str) = trimmed.strip_suffix('d') {
        let days = parse_unsigned_integer(num_str, 0);
        if days > 0 {
            return days * 86400;
        }
    }

    // Try parsing as plain number (assume seconds)
    parse_unsigned_integer(&trimmed, default)
}

/// Parse a size string into bytes
///
/// Supports various size units: B, KB, MB, GB, TB.
///
/// # Arguments
/// * `value` - Size string (e.g., "1024B", "1KB", "5MB", "2GB")
/// * `default` - Default value in bytes if parsing fails
///
/// # Example
/// ```rust,ignore
/// assert_eq!(parse_size("1024B", 0), 1024);
/// assert_eq!(parse_size("1KB", 0), 1024);
/// assert_eq!(parse_size("5MB", 0), 5_242_880);
/// assert_eq!(parse_size("2GB", 0), 2_147_483_648);
/// ```
pub fn parse_size(value: &str, default: u64) -> u64 {
    let trimmed = value.trim().to_uppercase();

    if let Some(num_str) = trimmed.strip_suffix("TB") {
        let tb = parse_unsigned_integer(num_str, 0);
        if tb > 0 {
            return tb * 1_099_511_627_776;
        }
    }

    if let Some(num_str) = trimmed.strip_suffix("GB") {
        let gb = parse_unsigned_integer(num_str, 0);
        if gb > 0 {
            return gb * 1_073_741_824;
        }
    }

    if let Some(num_str) = trimmed.strip_suffix("MB") {
        let mb = parse_unsigned_integer(num_str, 0);
        if mb > 0 {
            return mb * 1_048_576;
        }
    }

    if let Some(num_str) = trimmed.strip_suffix("KB") {
        let kb = parse_unsigned_integer(num_str, 0);
        if kb > 0 {
            return kb * 1024;
        }
    }

    if let Some(num_str) = trimmed.strip_suffix("B") {
        return parse_unsigned_integer(num_str, default);
    }

    // Try parsing as plain number (assume bytes)
    parse_unsigned_integer(&trimmed, default)
}

/// Parse a percentage string to float
///
/// # Arguments
/// * `value` - Percentage string (e.g., "75%", "100%")
/// * `default` - Default value if parsing fails
///
/// # Example
/// ```rust,ignore
/// assert_eq!(parse_percentage("75%", 0.0), 0.75);
/// assert_eq!(parse_percentage("100%", 0.0), 1.0);
/// assert_eq!(parse_percentage("invalid", 0.5), 0.5);
/// ```
pub fn parse_percentage(value: &str, default: f64) -> f64 {
    let trimmed = value.trim();

    if let Some(num_str) = trimmed.strip_suffix('%') {
        let percentage = float(num_str, -1.0);
        if percentage >= 0.0 {
            return percentage / 100.0;
        }
    }

    default
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool() {
        // True values
        assert_eq!(bool("true", false), true);
        assert_eq!(bool("TRUE", false), true);
        assert_eq!(bool("1", false), true);
        assert_eq!(bool("yes", false), true);
        assert_eq!(bool("on", false), true);
        assert_eq!(bool("y", false), true);
        assert_eq!(bool("t", false), true);

        // False values
        assert_eq!(bool("false", true), false);
        assert_eq!(bool("FALSE", true), false);
        assert_eq!(bool("0", true), false);
        assert_eq!(bool("no", true), false);
        assert_eq!(bool("off", true), false);
        assert_eq!(bool("n", true), false);
        assert_eq!(bool("f", true), false);

        // Default values
        assert_eq!(bool("invalid", true), true);
        assert_eq!(bool("invalid", false), false);
        assert_eq!(bool("", true), true);
    }

    #[test]
    fn test_int() {
        assert_eq!(int("123", 0), 123);
        assert_eq!(int("-456", 0), -456);
        assert_eq!(int("  789  ", 0), 789);
        assert_eq!(int("invalid", 42), 42);
        assert_eq!(int("", 100), 100);
    }

    #[test]
    fn test_float() {
        assert_eq!(float("123.45", 0.0), 123.45);
        assert_eq!(float("-67.89", 0.0), -67.89);
        assert_eq!(float("  3.14  ", 0.0), 3.14);
        assert_eq!(float("invalid", 2.71), 2.71);
        assert_eq!(float("", 1.0), 1.0);
    }

    #[test]
    fn test_parse_comma_separated() {
        let items = parse_comma_separated("apple, banana, cherry");
        assert_eq!(items, vec!["apple", "banana", "cherry"]);

        let items = parse_comma_separated("one,two,,three,");
        assert_eq!(items, vec!["one", "two", "three"]);

        let items = parse_comma_separated("");
        assert!(items.is_empty());

        let items = parse_comma_separated("single");
        assert_eq!(items, vec!["single"]);
    }

    #[test]
    fn test_parse_comma_separated_integers() {
        let numbers = parse_comma_separated_integers("1, 2, 3", 0);
        assert_eq!(numbers, vec![1, 2, 3]);

        let numbers = parse_comma_separated_integers("1, invalid, 3", 99);
        assert_eq!(numbers, vec![1, 99, 3]);

        let numbers = parse_comma_separated_integers("", 0);
        assert_eq!(numbers, vec![0]);
    }

    #[test]
    fn test_parse_key_value() {
        let (key, value) = parse_key_value("username=john", "=").unwrap();
        assert_eq!(key, "username");
        assert_eq!(value, "john");

        let (key, value) = parse_key_value("port:8080", ":").unwrap();
        assert_eq!(key, "port");
        assert_eq!(value, "8080");

        let (key, value) = parse_key_value("config = debug mode ", "=").unwrap();
        assert_eq!(key, "config");
        assert_eq!(value, "debug mode");

        assert!(parse_key_value("invalid", "=").is_none());
        assert!(parse_key_value("", "=").is_none());
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s", 0), 30);
        assert_eq!(parse_duration("5m", 0), 300);
        assert_eq!(parse_duration("2h", 0), 7200);
        assert_eq!(parse_duration("1d", 0), 86400);
        assert_eq!(parse_duration("60", 0), 60); // Plain number
        assert_eq!(parse_duration("invalid", 123), 123);
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("1024B", 0), 1024);
        assert_eq!(parse_size("1KB", 0), 1024);
        assert_eq!(parse_size("1MB", 0), 1_048_576);
        assert_eq!(parse_size("1GB", 0), 1_073_741_824);
        assert_eq!(parse_size("2048", 0), 2048); // Plain number
        assert_eq!(parse_size("invalid", 1000), 1000);
    }

    #[test]
    fn test_parse_percentage() {
        assert_eq!(parse_percentage("75%", 0.0), 0.75);
        assert_eq!(parse_percentage("100%", 0.0), 1.0);
        assert_eq!(parse_percentage("0%", 1.0), 0.0);
        assert_eq!(parse_percentage("invalid", 0.5), 0.5);
        assert_eq!(parse_percentage("50", 0.0), 0.0); // No % sign
    }
}
