//! Random generation utilities for RustF framework
//!
//! This module provides functions for generating random values commonly used
//! in web development such as UUIDs, random strings, and random numbers.

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use uuid::Uuid;

/// Generate a new UUID/GUID string
///
/// Returns a UUID v4 as a lowercase string without hyphens by default.
/// This format is commonly used for database IDs and session tokens.
///
/// # Example
/// ```rust,ignore
/// let id = guid();
/// println!("Generated ID: {}", id); // e.g., "a1b2c3d4e5f67890abcdef1234567890"
/// ```
pub fn guid() -> String {
    Uuid::new_v4().simple().to_string()
}

/// Generate a new UUID/GUID string with hyphens
///
/// Returns a UUID v4 as a lowercase string with standard hyphen formatting.
///
/// # Example
/// ```rust,ignore
/// let id = generate_guid_with_hyphens();
/// println!("Generated ID: {}", id); // e.g., "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
/// ```
pub fn generate_guid_with_hyphens() -> String {
    Uuid::new_v4().to_string()
}

/// Generate a random alphanumeric string of specified length
///
/// The generated string contains only uppercase and lowercase letters and digits (A-Z, a-z, 0-9).
/// This is useful for generating tokens, passwords, and other random identifiers.
///
/// # Arguments
/// * `length` - The desired length of the random string
///
/// # Example
/// ```rust,ignore
/// let token = string(32);
/// println!("Random token: {}", token); // e.g., "Kj8mN2pQ5rT9wX3yZ7aB4cD6fG1hI0kL"
/// ```
pub fn string(length: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// Generate a random string with custom character set
///
/// # Arguments
/// * `length` - The desired length of the random string
/// * `charset` - String containing all allowed characters
///
/// # Example
/// ```rust,ignore
/// let code = generate_random_string_with_charset(6, "0123456789");
/// println!("Numeric code: {}", code); // e.g., "482957"
/// ```
pub fn generate_random_string_with_charset(length: usize, charset: &str) -> String {
    let chars: Vec<char> = charset.chars().collect();
    if chars.is_empty() {
        return String::new();
    }

    let mut rng = thread_rng();
    (0..length)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}

/// Generate a random number between min and max (inclusive)
///
/// # Arguments
/// * `min` - Minimum value (inclusive)
/// * `max` - Maximum value (inclusive)
///
/// # Example
/// ```rust,ignore
/// let dice_roll = number(1, 6);
/// println!("Dice roll: {}", dice_roll); // e.g., 4
/// ```
pub fn number(min: i64, max: i64) -> i64 {
    if min > max {
        return min;
    }
    thread_rng().gen_range(min..=max)
}

/// Generate a random floating-point number between min and max
///
/// # Arguments
/// * `min` - Minimum value (inclusive)
/// * `max` - Maximum value (exclusive)
///
/// # Example
/// ```rust,ignore
/// let random_price = generate_random_float(10.0, 100.0);
/// println!("Random price: ${:.2}", random_price);
/// ```
pub fn generate_random_float(min: f64, max: f64) -> f64 {
    if min >= max {
        return min;
    }
    thread_rng().gen_range(min..max)
}

/// Generate a random text string with readable words
///
/// This creates pseudo-random text that resembles natural language,
/// useful for generating test data or placeholder content.
///
/// # Arguments
/// * `word_count` - Number of words to generate
///
/// # Example
/// ```rust,ignore
/// let text = generate_random_text(5);
/// println!("Random text: {}", text); // e.g., "lorem ipsum dolor sit amet"
/// ```
pub fn generate_random_text(word_count: usize) -> String {
    const WORDS: &[&str] = &[
        "lorem",
        "ipsum",
        "dolor",
        "sit",
        "amet",
        "consectetur",
        "adipiscing",
        "elit",
        "sed",
        "do",
        "eiusmod",
        "tempor",
        "incididunt",
        "ut",
        "labore",
        "et",
        "dolore",
        "magna",
        "aliqua",
        "enim",
        "ad",
        "minim",
        "veniam",
        "quis",
        "nostrud",
        "exercitation",
        "ullamco",
        "laboris",
        "nisi",
        "aliquip",
        "ex",
        "ea",
        "commodo",
        "consequat",
        "duis",
        "aute",
        "irure",
        "in",
        "reprehenderit",
        "voluptate",
        "velit",
        "esse",
        "cillum",
        "fugiat",
        "nulla",
        "pariatur",
        "excepteur",
        "sint",
        "occaecat",
        "cupidatat",
        "non",
        "proident",
        "sunt",
        "culpa",
        "qui",
        "officia",
        "deserunt",
        "mollit",
        "anim",
        "id",
        "est",
        "laborum",
    ];

    let mut rng = thread_rng();
    (0..word_count)
        .map(|_| WORDS[rng.gen_range(0..WORDS.len())])
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Generate a secure random token suitable for authentication
///
/// This generates a cryptographically secure random token using
/// URL-safe base64 encoding. Suitable for session tokens, API keys, etc.
///
/// # Arguments
/// * `byte_length` - Number of random bytes to generate (output will be longer due to encoding)
///
/// # Example
/// ```rust,ignore
/// let token = generate_secure_token(32);
/// println!("Secure token: {}", token);
/// ```
pub fn generate_secure_token(byte_length: usize) -> String {
    use base64::{engine::general_purpose, Engine as _};

    let mut bytes = vec![0u8; byte_length];
    thread_rng().fill(&mut bytes[..]);
    general_purpose::URL_SAFE_NO_PAD.encode(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_guid() {
        let guid1 = guid();
        let guid2 = guid();

        // Should be 32 characters long (UUID without hyphens)
        assert_eq!(guid1.len(), 32);
        assert_eq!(guid2.len(), 32);

        // Should be different
        assert_ne!(guid1, guid2);

        // Should contain only valid hex characters
        assert!(guid1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_guid_with_hyphens() {
        let guid = generate_guid_with_hyphens();

        // Should be 36 characters long (UUID with hyphens)
        assert_eq!(guid.len(), 36);

        // Should contain hyphens in correct positions
        assert_eq!(guid.chars().nth(8), Some('-'));
        assert_eq!(guid.chars().nth(13), Some('-'));
        assert_eq!(guid.chars().nth(18), Some('-'));
        assert_eq!(guid.chars().nth(23), Some('-'));
    }

    #[test]
    fn test_generate_random_string() {
        let str1 = string(10);
        let str2 = string(10);

        // Should be correct length
        assert_eq!(str1.len(), 10);
        assert_eq!(str2.len(), 10);

        // Should be different
        assert_ne!(str1, str2);

        // Should contain only alphanumeric characters
        assert!(str1.chars().all(|c| c.is_alphanumeric()));
        assert!(str2.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_generate_random_string_with_charset() {
        let numbers = generate_random_string_with_charset(8, "0123456789");

        // Should be correct length
        assert_eq!(numbers.len(), 8);

        // Should contain only digits
        assert!(numbers.chars().all(|c| c.is_ascii_digit()));

        // Test with empty charset
        let empty = generate_random_string_with_charset(5, "");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_generate_random_number() {
        let num = number(1, 10);
        assert!(num >= 1 && num <= 10);

        // Test single value range
        let single = number(5, 5);
        assert_eq!(single, 5);

        // Test inverted range
        let inverted = number(10, 5);
        assert_eq!(inverted, 10); // Should return min when min > max
    }

    #[test]
    fn test_generate_random_float() {
        let num = generate_random_float(1.0, 10.0);
        assert!(num >= 1.0 && num < 10.0);

        // Test inverted range
        let inverted = generate_random_float(10.0, 5.0);
        assert_eq!(inverted, 10.0); // Should return min when min >= max
    }

    #[test]
    fn test_generate_random_text() {
        let text = generate_random_text(5);
        let words: Vec<&str> = text.split_whitespace().collect();

        // Should have correct number of words
        assert_eq!(words.len(), 5);

        // All words should be from our word list
        const WORDS: &[&str] = &[
            "lorem",
            "ipsum",
            "dolor",
            "sit",
            "amet",
            "consectetur",
            "adipiscing",
            "elit",
            "sed",
            "do",
            "eiusmod",
            "tempor",
            "incididunt",
            "ut",
            "labore",
            "et",
            "dolore",
            "magna",
            "aliqua",
            "enim",
            "ad",
            "minim",
            "veniam",
            "quis",
            "nostrud",
            "exercitation",
            "ullamco",
            "laboris",
            "nisi",
            "aliquip",
            "ex",
            "ea",
            "commodo",
            "consequat",
            "duis",
            "aute",
            "irure",
            "in",
            "reprehenderit",
            "voluptate",
            "velit",
            "esse",
            "cillum",
            "fugiat",
            "nulla",
            "pariatur",
            "excepteur",
            "sint",
            "occaecat",
            "cupidatat",
            "non",
            "proident",
            "sunt",
            "culpa",
            "qui",
            "officia",
            "deserunt",
            "mollit",
            "anim",
            "id",
            "est",
            "laborum",
        ];

        for word in words {
            assert!(WORDS.contains(&word));
        }
    }

    #[test]
    fn test_generate_secure_token() {
        let token1 = generate_secure_token(32);
        let token2 = generate_secure_token(32);

        // Should be different
        assert_ne!(token1, token2);

        // Should be URL-safe base64 (no padding, no special chars)
        assert!(token1
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_'));
        assert!(token2
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_'));
    }
}
