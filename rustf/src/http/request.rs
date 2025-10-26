use crate::error::{Error, Result};
use crate::http::files::{FileCollection, MultipartParser};
use hyper::{Body, Request as HyperRequest};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Represents form data that can be either a single value or an array of values
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FormValue {
    Single(String),
    Multiple(Vec<String>),
}

impl FormValue {
    /// Get as a single value (returns first element if array)
    pub fn as_string(&self) -> &str {
        match self {
            FormValue::Single(s) => s,
            FormValue::Multiple(v) => v.first().map(|s| s.as_str()).unwrap_or(""),
        }
    }

    /// Get as array (wraps single value in array if needed)
    pub fn as_array(&self) -> Vec<&str> {
        match self {
            FormValue::Single(s) => vec![s.as_str()],
            FormValue::Multiple(v) => v.iter().map(|s| s.as_str()).collect(),
        }
    }

    /// Check if this is an array value
    pub fn is_array(&self) -> bool {
        matches!(self, FormValue::Multiple(_))
    }
}

#[derive(Default)]
pub struct Request {
    pub method: String,
    pub uri: String,
    pub headers: HashMap<String, String>,
    pub params: HashMap<String, String>,
    pub query: HashMap<String, String>,
    body_bytes: Vec<u8>,
    files: Option<FileCollection>,
}

impl Request {
    /// Create a new Request for testing purposes
    /// This is only intended for testing and should not be used in production code
    #[doc(hidden)]
    pub fn new(method: &str, uri: &str, _version: &str) -> Self {
        Request {
            method: method.to_string(),
            uri: uri.to_string(),
            headers: HashMap::new(),
            params: HashMap::new(),
            query: HashMap::new(),
            body_bytes: Vec::new(),
            files: None,
        }
    }

    /// Set body for testing purposes
    #[doc(hidden)]
    #[cfg(test)]
    pub fn set_body(&mut self, body: Vec<u8>) {
        self.body_bytes = body;
    }

    pub async fn from_hyper(req: HyperRequest<Body>) -> Result<Self> {
        let method = req.method().to_string();
        let uri = req.uri().to_string();

        // Extract headers
        let mut headers = HashMap::new();
        for (name, value) in req.headers() {
            if let Ok(value_str) = value.to_str() {
                headers.insert(name.to_string(), value_str.to_string());
            }
        }

        // Extract query parameters
        let query = Self::parse_query(req.uri().query().unwrap_or(""));

        // Read body
        let body_bytes = hyper::body::to_bytes(req.into_body()).await?.to_vec();

        Ok(Request {
            method,
            uri,
            headers,
            params: HashMap::new(), // Will be filled by router
            query,
            body_bytes,
            files: None, // Will be parsed on demand
        })
    }

    pub fn body_as_json<T: DeserializeOwned>(&self) -> Result<T> {
        let body_str = String::from_utf8_lossy(&self.body_bytes);
        serde_json::from_str(&body_str).map_err(Error::Json)
    }

    pub fn body_as_form(&self) -> Result<HashMap<String, String>> {
        let body_str = String::from_utf8_lossy(&self.body_bytes);
        Ok(Self::parse_query(&body_str))
    }

    /// Parse form data with support for arrays (field[] syntax)
    pub fn body_as_form_data(&self) -> Result<HashMap<String, FormValue>> {
        let body_str = String::from_utf8_lossy(&self.body_bytes);
        Ok(Self::parse_query_with_arrays(&body_str))
    }

    pub fn body_as_string(&self) -> String {
        String::from_utf8_lossy(&self.body_bytes).to_string()
    }

    fn parse_query(query: &str) -> HashMap<String, String> {
        let mut result = HashMap::new();
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                // Use safe decoding that filters malicious input
                let decoded_key = urlencoding::decode(key);
                let decoded_value = urlencoding::decode(value);

                match (decoded_key, decoded_value) {
                    (Some(k), Some(v)) => {
                        result.insert(k.to_string(), v.to_string());
                    }
                    (Some(k), None) => {
                        // Value is malicious - insert empty string
                        result.insert(k.to_string(), String::new());
                    }
                    (None, Some(v)) => {
                        // Key is malicious - insert empty key with value
                        result.insert(String::new(), v.to_string());
                    }
                    (None, None) => {
                        // Both malicious - insert empty key and value
                        result.insert(String::new(), String::new());
                    }
                }
            }
        }
        result
    }

    /// Parse query/form data with support for arrays
    fn parse_query_with_arrays(query: &str) -> HashMap<String, FormValue> {
        let mut result: HashMap<String, Vec<String>> = HashMap::new();

        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                // Use safe decoding that filters malicious input
                if let (Some(decoded_key), Some(decoded_value)) =
                    (urlencoding::decode(key), urlencoding::decode(value))
                {
                    let decoded_key_str = decoded_key.to_string();
                    let decoded_value_str = decoded_value.to_string();

                    // Check if key ends with [] (array notation)
                    let actual_key = if decoded_key_str.ends_with("[]") {
                        decoded_key_str[..decoded_key_str.len() - 2].to_string()
                    } else {
                        decoded_key_str
                    };

                    result
                        .entry(actual_key)
                        .or_default()
                        .push(decoded_value_str);
                }
                // Skip pairs with invalid encoding
            }
        }

        // Convert to FormValue enum
        result
            .into_iter()
            .map(|(key, values)| {
                let form_value = if values.len() == 1 {
                    FormValue::Single(values.into_iter().next().unwrap())
                } else {
                    FormValue::Multiple(values)
                };
                (key, form_value)
            })
            .collect()
    }

    // Client information helper methods

    /// Get client IP address (supports X-Forwarded-For and X-Real-IP)
    pub fn client_ip(&self) -> String {
        // Check X-Forwarded-For header first (comma-separated list, first is client)
        if let Some(forwarded) = self.headers.get("x-forwarded-for") {
            if let Some(first_ip) = forwarded.split(',').next() {
                return first_ip.trim().to_string();
            }
        }

        // Check X-Real-IP header
        if let Some(real_ip) = self.headers.get("x-real-ip") {
            return real_ip.to_string();
        }

        // Fallback to "remote" (not available in current implementation)
        "127.0.0.1".to_string()
    }

    /// Get user agent string
    pub fn user_agent(&self) -> Option<&str> {
        self.headers.get("user-agent").map(|s| s.as_str())
    }

    /// Detect if request is from mobile device
    pub fn is_mobile(&self) -> bool {
        if let Some(ua) = self.user_agent() {
            let ua_lower = ua.to_lowercase();
            ua_lower.contains("mobile")
                || ua_lower.contains("android")
                || ua_lower.contains("iphone")
                || ua_lower.contains("ipad")
                || ua_lower.contains("blackberry")
                || ua_lower.contains("windows phone")
        } else {
            false
        }
    }

    /// Detect if request is from a bot/crawler
    pub fn is_robot(&self) -> bool {
        if let Some(ua) = self.user_agent() {
            let ua_lower = ua.to_lowercase();
            ua_lower.contains("bot")
                || ua_lower.contains("crawler")
                || ua_lower.contains("spider")
                || ua_lower.contains("scraper")
                || ua_lower.contains("googlebot")
                || ua_lower.contains("bingbot")
                || ua_lower.contains("facebookexternalhit")
                || ua_lower.contains("twitterbot")
        } else {
            false
        }
    }

    /// Check if request is HTTPS (via X-Forwarded-Proto or URI scheme)
    pub fn is_secure(&self) -> bool {
        // Check X-Forwarded-Proto header
        if let Some(proto) = self.headers.get("x-forwarded-proto") {
            return proto.to_lowercase() == "https";
        }

        // Check URI scheme
        self.uri.starts_with("https://")
    }

    /// Check if request is AJAX/XHR
    pub fn is_xhr(&self) -> bool {
        self.headers
            .get("x-requested-with")
            .map(|v| v.to_lowercase() == "xmlhttprequest")
            .unwrap_or(false)
    }

    /// Get preferred language from Accept-Language header
    pub fn language(&self) -> Option<&str> {
        if let Some(accept_lang) = self.headers.get("accept-language") {
            // Parse "en-US,en;q=0.9,fr;q=0.8" and return first language
            accept_lang
                .split(',')
                .next()
                .and_then(|lang| lang.split(';').next())
                .map(|lang| lang.trim())
        } else {
            None
        }
    }

    /// Get HTTP referrer
    pub fn referrer(&self) -> Option<&str> {
        self.headers.get("referer").map(|s| s.as_str())
    }

    // File upload handling methods

    /// Get uploaded files (Total.js: controller.files)  
    pub fn files(&mut self) -> Result<&FileCollection> {
        if self.files.is_none() {
            self.parse_files()?;
        }
        Ok(self.files.as_ref().unwrap())
    }

    /// Get a specific uploaded file by field name
    pub fn file(&mut self, field_name: &str) -> Result<Option<&crate::http::files::UploadedFile>> {
        Ok(self.files()?.get(field_name))
    }

    /// Parse multipart form data to extract files
    fn parse_files(&mut self) -> Result<()> {
        // Check if this is a multipart form
        if let Some(content_type) = self.headers.get("content-type") {
            if content_type.starts_with("multipart/form-data") {
                // Extract boundary
                if let Some(boundary) = self.extract_boundary(content_type) {
                    let (files, _form_data) = MultipartParser::parse(&self.body_bytes, &boundary)?;

                    // Merge form data into existing body parsing (for body_as_form compatibility)
                    // This is a bit hacky but maintains compatibility

                    self.files = Some(files);
                    return Ok(());
                }
            }
        }

        // Not multipart, create empty file collection
        self.files = Some(FileCollection::new());
        Ok(())
    }

    /// Extract boundary from Content-Type header
    fn extract_boundary(&self, content_type: &str) -> Option<String> {
        // Parse: multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW
        for part in content_type.split(';') {
            let part = part.trim();
            if part.starts_with("boundary=") {
                return Some(part[9..].to_string());
            }
        }
        None
    }

    // Total.js compatibility methods - Phase 1: High Priority Features

    /// Get cookie value by name (Total.js: request.cookie(name))
    pub fn cookie(&self, name: &str) -> Option<String> {
        if let Some(cookie_header) = self.headers.get("cookie") {
            Self::parse_cookies(cookie_header).get(name).cloned()
        } else {
            None
        }
    }

    /// Get host from Host header (Total.js: request.host)
    pub fn host(&self) -> Option<&str> {
        self.headers.get("host").map(|s| s.as_str())
    }

    /// Get hostname with optional path (Total.js: request.hostname([path]))
    pub fn hostname(&self, path: Option<&str>) -> String {
        let host = self.host().unwrap_or("localhost");
        let scheme = if self.is_secure() { "https" } else { "http" };

        if let Some(path) = path {
            let path = if path.starts_with('/') {
                path
            } else {
                &format!("/{}", path)
            };
            format!("{}://{}{}", scheme, host, path)
        } else {
            format!("{}://{}", scheme, host)
        }
    }

    /// Get request path from URI (Total.js: request.path)
    pub fn path(&self) -> &str {
        // Extract path from URI, handling both full URLs and paths
        if let Some(path_start) = self.uri.find("://") {
            // Full URL: extract path after domain
            let after_scheme = &self.uri[path_start + 3..];
            if let Some(path_start) = after_scheme.find('/') {
                let path_with_query = &after_scheme[path_start..];
                // Remove query string if present
                if let Some(query_start) = path_with_query.find('?') {
                    &path_with_query[..query_start]
                } else {
                    path_with_query
                }
            } else {
                "/"
            }
        } else {
            // Relative path: remove query string if present
            if let Some(query_start) = self.uri.find('?') {
                &self.uri[..query_start]
            } else {
                &self.uri
            }
        }
    }

    /// Get file extension from path (Total.js: request.extension)
    pub fn extension(&self) -> Option<&str> {
        let path = self.path();
        Path::new(path).extension().and_then(|ext| ext.to_str())
    }

    /// Check if request is authorized (Total.js: request.isAuthorized)
    pub fn is_authorized(&self) -> bool {
        self.headers.get("authorization").is_some()
    }

    /// Get authorization header (Total.js: request.authorization())
    pub fn authorization(&self) -> Option<&str> {
        self.headers.get("authorization").map(|s| s.as_str())
    }

    // Phase 2: Medium Priority Features

    /// Check if request is from a proxy (Total.js: request.isProxy)
    pub fn is_proxy(&self) -> bool {
        // Check for common proxy headers
        self.headers.contains_key("x-forwarded-for")
            || self.headers.contains_key("x-real-ip")
            || self.headers.contains_key("x-forwarded-proto")
            || self.headers.contains_key("x-forwarded-host")
            || self.headers.contains_key("forwarded")
    }

    /// Check if request is for a static file (Total.js: request.isStaticFile)
    pub fn is_static_file(&self) -> bool {
        if let Some(ext) = self.extension() {
            // Common static file extensions
            matches!(
                ext.to_lowercase().as_str(),
                "css"
                    | "js"
                    | "jpg"
                    | "jpeg"
                    | "png"
                    | "gif"
                    | "svg"
                    | "webp"
                    | "ico"
                    | "woff"
                    | "woff2"
                    | "ttf"
                    | "eot"
                    | "pdf"
                    | "zip"
                    | "txt"
                    | "xml"
                    | "json"
                    | "csv"
                    | "mp3"
                    | "mp4"
                    | "webm"
                    | "html"
                    | "htm"
                    | "map"
            )
        } else {
            false
        }
    }

    /// Get subdomain from host (Total.js: request.subdomain)
    pub fn subdomain(&self) -> Option<String> {
        if let Some(host) = self.host() {
            // Remove port if present
            let host = if let Some(port_pos) = host.find(':') {
                &host[..port_pos]
            } else {
                host
            };

            let parts: Vec<&str> = host.split('.').collect();
            if parts.len() > 2 {
                // Return everything before the last two parts (domain.tld)
                let subdomain_parts = &parts[..parts.len() - 2];
                if !subdomain_parts.is_empty() {
                    Some(subdomain_parts.join("."))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get path segments as array (Total.js: request.split)
    pub fn split(&self) -> Vec<&str> {
        self.path()
            .trim_start_matches('/')
            .trim_end_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Generate or retrieve CSRF token (Total.js: request.csrf())
    ///
    /// This generates a cryptographically secure CSRF token for the current request.
    /// In a full implementation, this would typically be stored in the session
    /// and validated against form submissions.
    pub fn csrf(&self) -> String {
        // Check if CSRF token already exists in headers (for validation)
        if let Some(existing_token) = self.headers.get("x-csrf-token") {
            return existing_token.clone();
        }

        // Generate new CSRF token
        Self::generate_csrf_token()
    }

    /// Generate a cryptographically secure CSRF token
    fn generate_csrf_token() -> String {
        use rand::{thread_rng, Rng};

        // Generate 32-byte random token and encode as base64
        let token_bytes: Vec<u8> = (0..32).map(|_| thread_rng().gen::<u8>()).collect();

        Self::base64_encode(&token_bytes)
    }

    // Helper methods

    /// Parse cookies from Cookie header
    fn parse_cookies(cookie_header: &str) -> HashMap<String, String> {
        let mut cookies = HashMap::new();

        for cookie_pair in cookie_header.split(';') {
            let cookie_pair = cookie_pair.trim();
            if let Some((name, value)) = cookie_pair.split_once('=') {
                let name = name.trim().to_string();
                let value = value.trim().to_string();
                cookies.insert(name, value);
            }
        }

        cookies
    }

    /// Simple base64 encoding without external dependencies
    fn base64_encode(input: &[u8]) -> String {
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::new();

        for chunk in input.chunks(3) {
            let b1 = chunk[0] as u32;
            let b2 = chunk.get(1).copied().unwrap_or(0) as u32;
            let b3 = chunk.get(2).copied().unwrap_or(0) as u32;

            let combined = (b1 << 16) | (b2 << 8) | b3;

            result.push(CHARS[((combined >> 18) & 63) as usize] as char);
            result.push(CHARS[((combined >> 12) & 63) as usize] as char);
            result.push(if chunk.len() > 1 {
                CHARS[((combined >> 6) & 63) as usize] as char
            } else {
                '='
            });
            result.push(if chunk.len() > 2 {
                CHARS[(combined & 63) as usize] as char
            } else {
                '='
            });
        }

        result
    }

    /// Reset Request state for pool reuse
    ///
    /// Clears all fields without deallocating underlying storage
    /// to maximize reuse efficiency.
    pub fn reset(&mut self) {
        self.method.clear();
        self.uri.clear();
        self.headers.clear();
        self.params.clear();
        self.query.clear();
        self.body_bytes.clear();
        self.files = None;

        // Shrink collections if they've grown too large
        // This prevents memory bloat from requests with large payloads
        const MAX_CAPACITY: usize = 1024;

        if self.headers.capacity() > MAX_CAPACITY {
            self.headers = HashMap::new();
        }
        if self.params.capacity() > MAX_CAPACITY {
            self.params = HashMap::new();
        }
        if self.query.capacity() > MAX_CAPACITY {
            self.query = HashMap::new();
        }
        if self.body_bytes.capacity() > MAX_CAPACITY * 1024 {
            // 1MB
            self.body_bytes = Vec::new();
        }
    }
}

// Secure URL encoding/decoding implementation
mod urlencoding {
    use std::borrow::Cow;

    /// Securely decode URL-encoded strings with validation
    pub fn decode(s: &str) -> Option<Cow<str>> {
        decode_safe(s).ok()
    }

    /// Safe URL decoding with comprehensive validation
    pub fn decode_safe(input: &str) -> Result<Cow<str>, UrlDecodeError> {
        // Input validation
        if input.len() > MAX_URL_LENGTH {
            return Err(UrlDecodeError::TooLong);
        }

        // Check for obviously malicious patterns
        if contains_malicious_patterns(input) {
            return Err(UrlDecodeError::MaliciousPattern);
        }

        let mut result = Vec::new();
        let mut chars = input.char_indices();

        while let Some((i, ch)) = chars.next() {
            match ch {
                '%' => {
                    // Ensure we have at least 2 more characters
                    if i + 2 >= input.len() {
                        return Err(UrlDecodeError::InvalidEncoding);
                    }

                    // Get the next two characters
                    let hex_str = &input[i + 1..i + 3];

                    // Validate hex characters
                    if !hex_str.chars().all(|c| c.is_ascii_hexdigit()) {
                        return Err(UrlDecodeError::InvalidHex);
                    }

                    // Parse hex value
                    let hex_value =
                        u8::from_str_radix(hex_str, 16).map_err(|_| UrlDecodeError::InvalidHex)?;

                    // Validate the decoded byte
                    if !is_safe_decoded_byte(hex_value) {
                        return Err(UrlDecodeError::UnsafeByte);
                    }

                    result.push(hex_value);

                    // Skip the next two characters
                    chars.next();
                    chars.next();
                }
                '+' => {
                    // Convert + to space (application/x-www-form-urlencoded)
                    result.push(b' ');
                }
                c if c.is_ascii() && is_safe_url_char(c) => {
                    result.push(c as u8);
                }
                _ => {
                    // Invalid or unsafe character
                    return Err(UrlDecodeError::InvalidCharacter);
                }
            }
        }

        // Convert bytes to string, validating UTF-8
        match String::from_utf8(result) {
            Ok(decoded) => {
                // Final validation of the decoded result
                if is_safe_decoded_string(&decoded) {
                    if decoded == input {
                        Ok(Cow::Borrowed(input))
                    } else {
                        Ok(Cow::Owned(decoded))
                    }
                } else {
                    Err(UrlDecodeError::UnsafeResult)
                }
            }
            Err(_) => Err(UrlDecodeError::InvalidUtf8),
        }
    }

    const MAX_URL_LENGTH: usize = 8192; // 8KB limit

    #[derive(Debug)]
    pub enum UrlDecodeError {
        TooLong,
        MaliciousPattern,
        InvalidEncoding,
        InvalidHex,
        UnsafeByte,
        InvalidCharacter,
        InvalidUtf8,
        UnsafeResult,
    }

    /// Check for malicious patterns in URL input
    fn contains_malicious_patterns(input: &str) -> bool {
        let dangerous_patterns = [
            // Path traversal
            "../",
            "..\\",
            "%2e%2e%2f",
            "%2e%2e%5c",
            // Null bytes
            "%00",
            "\0",
            // Script injection attempts
            "%3cscript",
            "%3c%73%63%72%69%70%74",
            // Unicode bypasses
            "%c0%ae",
            "%c1%9c",
            // Double encoding
            "%252e",
            "%252f",
        ];

        let input_lower = input.to_lowercase();
        dangerous_patterns
            .iter()
            .any(|pattern| input_lower.contains(pattern))
    }

    /// Check if a URL character is safe to include
    fn is_safe_url_char(c: char) -> bool {
        match c {
            // Unreserved characters
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~' => true,
            // Reserved characters that are safe in query strings
            ':' | '/' | '?' | '#' | '[' | ']' | '@' => true,
            // Sub-delims
            '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '=' => true,
            _ => false,
        }
    }

    /// Check if a decoded byte value is safe
    fn is_safe_decoded_byte(byte: u8) -> bool {
        match byte {
            // Null byte is never safe
            0 => false,
            // Control characters (except common whitespace)
            1..=8 | 11..=12 | 14..=31 | 127 => false,
            // DEL and high control characters
            128..=159 => false,
            // Everything else is potentially safe
            _ => true,
        }
    }

    /// Final validation of decoded string
    fn is_safe_decoded_string(s: &str) -> bool {
        // Check for null bytes
        if s.contains('\0') {
            return false;
        }

        // Check for malicious file patterns
        let dangerous_files = [
            "etc/passwd",
            "windows/system32",
            "boot.ini",
            "web.config",
            ".htaccess",
            ".env",
            "id_rsa",
            "shadow",
        ];

        let s_lower = s.to_lowercase();
        if dangerous_files
            .iter()
            .any(|pattern| s_lower.contains(pattern))
        {
            return false;
        }

        // Check for excessive control characters
        let control_count = s.chars().filter(|c| c.is_control()).count();
        if control_count > s.len() / 10 {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_url_decoding() {
        use super::urlencoding::decode;

        // Test basic decoding
        assert_eq!(decode("hello%20world").unwrap(), "hello world");
        assert_eq!(decode("test+string").unwrap(), "test string");
        assert_eq!(decode("normal_string").unwrap(), "normal_string");

        // Test malicious pattern detection
        assert!(decode("../etc/passwd").is_none());
        assert!(decode("%2e%2e%2fpasswd").is_none());
        assert!(decode("%00null").is_none());
        assert!(decode("%3cscript%3e").is_none());

        // Test length limits
        let long_string = "a".repeat(10000);
        assert!(decode(&long_string).is_none());

        // Test invalid encoding
        assert!(decode("invalid%GG").is_none());
        assert!(decode("incomplete%2").is_none());

        // Test control character filtering
        assert!(decode("%01%02%03").is_none());
    }

    #[test]
    fn test_url_decode_error_types() {
        use super::urlencoding::{decode_safe, UrlDecodeError};

        // Test specific error types
        let long_input = "a".repeat(10000);
        assert!(matches!(
            decode_safe(&long_input),
            Err(UrlDecodeError::TooLong)
        ));

        assert!(matches!(
            decode_safe("../passwd"),
            Err(UrlDecodeError::MaliciousPattern)
        ));
        assert!(matches!(
            decode_safe("invalid%GG"),
            Err(UrlDecodeError::InvalidHex)
        ));
        assert!(matches!(
            decode_safe("incomplete%2"),
            Err(UrlDecodeError::InvalidEncoding)
        ));

        // Test malicious pattern first - %00 is caught by malicious pattern detection
        assert!(matches!(
            decode_safe("%00test"),
            Err(UrlDecodeError::MaliciousPattern)
        ));

        // Test unsafe byte with a different control character that's not in malicious patterns
        assert!(matches!(
            decode_safe("%01test"),
            Err(UrlDecodeError::UnsafeByte)
        ));
    }

    #[test]
    fn test_safe_query_parsing() {
        // Test that query parsing handles malicious input safely
        let malicious_query = "key1=../passwd&key2=%00null&key3=normal";
        let parsed = Request::parse_query(malicious_query);

        // Malicious values should be filtered out by the decode function returning None
        // which causes unwrap_or_default() to return empty string, but the key will still exist
        // Let's just verify the normal value works
        assert_eq!(parsed.get("key3"), Some(&"normal".to_string()));

        // And that malicious patterns get replaced with empty strings
        assert_eq!(parsed.get("key1"), Some(&"".to_string()));
        assert_eq!(parsed.get("key2"), Some(&"".to_string()));
    }

    #[test]
    fn test_parse_query_function() {
        let query = Request::parse_query("param=value&other=test");

        assert_eq!(query.get("param"), Some(&"value".to_string()));
        assert_eq!(query.get("other"), Some(&"test".to_string()));
    }

    #[test]
    fn test_form_arrays() {
        // Test array notation with []
        let query = Request::parse_query_with_arrays("tags[]=rust&tags[]=web&tags[]=framework");

        assert_eq!(query.len(), 1);
        let tags_value = query.get("tags").unwrap();
        assert!(tags_value.is_array());
        assert_eq!(tags_value.as_array(), vec!["rust", "web", "framework"]);

        // Test mixed array and single values
        let query =
            Request::parse_query_with_arrays("name=John&hobbies[]=coding&hobbies[]=reading&age=30");

        assert_eq!(query.len(), 3);

        let name = query.get("name").unwrap();
        assert!(!name.is_array());
        assert_eq!(name.as_string(), "John");

        let hobbies = query.get("hobbies").unwrap();
        assert!(hobbies.is_array());
        assert_eq!(hobbies.as_array(), vec!["coding", "reading"]);

        let age = query.get("age").unwrap();
        assert!(!age.is_array());
        assert_eq!(age.as_string(), "30");
    }

    #[test]
    fn test_form_value_conversions() {
        // Test single value
        let single = FormValue::Single("test".to_string());
        assert_eq!(single.as_string(), "test");
        assert_eq!(single.as_array(), vec!["test"]);
        assert!(!single.is_array());

        // Test multiple values
        let multiple = FormValue::Multiple(vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
        ]);
        assert_eq!(multiple.as_string(), "one"); // Gets first element
        assert_eq!(multiple.as_array(), vec!["one", "two", "three"]);
        assert!(multiple.is_array());

        // Test empty array
        let empty_multiple = FormValue::Multiple(vec![]);
        assert_eq!(empty_multiple.as_string(), "");
        assert_eq!(empty_multiple.as_array(), Vec::<&str>::new());
        assert!(empty_multiple.is_array());
    }

    #[test]
    fn test_form_parsing_with_encoding() {
        // Test URL encoded values
        let query = Request::parse_query_with_arrays(
            "message=Hello%20World&tags[]=rust%2Blang&tags[]=web%20dev",
        );

        let message = query.get("message").unwrap();
        assert_eq!(message.as_string(), "Hello World");

        let tags = query.get("tags").unwrap();
        assert_eq!(tags.as_array(), vec!["rust+lang", "web dev"]);
    }

    #[test]
    fn test_form_parsing_malicious_input() {
        // Test that malicious patterns are filtered out
        let query = Request::parse_query("safe=value&bad=../etc/passwd&normal=test");

        assert_eq!(query.get("safe"), Some(&"value".to_string()));
        assert_eq!(query.get("bad"), Some(&"".to_string())); // Malicious input filtered
        assert_eq!(query.get("normal"), Some(&"test".to_string()));
    }

    #[test]
    fn test_form_arrays_edge_cases() {
        // Test single value that becomes array when more values added
        let query = Request::parse_query_with_arrays("item=first&item=second&item=third");

        let items = query.get("item").unwrap();
        assert!(items.is_array());
        assert_eq!(items.as_array(), vec!["first", "second", "third"]);

        // Test empty brackets
        let query = Request::parse_query_with_arrays("empty[]=&empty[]=&filled[]=value");

        let empty = query.get("empty").unwrap();
        assert!(empty.is_array());
        assert_eq!(empty.as_array(), vec!["", ""]);

        let filled = query.get("filled").unwrap();
        assert_eq!(filled.as_string(), "value");
    }

    // Tests for new Total.js compatibility features

    #[test]
    fn test_cookie_parsing() {
        let mut request = Request::default();
        request.headers.insert(
            "cookie".to_string(),
            "session=abc123; user=john; theme=dark".to_string(),
        );

        assert_eq!(request.cookie("session"), Some("abc123".to_string()));
        assert_eq!(request.cookie("user"), Some("john".to_string()));
        assert_eq!(request.cookie("theme"), Some("dark".to_string()));
        assert_eq!(request.cookie("nonexistent"), None);

        // Test with no cookies
        let empty_request = Request::default();
        assert_eq!(empty_request.cookie("any"), None);
    }

    #[test]
    fn test_host_and_hostname() {
        let mut request = Request::default();
        request
            .headers
            .insert("host".to_string(), "example.com:8080".to_string());

        assert_eq!(request.host(), Some("example.com:8080"));
        assert_eq!(request.hostname(None), "http://example.com:8080");
        assert_eq!(
            request.hostname(Some("/api/users")),
            "http://example.com:8080/api/users"
        );
        assert_eq!(
            request.hostname(Some("api/users")),
            "http://example.com:8080/api/users"
        );

        // Test HTTPS detection
        request
            .headers
            .insert("x-forwarded-proto".to_string(), "https".to_string());
        assert_eq!(request.hostname(None), "https://example.com:8080");

        // Test with no host
        let empty_request = Request::default();
        assert_eq!(empty_request.host(), None);
        assert_eq!(empty_request.hostname(None), "http://localhost");
    }

    #[test]
    fn test_path_and_extension() {
        let mut request = Request::default();

        // Test simple path
        request.uri = "/api/users.json".to_string();
        assert_eq!(request.path(), "/api/users.json");
        assert_eq!(request.extension(), Some("json"));

        // Test path with query parameters
        request.uri = "/api/users.json?limit=10&offset=0".to_string();
        assert_eq!(request.path(), "/api/users.json");
        assert_eq!(request.extension(), Some("json"));

        // Test full URL
        request.uri = "https://example.com:8080/api/users.html?param=value".to_string();
        assert_eq!(request.path(), "/api/users.html");
        assert_eq!(request.extension(), Some("html"));

        // Test root path
        request.uri = "/".to_string();
        assert_eq!(request.path(), "/");
        assert_eq!(request.extension(), None);

        // Test path without extension
        request.uri = "/api/users".to_string();
        assert_eq!(request.path(), "/api/users");
        assert_eq!(request.extension(), None);

        // Test full URL without path
        request.uri = "https://example.com".to_string();
        assert_eq!(request.path(), "/");
        assert_eq!(request.extension(), None);
    }

    #[test]
    fn test_authorization() {
        let mut request = Request::default();

        // Test with no authorization
        assert!(!request.is_authorized());
        assert_eq!(request.authorization(), None);

        // Test with authorization header
        request
            .headers
            .insert("authorization".to_string(), "Bearer token123".to_string());
        assert!(request.is_authorized());
        assert_eq!(request.authorization(), Some("Bearer token123"));

        // Test with basic auth
        request.headers.insert(
            "authorization".to_string(),
            "Basic dXNlcjpwYXNz".to_string(),
        );
        assert!(request.is_authorized());
        assert_eq!(request.authorization(), Some("Basic dXNlcjpwYXNz"));
    }

    #[test]
    fn test_proxy_detection() {
        let mut request = Request::default();

        // Test with no proxy headers
        assert!(!request.is_proxy());

        // Test with X-Forwarded-For
        request
            .headers
            .insert("x-forwarded-for".to_string(), "192.168.1.1".to_string());
        assert!(request.is_proxy());

        // Test with X-Real-IP
        request = Request::default();
        request
            .headers
            .insert("x-real-ip".to_string(), "10.0.0.1".to_string());
        assert!(request.is_proxy());

        // Test with X-Forwarded-Proto
        request = Request::default();
        request
            .headers
            .insert("x-forwarded-proto".to_string(), "https".to_string());
        assert!(request.is_proxy());

        // Test with Forwarded header
        request = Request::default();
        request.headers.insert(
            "forwarded".to_string(),
            "for=192.0.2.60;proto=http".to_string(),
        );
        assert!(request.is_proxy());
    }

    #[test]
    fn test_static_file_detection() {
        let mut request = Request::default();

        // Test static file extensions
        let static_files = [
            "/assets/style.css",
            "/js/app.js",
            "/images/logo.png",
            "/favicon.ico",
            "/font.woff2",
            "/document.pdf",
            "/data.json",
            "/video.mp4",
            "/map.xml",
        ];

        for file_path in static_files {
            request.uri = file_path.to_string();
            assert!(
                request.is_static_file(),
                "Should detect {} as static file",
                file_path
            );
        }

        // Test non-static paths
        let dynamic_paths = ["/api/users", "/", "/users/123", "/admin/login"];

        for path in dynamic_paths {
            request.uri = path.to_string();
            assert!(
                !request.is_static_file(),
                "Should not detect {} as static file",
                path
            );
        }
    }

    #[test]
    fn test_subdomain_extraction() {
        let mut request = Request::default();

        // Test with subdomain
        request
            .headers
            .insert("host".to_string(), "api.example.com".to_string());
        assert_eq!(request.subdomain(), Some("api".to_string()));

        // Test with multiple subdomains
        request
            .headers
            .insert("host".to_string(), "v1.api.example.com".to_string());
        assert_eq!(request.subdomain(), Some("v1.api".to_string()));

        // Test with no subdomain
        request
            .headers
            .insert("host".to_string(), "example.com".to_string());
        assert_eq!(request.subdomain(), None);

        // Test with port
        request
            .headers
            .insert("host".to_string(), "api.example.com:8080".to_string());
        assert_eq!(request.subdomain(), Some("api".to_string()));

        // Test with www
        request
            .headers
            .insert("host".to_string(), "www.example.com".to_string());
        assert_eq!(request.subdomain(), Some("www".to_string()));

        // Test with no host
        request.headers.clear();
        assert_eq!(request.subdomain(), None);
    }

    #[test]
    fn test_path_split() {
        let mut request = Request::default();

        // Test normal path
        request.uri = "/api/v1/users/123".to_string();
        assert_eq!(request.split(), vec!["api", "v1", "users", "123"]);

        // Test root path
        request.uri = "/".to_string();
        assert_eq!(request.split(), Vec::<&str>::new());

        // Test path with trailing slash
        request.uri = "/api/users/".to_string();
        assert_eq!(request.split(), vec!["api", "users"]);

        // Test path with query parameters
        request.uri = "/api/users?limit=10".to_string();
        assert_eq!(request.split(), vec!["api", "users"]);

        // Test single segment
        request.uri = "/dashboard".to_string();
        assert_eq!(request.split(), vec!["dashboard"]);
    }

    #[test]
    fn test_csrf_token() {
        let mut request = Request::default();

        // Test CSRF token generation
        let token1 = request.csrf();
        let token2 = request.csrf();

        // Tokens should be different each time (new generation)
        assert_ne!(token1, token2);

        // Tokens should be base64 encoded (contain valid base64 characters)
        assert!(token1
            .chars()
            .all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '='));
        assert!(token2
            .chars()
            .all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '='));

        // Tokens should have reasonable length (base64 encoding of 32 bytes = 44 chars with padding)
        assert!(token1.len() >= 40);
        assert!(token2.len() >= 40);

        // Test with existing CSRF token in headers
        request
            .headers
            .insert("x-csrf-token".to_string(), "existing-token-123".to_string());
        let existing_token = request.csrf();
        assert_eq!(existing_token, "existing-token-123");
    }

    #[test]
    fn test_base64_encode() {
        // Test base64 encoding function
        let input1 = b"hello";
        let encoded1 = Request::base64_encode(input1);
        assert_eq!(encoded1, "aGVsbG8=");

        let input2 = b"hello world";
        let encoded2 = Request::base64_encode(input2);
        assert_eq!(encoded2, "aGVsbG8gd29ybGQ=");

        // Test empty input
        let input3 = b"";
        let encoded3 = Request::base64_encode(input3);
        assert_eq!(encoded3, "");
    }
}
