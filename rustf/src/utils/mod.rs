//! Global utilities module for RustF framework
//!
//! This module provides commonly used utility functions for web development,
//! accessible globally through the `U` struct (similar to Total.js Utils).
//!
//! # Usage
//! ```rust
//! use rustf::U;
//!
//! let guid = U::guid();
//! let encoded = U::encode("hello world");
//! let status_text = U::http_status(404);
//! ```

pub mod crypto;
pub mod encoding;
pub mod geo;
pub mod http;
pub mod object;
pub mod pagination;
pub mod parsing;
pub mod random;
pub mod string;

/// Global utilities module - the main entry point for all utility functions
///
/// This module provides access to utility functions for common operations in web development.
/// It follows the Total.js convention of providing utilities through a global `U` namespace.
#[allow(non_snake_case)]
pub mod U {
    use super::*;
    use crate::error::Result;
    use serde_json::Value;
    use std::string::String as StdString;

    /// String utilities namespace
    pub mod String {
        pub use super::super::string::*;
    }

    /// Encoding utilities namespace
    pub mod Encoding {
        pub use super::super::encoding::*;
    }

    /// Parsing utilities namespace
    pub mod Parsing {
        pub use super::super::parsing::*;
    }

    /// Random utilities namespace
    pub mod Random {
        pub use super::super::random::*;
    }

    /// Crypto utilities namespace
    pub mod Crypto {
        pub use super::super::crypto::*;
    }

    /// HTTP utilities namespace
    pub mod Http {
        pub use super::super::http::*;
    }

    /// Object utilities namespace
    pub mod Object {
        pub use super::super::object::*;
    }

    /// Geo utilities namespace
    pub mod Geo {
        pub use super::super::geo::*;
    }

    // Random generation utilities

    /// Generate a new UUID/GUID
    ///
    /// # Example
    /// ```rust,ignore
    /// let id = U::guid();
    /// println!("Generated ID: {}", id);
    /// ```
    pub fn guid() -> StdString {
        random::guid()
    }

    /// Generate a random string of specified length
    ///
    /// # Arguments
    /// * `length` - The length of the random string to generate
    ///
    /// # Example
    /// ```rust,ignore
    /// let token = U::random_string(32);
    /// println!("Random token: {}", token);
    /// ```
    pub fn random_string(length: usize) -> StdString {
        random::string(length)
    }

    /// Generate a random number between min and max (inclusive)
    ///
    /// # Arguments
    /// * `min` - Minimum value (inclusive)
    /// * `max` - Maximum value (inclusive)
    ///
    /// # Example
    /// ```rust,ignore
    /// let num = U::random_number(1, 100);
    /// println!("Random number: {}", num);
    /// ```
    pub fn random_number(min: i64, max: i64) -> i64 {
        random::number(min, max)
    }

    // HTTP utilities

    /// Get HTTP status text for a status code
    ///
    /// # Arguments
    /// * `code` - HTTP status code
    ///
    /// # Example
    /// ```rust,ignore
    /// let status = U::http_status(404);
    /// assert_eq!(status, "Not Found");
    /// ```
    pub fn http_status(code: u16) -> &'static str {
        http::status_text(code)
    }

    /// Generate an ETag for content
    ///
    /// # Arguments
    /// * `content` - The content to generate ETag for
    ///
    /// # Example
    /// ```rust,ignore
    /// let etag = U::etag("Hello, World!");
    /// println!("ETag: {}", etag);
    /// ```
    pub fn etag(content: &str) -> StdString {
        http::etag(content)
    }

    /// Get MIME content type for file extension
    ///
    /// # Arguments
    /// * `extension` - File extension (with or without dot)
    ///
    /// # Example
    /// ```rust,ignore
    /// let mime = U::get_content_type("html");
    /// assert_eq!(mime, "text/html");
    /// ```
    pub fn get_content_type(extension: &str) -> &'static str {
        http::content_type(extension)
    }

    // Encoding/Decoding utilities

    /// URL encode a string
    ///
    /// # Arguments
    /// * `input` - String to encode
    ///
    /// # Example
    /// ```rust,ignore
    /// let encoded = U::encode("hello world");
    /// assert_eq!(encoded, "hello%20world");
    /// ```
    pub fn encode(input: &str) -> StdString {
        encoding::encode(input)
    }

    /// URL decode a string
    ///
    /// # Arguments
    /// * `input` - String to decode
    ///
    /// # Example
    /// ```rust,ignore
    /// let decoded = U::decode("hello%20world").unwrap();
    /// assert_eq!(decoded, "hello world");
    /// ```
    pub fn decode(input: &str) -> Result<StdString> {
        encoding::decode(input)
    }

    /// Base64 encode a string
    ///
    /// # Arguments
    /// * `input` - String to encode
    ///
    /// # Example
    /// ```rust,ignore
    /// let encoded = U::btoa("hello");
    /// println!("Base64: {}", encoded);
    /// ```
    pub fn btoa(input: &str) -> StdString {
        encoding::btoa(input)
    }

    /// Base64 decode a string
    ///
    /// # Arguments
    /// * `input` - Base64 string to decode
    ///
    /// # Example
    /// ```rust,ignore
    /// let decoded = U::atob("aGVsbG8=").unwrap();
    /// assert_eq!(decoded, "hello");
    /// ```
    pub fn atob(input: &str) -> Result<StdString> {
        encoding::atob(input)
    }

    // Parsing utilities

    /// Parse a string to boolean with default value
    ///
    /// # Arguments
    /// * `value` - String to parse
    /// * `default` - Default value if parsing fails
    ///
    /// # Example
    /// ```rust,ignore
    /// let result = U::parse_bool("true", false);
    /// assert_eq!(result, true);
    /// ```
    pub fn parse_bool(value: &str, default: bool) -> bool {
        parsing::bool(value, default)
    }

    /// Parse a string to integer with default value
    ///
    /// # Arguments
    /// * `value` - String to parse
    /// * `default` - Default value if parsing fails
    ///
    /// # Example
    /// ```rust,ignore
    /// let result = U::parse_int("123", 0);
    /// assert_eq!(result, 123);
    /// ```
    pub fn parse_int(value: &str, default: i64) -> i64 {
        parsing::int(value, default)
    }

    /// Parse a string to float with default value
    ///
    /// # Arguments
    /// * `value` - String to parse
    /// * `default` - Default value if parsing fails
    ///
    /// # Example
    /// ```rust,ignore
    /// let result = U::parse_float("123.45", 0.0);
    /// assert_eq!(result, 123.45);
    /// ```
    pub fn parse_float(value: &str, default: f64) -> f64 {
        parsing::float(value, default)
    }

    // String utilities

    /// Trim whitespace and clean up a string
    ///
    /// # Arguments
    /// * `input` - String to trim
    ///
    /// # Example
    /// ```rust,ignore
    /// let cleaned = U::trim("  hello world  ");
    /// assert_eq!(cleaned, "hello world");
    /// ```
    pub fn trim(input: &str) -> StdString {
        string::trim(input)
    }

    /// Extract keywords from text for search indexing
    ///
    /// # Arguments
    /// * `content` - Text content to process
    /// * `max_count` - Maximum number of keywords to return
    /// * `min_length` - Minimum keyword length
    ///
    /// # Example
    /// ```rust,ignore
    /// let keywords = U::keywords("This is a sample text", 10, 3);
    /// println!("Keywords: {:?}", keywords);
    /// ```
    pub fn keywords(content: &str, max_count: usize, min_length: usize) -> Vec<StdString> {
        string::keywords(content, max_count, min_length)
    }

    // Object manipulation utilities

    /// Get a nested property from a JSON object safely
    ///
    /// # Arguments
    /// * `obj` - JSON object to search in
    /// * `path` - Dot-separated path (e.g., "user.profile.name")
    ///
    /// # Example
    /// ```rust,ignore
    /// let data = json!({"user": {"name": "John"}});
    /// let name = U::get(&data, "user.name");
    /// ```
    pub fn get<'a>(obj: &'a Value, path: &str) -> Option<&'a Value> {
        object::get(obj, path)
    }

    /// Set a nested property in a JSON object
    ///
    /// # Arguments
    /// * `obj` - Mutable JSON object to modify
    /// * `path` - Dot-separated path (e.g., "user.profile.name")
    /// * `value` - Value to set
    ///
    /// # Example
    /// ```rust,ignore
    /// let mut data = json!({});
    /// U::set(&mut data, "user.name", json!("John"));
    /// ```
    pub fn set(obj: &mut Value, path: &str, value: Value) -> Result<()> {
        object::set(obj, path, value)
    }

    // Geographic utilities

    /// Calculate distance between two geographic points in kilometers
    ///
    /// # Arguments
    /// * `lat1` - Latitude of first point
    /// * `lon1` - Longitude of first point
    /// * `lat2` - Latitude of second point
    /// * `lon2` - Longitude of second point
    ///
    /// # Example
    /// ```rust,ignore
    /// let distance = U::distance(40.7128, -74.0060, 34.0522, -118.2437);
    /// println!("Distance: {:.2} km", distance);
    /// ```
    pub fn distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
        geo::distance(lat1, lon1, lat2, lon2)
    }

    // Cryptographic hash utilities (direct access to most common ones)

    /// Generate MD5 hash of input string
    ///
    /// **Security Warning**: MD5 is cryptographically broken and should not be used
    /// for security purposes. Use SHA256 or higher for security applications.
    ///
    /// # Arguments
    /// * `input` - String to hash
    ///
    /// # Example
    /// ```rust,ignore
    /// let hash = U::md5("hello world");
    /// assert_eq!(hash, "5eb63bbbe01eeed093cb22bb8f5acdc3");
    /// ```
    pub fn md5(input: &str) -> StdString {
        crypto::md5(input)
    }

    /// Generate SHA256 hash of input string
    ///
    /// This is a secure hash function suitable for security applications.
    ///
    /// # Arguments
    /// * `input` - String to hash
    ///
    /// # Example
    /// ```rust,ignore
    /// let hash = U::sha256("hello world");
    /// assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    /// ```
    pub fn sha256(input: &str) -> StdString {
        crypto::sha256(input)
    }

    // Pagination utilities

    /// Create a pagination object for building pagination UI
    ///
    /// This creates a pagination helper that calculates page numbers, URLs,
    /// and provides all necessary data for rendering pagination controls.
    ///
    /// # Arguments
    /// * `total` - Total number of items
    /// * `page` - Current page (1-based)
    /// * `per_page` - Items per page
    /// * `url_pattern` - URL pattern with {0} placeholder for page number
    ///
    /// # Example
    /// ```rust,ignore
    /// let total_users = Users::count().await?;
    /// let page = U::parse_int(ctx.query("page").unwrap_or("1"), 1) as u32;
    /// let per_page = 20;
    ///
    /// let pagination = U::paginate(total_users, page, per_page, "/users?page={0}");
    ///
    /// // Use in template data
    /// ctx.view("users/list", json!({
    ///     "pagination": pagination.to_json()
    /// }))
    /// ```
    ///
    /// # Template Usage
    /// ```html
    /// @{if pagination.isPrev}
    ///     <a href="@{pagination.prev.url}">Previous</a>
    /// @{fi}
    ///
    /// @{foreach page in pagination.range}
    ///     @{if page.selected}
    ///         <span>@{page.page}</span>
    ///     @{else}
    ///         <a href="@{page.url}">@{page.page}</a>
    ///     @{fi}
    /// @{end}
    ///
    /// @{if pagination.isNext}
    ///     <a href="@{pagination.next.url}">Next</a>
    /// @{fi}
    /// ```
    pub fn paginate(
        total: i64,
        page: u32,
        per_page: u32,
        url_pattern: StdString,
    ) -> pagination::Pagination {
        pagination::Pagination::new(total, page, per_page, url_pattern)
    }
}

/// Alias for the global utilities module (for compatibility)
pub use U as Utils;

// Extended utilities are now available via the nested modules within U::
// This provides the exact U::ModuleName::function() syntax that was requested!
//
// Available extended modules:
// - U::String::to_slug(), U::String::to_camel_case(), U::String::title_case(), etc.
// - U::Encoding::html_encode(), U::Encoding::hex_encode(), U::Encoding::base64_url_encode(), etc.
// - U::Parsing::parse_duration(), U::Parsing::parse_size(), U::Parsing::parse_percentage(), etc.
// - U::Random::generate_guid_with_hyphens(), U::Random::generate_secure_token(), etc.
// - U::Crypto::hash_string(), U::Crypto::md5(), U::Crypto::sha1(), U::Crypto::sha256(), U::Crypto::sha512(), etc.
// - U::Http::is_success_status(), U::Http::get_extension_from_content_type(), etc.
// - U::Object::deep_merge(), U::Object::has_nested_property(), U::Object::flatten_object(), etc.
// - U::Geo::distance_miles(), U::Geo::in_bounds(), etc.
//
// Usage examples:
// ```rust
// use rustf::utils::U;
//
// // Basic utilities (direct functions)
// let id = U::guid();
// let encoded = U::encode("hello world");
//
// // Hash functions (direct access to most common)
// let md5_hash = U::md5("hello world");        // Warning: MD5 not secure
// let sha256_hash = U::sha256("hello world");  // Secure hash function
//
// // Extended utilities (nested modules)
// let slug = U::String::to_slug("Hello World!");
// let html = U::Encoding::html_encode("<script>alert('xss')</script>");
// let duration = U::Parsing::parse_duration("1h", 0);
//
// // All hash functions via Crypto namespace
// let sha1_hash = U::Crypto::sha1("data");       // Warning: SHA1 not secure
// let sha512_hash = U::Crypto::sha512("data");   // Very secure hash function
// let hash_from_bytes = U::Crypto::md5_bytes(&[1, 2, 3, 4, 5]);
// ```

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_u_struct_basic_operations() {
        // Test basic utilities are accessible
        let guid = U::guid();
        assert!(!guid.is_empty());

        let random_str = U::random_string(10);
        assert_eq!(random_str.len(), 10);

        let status = U::http_status(200);
        assert_eq!(status, "OK");

        let encoded = U::encode("hello world");
        assert!(encoded.contains("%20"));

        let bool_val = U::parse_bool("true", false);
        assert_eq!(bool_val, true);
    }

    #[test]
    fn test_u_struct_object_operations() {
        let data = json!({"user": {"name": "John", "age": 25}});

        let name = U::get(&data, "user.name");
        assert_eq!(name, Some(&json!("John")));

        let mut data = json!({});
        U::set(&mut data, "user.name", json!("Jane")).unwrap();
        assert_eq!(U::get(&data, "user.name"), Some(&json!("Jane")));
    }

    #[test]
    fn test_extended_utilities_via_modules() {
        // Test string utilities via module access
        let slug = string::to_slug("Hello World!");
        assert_eq!(slug, "hello-world");

        let camel = string::to_camel_case("hello world");
        assert_eq!(camel, "helloWorld");

        // Test encoding utilities
        let html = encoding::html_encode("<p>Hello</p>");
        assert_eq!(html, "&lt;p&gt;Hello&lt;&#x2F;p&gt;");

        let hex = encoding::hex_encode(&[255, 0, 128]);
        assert_eq!(hex, "ff0080");

        // Test parsing utilities
        let duration = parsing::parse_duration("1h", 0);
        assert_eq!(duration, 3600);

        let size = parsing::parse_size("1KB", 0);
        assert_eq!(size, 1024);

        // Test crypto utilities
        let hash1 = crypto::hash_string("hello");
        let hash2 = crypto::hash_string("hello");
        assert_eq!(hash1, hash2);

        // Test random utilities
        let uuid = random::generate_guid_with_hyphens();
        assert_eq!(uuid.len(), 36);
        assert!(uuid.contains('-'));

        // Test HTTP utilities
        assert!(http::is_success_status(200));
        assert!(!http::is_success_status(404));

        let ext = http::get_extension_from_content_type("application/json");
        assert_eq!(ext, "json");

        // Test object utilities
        let data = json!({"user": {"name": "John"}});
        assert!(object::has_nested_property(&data, "user.name"));
        assert!(!object::has_nested_property(&data, "user.email"));

        // Test geo utilities
        let distance_miles = geo::distance_miles(40.7, -74.0, 34.0, -118.2);
        assert!(distance_miles > 2000.0); // NY to LA is > 2000 miles

        assert!(geo::in_bounds(40.7, -74.0, 40.0, 41.0, -75.0, -73.0));
    }

    #[test]
    fn test_nested_module_syntax() {
        // Test the new U::ModuleName::function() syntax - exactly what you requested!

        // Basic utilities - direct access through U::
        let id = U::guid();
        assert!(!id.is_empty());
        assert_eq!(id.len(), 32); // UUID without hyphens

        let encoded = U::encode("hello world");
        assert_eq!(encoded, "hello%20world");

        // Extended utilities - nested module access through U::ModuleName::
        let slug = U::String::to_slug("Hello World!");
        assert_eq!(slug, "hello-world");

        let camel = U::String::to_camel_case("hello world");
        assert_eq!(camel, "helloWorld");

        // Test encoding utilities via U::Encoding::
        let html = U::Encoding::html_encode("<p>Hello</p>");
        assert_eq!(html, "&lt;p&gt;Hello&lt;&#x2F;p&gt;");

        let hex = U::Encoding::hex_encode(&[255, 0, 128]);
        assert_eq!(hex, "ff0080");

        // Test parsing utilities via U::Parsing::
        let duration = U::Parsing::parse_duration("1h", 0);
        assert_eq!(duration, 3600);

        let size = U::Parsing::parse_size("1KB", 0);
        assert_eq!(size, 1024);

        // Test crypto utilities via U::Crypto::
        let hash1 = U::Crypto::hash_string("hello");
        let hash2 = U::Crypto::hash_string("hello");
        assert_eq!(hash1, hash2);

        // Test new hash functions via U::Crypto::
        let md5_hash = U::Crypto::md5("hello world");
        assert_eq!(md5_hash, "5eb63bbbe01eeed093cb22bb8f5acdc3");

        let sha256_hash = U::Crypto::sha256("hello world");
        assert_eq!(
            sha256_hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );

        // Test direct access hash functions
        let md5_direct = U::md5("hello world");
        assert_eq!(md5_direct, "5eb63bbbe01eeed093cb22bb8f5acdc3");

        let sha256_direct = U::sha256("hello world");
        assert_eq!(
            sha256_direct,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );

        // Test random utilities via U::Random::
        let uuid = U::Random::generate_guid_with_hyphens();
        assert_eq!(uuid.len(), 36);
        assert!(uuid.contains('-'));

        // Test HTTP utilities via U::Http::
        assert!(U::Http::is_success_status(200));
        assert!(!U::Http::is_success_status(404));

        let ext = U::Http::get_extension_from_content_type("application/json");
        assert_eq!(ext, "json");

        // Test object utilities via U::Object::
        let data = json!({"user": {"name": "John"}});
        assert!(U::Object::has_nested_property(&data, "user.name"));
        assert!(!U::Object::has_nested_property(&data, "user.email"));

        // Test geo utilities via U::Geo::
        let distance_miles = U::Geo::distance_miles(40.7, -74.0, 34.0, -118.2);
        assert!(distance_miles > 2000.0); // NY to LA is > 2000 miles

        assert!(U::Geo::in_bounds(40.7, -74.0, 40.0, 41.0, -75.0, -73.0));
    }
}
