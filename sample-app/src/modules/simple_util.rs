//! Simple utility module that does NOT implement SharedModule
//!
//! This demonstrates the design principle: not everything needs to be a singleton.
//! Simple utilities can be used directly via import without registration.
//! Only services that need to be singletons should implement SharedModule.

/// A simple utility for string manipulation
pub struct StringUtils;

impl StringUtils {
    /// Convert string to uppercase
    pub fn to_upper(s: &str) -> String {
        s.to_uppercase()
    }

    /// Convert string to lowercase
    pub fn to_lower(s: &str) -> String {
        s.to_lowercase()
    }

    /// Capitalize first letter
    pub fn capitalize(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().chain(chars).collect(),
        }
    }

    /// Reverse string
    pub fn reverse(s: &str) -> String {
        s.chars().rev().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_utils_to_upper() {
        assert_eq!(StringUtils::to_upper("hello"), "HELLO");
    }

    #[test]
    fn test_string_utils_capitalize() {
        assert_eq!(StringUtils::capitalize("hello"), "Hello");
    }

    #[test]
    fn test_string_utils_reverse() {
        assert_eq!(StringUtils::reverse("hello"), "olleh");
    }
}
