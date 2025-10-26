//! Encoding and decoding utilities for RustF framework
//!
//! This module provides functions for URL encoding/decoding, Base64 encoding/decoding,
//! HTML entity encoding, and other common encoding operations used in web development.

use crate::error::{Error, Result};
use std::collections::HashMap;

/// URL encode a string using percent-encoding
///
/// Encodes unsafe characters in URLs using percent-encoding (RFC 3986).
/// This is useful for encoding query parameters and URL components.
///
/// # Arguments
/// * `input` - String to URL encode
///
/// # Example
/// ```rust,ignore
/// let encoded = encode("hello world");
/// assert_eq!(encoded, "hello%20world");
/// ```
pub fn encode(input: &str) -> String {
    percent_encoding::utf8_percent_encode(input, percent_encoding::NON_ALPHANUMERIC).to_string()
}

/// URL encode a string with custom reserved characters
///
/// # Arguments
/// * `input` - String to URL encode
/// * `safe_chars` - Characters that should not be encoded
///
/// # Example
/// ```rust,ignore
/// let encoded = encode_with_safe("hello/world", "/");
/// assert_eq!(encoded, "hello/world");
/// ```
pub fn encode_with_safe(input: &str, safe_chars: &str) -> String {
    // NOTE: Custom safe character set implementation planned for future release
    // Currently uses standard URL encoding regardless of safe_chars parameter
    let _ = safe_chars; // Suppress unused variable warning
    encode(input)
}

/// URL decode a percent-encoded string
///
/// Decodes percent-encoded characters in URLs back to their original form.
///
/// # Arguments
/// * `input` - String to URL decode
///
/// # Example
/// ```rust,ignore
/// let decoded = decode("hello%20world").unwrap();
/// assert_eq!(decoded, "hello world");
/// ```
pub fn decode(input: &str) -> Result<String> {
    percent_encoding::percent_decode_str(input)
        .decode_utf8()
        .map(|s| s.to_string())
        .map_err(|e| Error::template(format!("URL decode error: {}", e)))
}

/// Base64 encode a string
///
/// Encodes a string using standard Base64 encoding.
///
/// # Arguments
/// * `input` - String to Base64 encode
///
/// # Example
/// ```rust,ignore
/// let encoded = base64_encode("hello");
/// assert_eq!(encoded, "aGVsbG8=");
/// ```
pub fn btoa(input: &str) -> String {
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, input.as_bytes())
}

/// Base64 encode bytes
///
/// # Arguments
/// * `input` - Bytes to Base64 encode
///
/// # Example
/// ```rust,ignore
/// let encoded = btoa_bytes(&[104, 101, 108, 108, 111]);
/// assert_eq!(encoded, "aGVsbG8=");
/// ```
pub fn btoa_bytes(input: &[u8]) -> String {
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, input)
}

/// Base64 decode a string
///
/// Decodes a Base64 encoded string back to its original form.
///
/// # Arguments
/// * `input` - Base64 string to decode
///
/// # Example
/// ```rust,ignore
/// let decoded = atob("aGVsbG8=").unwrap();
/// assert_eq!(decoded, "hello");
/// ```
pub fn atob(input: &str) -> Result<String> {
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, input)
        .map_err(|e| Error::template(format!("Base64 decode error: {}", e)))?;

    String::from_utf8(bytes).map_err(|e| Error::template(format!("UTF-8 decode error: {}", e)))
}

/// Base64 decode to bytes
///
/// # Arguments
/// * `input` - Base64 string to decode
///
/// # Example
/// ```rust,ignore
/// let bytes = base64_decode_bytes("aGVsbG8=").unwrap();
/// assert_eq!(bytes, vec![104, 101, 108, 108, 111]);
/// ```
pub fn base64_decode_bytes(input: &str) -> Result<Vec<u8>> {
    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, input)
        .map_err(|e| Error::template(format!("Base64 decode error: {}", e)))
}

/// URL-safe Base64 encode (no padding)
///
/// Uses URL-safe Base64 encoding without padding, suitable for URLs and tokens.
///
/// # Arguments
/// * `input` - String to encode
///
/// # Example
/// ```rust,ignore
/// let encoded = base64_url_encode("hello world");
/// println!("URL-safe: {}", encoded);
/// ```
pub fn base64_url_encode(input: &str) -> String {
    base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        input.as_bytes(),
    )
}

/// URL-safe Base64 decode
///
/// # Arguments
/// * `input` - URL-safe Base64 string to decode
///
/// # Example
/// ```rust,ignore
/// let decoded = base64_url_decode("aGVsbG8gd29ybGQ").unwrap();
/// assert_eq!(decoded, "hello world");
/// ```
pub fn base64_url_decode(input: &str) -> Result<String> {
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, input)
        .map_err(|e| Error::template(format!("Base64 URL decode error: {}", e)))?;

    String::from_utf8(bytes).map_err(|e| Error::template(format!("UTF-8 decode error: {}", e)))
}

/// HTML entity encode a string
///
/// Encodes HTML special characters to their entity equivalents.
/// This is useful for preventing XSS attacks.
///
/// # Arguments
/// * `input` - String to HTML encode
///
/// # Example
/// ```rust,ignore
/// let encoded = html_encode("<script>alert('xss')</script>");
/// assert_eq!(encoded, "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
/// ```
pub fn html_encode(input: &str) -> String {
    input
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&#x27;")
        .replace("/", "&#x2F;")
}

/// HTML entity decode a string
///
/// Decodes HTML entities back to their original characters.
///
/// # Arguments
/// * `input` - HTML encoded string to decode
///
/// # Example
/// ```rust,ignore
/// let decoded = html_decode("&lt;div&gt;Hello&lt;/div&gt;");
/// assert_eq!(decoded, "<div>Hello</div>");
/// ```
pub fn html_decode(input: &str) -> String {
    let mut result = input.to_string();

    // Common HTML entities
    let entities = [
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&#x27;", "'"),
        ("&#39;", "'"),
        ("&#x2F;", "/"),
        ("&#47;", "/"),
        ("&nbsp;", " "),
    ];

    for (entity, character) in &entities {
        result = result.replace(entity, character);
    }

    result
}

/// JSON encode a string (escape for JSON)
///
/// Escapes characters that need to be escaped in JSON strings.
///
/// # Arguments
/// * `input` - String to JSON encode
///
/// # Example
/// ```rust,ignore
/// let encoded = json_encode("Hello \"World\"");
/// assert_eq!(encoded, "Hello \\\"World\\\"");
/// ```
pub fn json_encode(input: &str) -> String {
    input
        .replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
        .replace("\u{08}", "\\b")
        .replace("\u{0C}", "\\f")
}

/// Hex encode bytes to hexadecimal string
///
/// # Arguments
/// * `input` - Bytes to encode as hex
///
/// # Example
/// ```rust,ignore
/// let hex = hex_encode(&[255, 0, 128]);
/// assert_eq!(hex, "ff0080");
/// ```
pub fn hex_encode(input: &[u8]) -> String {
    input.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Hex decode hexadecimal string to bytes
///
/// # Arguments
/// * `input` - Hexadecimal string to decode
///
/// # Example
/// ```rust,ignore
/// let bytes = hex_decode("ff0080").unwrap();
/// assert_eq!(bytes, vec![255, 0, 128]);
/// ```
pub fn hex_decode(input: &str) -> Result<Vec<u8>> {
    if input.len() % 2 != 0 {
        return Err(Error::template(
            "Hex string must have even length".to_string(),
        ));
    }

    let mut result = Vec::new();
    for chunk in input.as_bytes().chunks(2) {
        let hex_str = std::str::from_utf8(chunk)
            .map_err(|e| Error::template(format!("Invalid UTF-8 in hex string: {}", e)))?;
        let byte = u8::from_str_radix(hex_str, 16)
            .map_err(|e| Error::template(format!("Invalid hex character: {}", e)))?;
        result.push(byte);
    }

    Ok(result)
}

/// Query string encode a hash map of parameters
///
/// # Arguments
/// * `params` - HashMap of parameters to encode
///
/// # Example
/// ```rust,ignore
/// let mut params = HashMap::new();
/// params.insert("name", "John Doe");
/// params.insert("age", "30");
/// let query = query_encode(&params);
/// // Result: "name=John%20Doe&age=30" (order may vary)
/// ```
pub fn query_encode(params: &HashMap<&str, &str>) -> String {
    params
        .iter()
        .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

/// Query string decode to hash map
///
/// # Arguments
/// * `query` - Query string to decode
///
/// # Example
/// ```rust,ignore
/// let params = query_decode("name=John%20Doe&age=30").unwrap();
/// assert_eq!(params.get("name"), Some(&"John Doe".to_string()));
/// ```
pub fn query_decode(query: &str) -> Result<HashMap<String, String>> {
    let mut params = HashMap::new();

    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }

        let parts: Vec<&str> = pair.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(Error::template(format!(
                "Invalid query parameter: {}",
                pair
            )));
        }

        let key = decode(parts[0])?;
        let value = decode(parts[1])?;
        params.insert(key, value);
    }

    Ok(params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encoding() {
        assert_eq!(encode("hello world"), "hello%20world");
        assert_eq!(encode("a+b=c"), "a%2Bb%3Dc");
        assert_eq!(encode("café"), "caf%C3%A9");

        let decoded = decode("hello%20world").unwrap();
        assert_eq!(decoded, "hello world");

        let decoded = decode("caf%C3%A9").unwrap();
        assert_eq!(decoded, "café");
    }

    #[test]
    fn test_base64_encoding() {
        assert_eq!(btoa("hello"), "aGVsbG8=");
        assert_eq!(btoa(""), "");

        let decoded = atob("aGVsbG8=").unwrap();
        assert_eq!(decoded, "hello");

        let decoded = atob("").unwrap();
        assert_eq!(decoded, "");
    }

    #[test]
    fn test_base64_url_encoding() {
        let encoded = btoa("hello world");
        let decoded = atob(&encoded).unwrap();
        assert_eq!(decoded, "hello world");
    }

    #[test]
    fn test_html_encoding() {
        let encoded = html_encode("<script>alert('xss')</script>");
        assert_eq!(
            encoded,
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;&#x2F;script&gt;"
        );

        let decoded = html_decode("&lt;div&gt;Hello&lt;/div&gt;");
        assert_eq!(decoded, "<div>Hello</div>");
    }

    #[test]
    fn test_json_encoding() {
        let encoded = json_encode("Hello \"World\"\nNew line");
        assert_eq!(encoded, "Hello \\\"World\\\"\\nNew line");
    }

    #[test]
    fn test_hex_encoding() {
        let hex = hex_encode(&[255, 0, 128]);
        assert_eq!(hex, "ff0080");

        let bytes = hex_decode("ff0080").unwrap();
        assert_eq!(bytes, vec![255, 0, 128]);

        // Test error cases
        assert!(hex_decode("ff008").is_err()); // Odd length
        assert!(hex_decode("gg0080").is_err()); // Invalid hex
    }

    #[test]
    fn test_query_encoding() {
        let mut params = HashMap::new();
        params.insert("name", "John Doe");
        params.insert("age", "30");

        let query = query_encode(&params);
        assert!(query.contains("name=John%20Doe"));
        assert!(query.contains("age=30"));
        assert!(query.contains("&"));

        let decoded = query_decode(&query).unwrap();
        assert_eq!(decoded.get("name").unwrap(), "John Doe");
        assert_eq!(decoded.get("age").unwrap(), "30");
    }

    #[test]
    fn test_base64_bytes() {
        let bytes = vec![104, 101, 108, 108, 111];
        let encoded = btoa_bytes(&bytes);
        assert_eq!(encoded, "aGVsbG8=");

        let decoded = base64_decode_bytes(&encoded).unwrap();
        assert_eq!(decoded, bytes);
    }
}
