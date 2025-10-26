//! HTTP utilities for RustF framework
//!
//! This module provides common HTTP-related utility functions including
//! status code handling, ETag generation, and MIME type detection.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Get HTTP status text for a given status code
///
/// Returns the standard HTTP status message for the given code.
/// If the status code is not recognized, returns "Unknown Status".
///
/// # Arguments
/// * `code` - HTTP status code
///
/// # Example
/// ```rust,ignore
/// let status = status_text(404);
/// assert_eq!(status, "Not Found");
/// ```
pub fn status_text(code: u16) -> &'static str {
    match code {
        // 1xx Informational
        100 => "Continue",
        101 => "Switching Protocols",
        102 => "Processing",
        103 => "Early Hints",

        // 2xx Success
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        203 => "Non-Authoritative Information",
        204 => "No Content",
        205 => "Reset Content",
        206 => "Partial Content",
        207 => "Multi-Status",
        208 => "Already Reported",
        226 => "IM Used",

        // 3xx Redirection
        300 => "Multiple Choices",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        305 => "Use Proxy",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",

        // 4xx Client Error
        400 => "Bad Request",
        401 => "Unauthorized",
        402 => "Payment Required",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        407 => "Proxy Authentication Required",
        408 => "Request Timeout",
        409 => "Conflict",
        410 => "Gone",
        411 => "Length Required",
        412 => "Precondition Failed",
        413 => "Payload Too Large",
        414 => "URI Too Long",
        415 => "Unsupported Media Type",
        416 => "Range Not Satisfiable",
        417 => "Expectation Failed",
        418 => "I'm a teapot",
        421 => "Misdirected Request",
        422 => "Unprocessable Entity",
        423 => "Locked",
        424 => "Failed Dependency",
        425 => "Too Early",
        426 => "Upgrade Required",
        428 => "Precondition Required",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        451 => "Unavailable For Legal Reasons",

        // 5xx Server Error
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        505 => "HTTP Version Not Supported",
        506 => "Variant Also Negotiates",
        507 => "Insufficient Storage",
        508 => "Loop Detected",
        510 => "Not Extended",
        511 => "Network Authentication Required",

        _ => "Unknown Status",
    }
}

/// Generate an ETag for the given content
///
/// Creates a simple ETag based on the content hash. This is useful for
/// HTTP caching mechanisms.
///
/// # Arguments
/// * `content` - The content to generate an ETag for
///
/// # Example
/// ```rust,ignore
/// let etag = etag("Hello, World!");
/// println!("ETag: {}", etag); // e.g., "W/\"1234567890abcdef\""
/// ```
pub fn etag(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    let hash = hasher.finish();
    format!("W/\"{:x}\"", hash)
}

/// Generate a strong ETag for the given content with additional metadata
///
/// Creates a strong ETag that includes content length and modification info.
///
/// # Arguments
/// * `content` - The content to generate an ETag for
/// * `last_modified` - Optional last modified timestamp
///
/// # Example
/// ```rust,ignore
/// let etag = generate_strong_etag("Hello, World!", Some(1640995200));
/// println!("ETag: {}", etag); // e.g., "\"1234567890abcdef-13-1640995200\""
/// ```
pub fn generate_strong_etag(content: &str, last_modified: Option<u64>) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    let content_hash = hasher.finish();

    match last_modified {
        Some(timestamp) => format!("\"{:x}-{}-{}\"", content_hash, content.len(), timestamp),
        None => format!("\"{:x}-{}\"", content_hash, content.len()),
    }
}

/// Get MIME content type for a file extension
///
/// Returns the appropriate MIME type for common file extensions.
/// If the extension is not recognized, returns "application/octet-stream".
///
/// # Arguments
/// * `extension` - File extension (with or without leading dot)
///
/// # Example
/// ```rust,ignore
/// let mime = content_type("html");
/// assert_eq!(mime, "text/html");
/// ```
pub fn content_type(extension: &str) -> &'static str {
    // Remove leading dot if present
    let ext = extension.trim_start_matches('.');

    match ext.to_lowercase().as_str() {
        // Text types
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "text/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "txt" => "text/plain",
        "csv" => "text/csv",
        "md" | "markdown" => "text/markdown",
        "yaml" | "yml" => "application/x-yaml",

        // Image types
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "tiff" | "tif" => "image/tiff",

        // Font types
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "eot" => "application/vnd.ms-fontobject",

        // Document types
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",

        // Archive types
        "zip" => "application/zip",
        "rar" => "application/vnd.rar",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        "7z" => "application/x-7z-compressed",

        // Audio types
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "flac" => "audio/flac",

        // Video types
        "mp4" => "video/mp4",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "wmv" => "video/x-ms-wmv",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",

        // Default
        _ => "application/octet-stream",
    }
}

/// Get file extension from MIME content type
///
/// Returns the most common file extension for the given MIME type.
/// If the MIME type is not recognized, returns "bin".
///
/// # Arguments
/// * `mime_type` - MIME content type
///
/// # Example
/// ```rust,ignore
/// let ext = get_extension_from_content_type("text/html");
/// assert_eq!(ext, "html");
/// ```
pub fn get_extension_from_content_type(mime_type: &str) -> &'static str {
    match mime_type.to_lowercase().as_str() {
        // Text types
        "text/html" => "html",
        "text/css" => "css",
        "text/javascript" | "application/javascript" => "js",
        "application/json" => "json",
        "application/xml" | "text/xml" => "xml",
        "text/plain" => "txt",
        "text/csv" => "csv",
        "text/markdown" => "md",
        "application/x-yaml" => "yaml",

        // Image types
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/svg+xml" => "svg",
        "image/x-icon" => "ico",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        "image/tiff" => "tiff",

        // Font types
        "font/woff" => "woff",
        "font/woff2" => "woff2",
        "font/ttf" => "ttf",
        "font/otf" => "otf",
        "application/vnd.ms-fontobject" => "eot",

        // Document types
        "application/pdf" => "pdf",
        "application/msword" => "doc",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx",
        "application/vnd.ms-excel" => "xls",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "xlsx",

        // Archive types
        "application/zip" => "zip",
        "application/vnd.rar" => "rar",
        "application/x-tar" => "tar",
        "application/gzip" => "gz",
        "application/x-7z-compressed" => "7z",

        // Audio types
        "audio/mpeg" => "mp3",
        "audio/wav" => "wav",
        "audio/ogg" => "ogg",
        "audio/mp4" => "m4a",
        "audio/flac" => "flac",

        // Video types
        "video/mp4" => "mp4",
        "video/x-msvideo" => "avi",
        "video/quicktime" => "mov",
        "video/x-ms-wmv" => "wmv",
        "video/webm" => "webm",
        "video/x-matroska" => "mkv",

        // Default
        _ => "bin",
    }
}

/// Check if a status code represents a successful response
///
/// # Arguments
/// * `code` - HTTP status code
///
/// # Example
/// ```rust,ignore
/// assert!(is_success_status(200));
/// assert!(!is_success_status(404));
/// ```
pub fn is_success_status(code: u16) -> bool {
    (200..300).contains(&code)
}

/// Check if a status code represents a client error
///
/// # Arguments
/// * `code` - HTTP status code
///
/// # Example
/// ```rust,ignore
/// assert!(is_client_error(404));
/// assert!(!is_client_error(200));
/// ```
pub fn is_client_error(code: u16) -> bool {
    (400..500).contains(&code)
}

/// Check if a status code represents a server error
///
/// # Arguments
/// * `code` - HTTP status code
///
/// # Example
/// ```rust,ignore
/// assert!(is_server_error(500));
/// assert!(!is_server_error(404));
/// ```
pub fn is_server_error(code: u16) -> bool {
    (500..600).contains(&code)
}

/// Check if a status code represents a redirection
///
/// # Arguments
/// * `code` - HTTP status code
///
/// # Example
/// ```rust,ignore
/// assert!(is_redirect_status(301));
/// assert!(!is_redirect_status(200));
/// ```
pub fn is_redirect_status(code: u16) -> bool {
    (300..400).contains(&code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_text() {
        assert_eq!(status_text(200), "OK");
        assert_eq!(status_text(404), "Not Found");
        assert_eq!(status_text(500), "Internal Server Error");
        assert_eq!(status_text(999), "Unknown Status");
    }

    #[test]
    fn test_etag() {
        let etag1 = etag("Hello, World!");
        let etag2 = etag("Hello, World!");
        let etag3 = etag("Different content");

        // Same content should produce same ETag
        assert_eq!(etag1, etag2);

        // Different content should produce different ETag
        assert_ne!(etag1, etag3);

        // Should have correct format
        assert!(etag1.starts_with("W/\""));
        assert!(etag1.ends_with("\""));
    }

    #[test]
    fn test_generate_strong_etag() {
        let etag1 = generate_strong_etag("Hello", None);
        let etag2 = generate_strong_etag("Hello", Some(1640995200));

        // Should have correct format
        assert!(etag1.starts_with("\""));
        assert!(etag1.ends_with("\""));
        assert!(etag2.contains("1640995200"));

        // Should include content length
        assert!(etag1.contains("-5")); // "Hello" is 5 characters
    }

    #[test]
    fn test_content_type() {
        assert_eq!(content_type("html"), "text/html");
        assert_eq!(content_type(".html"), "text/html");
        assert_eq!(content_type("HTML"), "text/html");
        assert_eq!(content_type("css"), "text/css");
        assert_eq!(content_type("js"), "text/javascript");
        assert_eq!(content_type("json"), "application/json");
        assert_eq!(content_type("png"), "image/png");
        assert_eq!(content_type("jpg"), "image/jpeg");
        assert_eq!(content_type("pdf"), "application/pdf");
        assert_eq!(content_type("unknown"), "application/octet-stream");
    }

    #[test]
    fn test_get_extension_from_content_type() {
        assert_eq!(get_extension_from_content_type("text/html"), "html");
        assert_eq!(get_extension_from_content_type("TEXT/HTML"), "html");
        assert_eq!(get_extension_from_content_type("text/css"), "css");
        assert_eq!(get_extension_from_content_type("application/json"), "json");
        assert_eq!(get_extension_from_content_type("image/png"), "png");
        assert_eq!(get_extension_from_content_type("image/jpeg"), "jpg");
        assert_eq!(get_extension_from_content_type("application/pdf"), "pdf");
        assert_eq!(get_extension_from_content_type("unknown/type"), "bin");
    }

    #[test]
    fn test_status_code_helpers() {
        // Success
        assert!(is_success_status(200));
        assert!(is_success_status(201));
        assert!(is_success_status(299));
        assert!(!is_success_status(300));
        assert!(!is_success_status(199));

        // Client error
        assert!(is_client_error(400));
        assert!(is_client_error(404));
        assert!(is_client_error(499));
        assert!(!is_client_error(500));
        assert!(!is_client_error(399));

        // Server error
        assert!(is_server_error(500));
        assert!(is_server_error(502));
        assert!(is_server_error(599));
        assert!(!is_server_error(400));
        assert!(!is_server_error(600));

        // Redirect
        assert!(is_redirect_status(301));
        assert!(is_redirect_status(302));
        assert!(is_redirect_status(399));
        assert!(!is_redirect_status(200));
        assert!(!is_redirect_status(400));
    }
}
