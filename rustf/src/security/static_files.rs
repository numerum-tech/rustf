//! Secure static file serving with path traversal protection
//!
//! This module provides secure static file serving functionality that prevents
//! path traversal attacks, validates file types, and implements content security measures.

use super::{PathValidator, SecurityConfig};
use crate::error::{Error, Result};
use crate::http::{Request, Response};
use chrono::{TimeZone, Utc};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Secure static file server
pub struct SecureStaticFileServer {
    validator: PathValidator,
    _enable_directory_listing: bool,
    cache_max_age: u32,
}

impl SecureStaticFileServer {
    /// Create a new secure static file server
    pub fn new(base_directory: impl AsRef<Path>, config: &SecurityConfig) -> Result<Self> {
        let validator = PathValidator::new(base_directory, config)?;

        Ok(Self {
            validator,
            _enable_directory_listing: false, // Disabled by default for security
            cache_max_age: 3600,              // 1 hour default cache
        })
    }

    /// Serve a static file securely
    pub async fn serve_file(&self, request_path: &str) -> Result<Response> {
        // Validate and resolve the path
        let file_path = self.validator.validate_path(request_path)?;

        // Check if file is safe to serve
        if !self.validator.is_safe_file(&file_path)? {
            return Err(Error::template("File cannot be served safely".to_string()));
        }

        // Read file content
        let content = tokio::fs::read(&file_path)
            .await
            .map_err(|e| Error::template(format!("Failed to read file: {}", e)))?;

        // Determine MIME type
        let mime_type = self.detect_mime_type(&file_path);

        // Get file metadata for headers
        let metadata = std::fs::metadata(&file_path)
            .map_err(|e| Error::template(format!("Failed to read file metadata: {}", e)))?;

        let last_modified = metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Build secure response
        let mut response = Response::ok();

        // Set content type
        response = response.with_header("Content-Type", mime_type);

        // Set security headers
        response = response.with_header("X-Content-Type-Options", "nosniff");
        response = response.with_header("X-Frame-Options", "DENY");

        // Set cache headers
        response = response.with_header(
            "Cache-Control",
            &format!("public, max-age={}", self.cache_max_age),
        );
        response = response.with_header("Last-Modified", &Self::format_http_date(last_modified));

        // Set content length
        response = response.with_header("Content-Length", &content.len().to_string());

        // For potentially dangerous file types, force download
        if self.should_force_download(&file_path) {
            let filename = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("download");

            response = response.with_header(
                "Content-Disposition",
                &format!("attachment; filename=\"{}\"", filename),
            );
        }

        Ok(response.with_body(content))
    }

    /// Check if file should be forced as download
    fn should_force_download(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
            let ext = extension.to_lowercase();
            matches!(
                ext.as_str(),
                "pdf"
                    | "doc"
                    | "docx"
                    | "xls"
                    | "xlsx"
                    | "ppt"
                    | "pptx"
                    | "zip"
                    | "rar"
                    | "7z"
                    | "tar"
                    | "gz"
                    | "exe"
                    | "dmg"
            )
        } else {
            true // Force download for files without extension
        }
    }

    /// Detect MIME type based on file extension
    fn detect_mime_type(&self, path: &Path) -> &'static str {
        if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
            match extension.to_lowercase().as_str() {
                // Text files
                "html" | "htm" => "text/html; charset=utf-8",
                "css" => "text/css; charset=utf-8",
                "js" => "application/javascript; charset=utf-8",
                "json" => "application/json; charset=utf-8",
                "txt" => "text/plain; charset=utf-8",
                "xml" => "application/xml; charset=utf-8",
                "md" => "text/markdown; charset=utf-8",

                // Images
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "svg" => "image/svg+xml",
                "ico" => "image/x-icon",
                "webp" => "image/webp",

                // Fonts
                "woff" => "font/woff",
                "woff2" => "font/woff2",
                "ttf" => "font/ttf",
                "eot" => "application/vnd.ms-fontobject",

                // Documents
                "pdf" => "application/pdf",
                "doc" => "application/msword",
                "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                "xls" => "application/vnd.ms-excel",
                "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",

                // Archives
                "zip" => "application/zip",
                "rar" => "application/x-rar-compressed",
                "7z" => "application/x-7z-compressed",
                "tar" => "application/x-tar",
                "gz" => "application/gzip",

                _ => "application/octet-stream",
            }
        } else {
            "application/octet-stream"
        }
    }

    /// Format timestamp as HTTP date (RFC 7231 format)
    fn format_http_date(timestamp: u64) -> String {
        // Convert timestamp to DateTime<Utc>
        let datetime = Utc
            .timestamp_opt(timestamp as i64, 0)
            .single()
            .unwrap_or_else(|| {
                Utc.timestamp_opt(0, 0)
                    .single()
                    .unwrap_or_else(Utc::now)
            });

        // Format according to RFC 7231: "Sun, 06 Nov 1994 08:49:37 GMT"
        datetime.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
    }

    /// Handle conditional requests (If-Modified-Since, ETag)
    pub fn handle_conditional_request(
        &self,
        request: &Request,
        file_path: &Path,
    ) -> Result<Option<Response>> {
        // Check If-Modified-Since header
        if let Some(if_modified_since_header) = request.headers.get("if-modified-since") {
            let metadata = std::fs::metadata(file_path)
                .map_err(|e| Error::template(format!("Failed to read file metadata: {}", e)))?;

            let last_modified = metadata
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            // Parse the If-Modified-Since header (basic HTTP date parsing)
            if let Some(if_modified_timestamp) = Self::parse_http_date(if_modified_since_header) {
                // If file hasn't been modified since the client's cache date, return 304
                if last_modified <= if_modified_timestamp {
                    return Ok(Some(Response::not_modified()));
                }
            }
        }

        Ok(None)
    }

    /// Parse HTTP date string to timestamp (RFC 7231/2616 compliant)
    fn parse_http_date(date_str: &str) -> Option<u64> {
        // HTTP/1.1 requires support for three date formats:
        // 1. RFC 7231: "Sun, 06 Nov 1994 08:49:37 GMT"
        // 2. RFC 850: "Sunday, 06-Nov-94 08:49:37 GMT"
        // 3. asctime(): "Sun Nov  6 08:49:37 1994"

        // Try RFC 7231 format first (most common) - strip GMT and parse as naive, then treat as UTC
        if let Some(without_gmt) = date_str.strip_suffix(" GMT") {
            if let Ok(naive_dt) =
                chrono::NaiveDateTime::parse_from_str(without_gmt, "%a, %d %b %Y %H:%M:%S")
            {
                let datetime = Utc.from_utc_datetime(&naive_dt);
                let timestamp = datetime.timestamp();
                if timestamp >= 0 {
                    return Some(timestamp as u64);
                }
            }

            // Try RFC 850 format
            if let Ok(naive_dt) =
                chrono::NaiveDateTime::parse_from_str(without_gmt, "%A, %d-%b-%y %H:%M:%S")
            {
                let datetime = Utc.from_utc_datetime(&naive_dt);
                let timestamp = datetime.timestamp();
                if timestamp >= 0 {
                    return Some(timestamp as u64);
                }
            }
        }

        // Try asctime() format (no timezone specified, assume GMT)
        if let Ok(naive_dt) =
            chrono::NaiveDateTime::parse_from_str(date_str, "%a %b %e %H:%M:%S %Y")
        {
            let datetime = Utc.from_utc_datetime(&naive_dt);
            let timestamp = datetime.timestamp();
            if timestamp >= 0 {
                return Some(timestamp as u64);
            }
        }

        None
    }
}

/// Static file middleware for automatic static file serving
pub struct StaticFileMiddleware {
    server: SecureStaticFileServer,
    url_prefix: String,
}

impl StaticFileMiddleware {
    /// Create new static file middleware
    pub fn new(
        base_directory: impl AsRef<Path>,
        url_prefix: &str,
        config: &SecurityConfig,
    ) -> Result<Self> {
        let server = SecureStaticFileServer::new(base_directory, config)?;

        Ok(Self {
            server,
            url_prefix: url_prefix.trim_end_matches('/').to_string(),
        })
    }

    /// Check if request should be handled by static file middleware
    pub fn should_handle(&self, request_path: &str) -> bool {
        if self.url_prefix.is_empty() {
            return true;
        }

        // Must start with prefix and have content after it
        request_path.starts_with(&self.url_prefix)
            && request_path.len() > self.url_prefix.len()
            && request_path.chars().nth(self.url_prefix.len()) == Some('/')
    }

    /// Handle static file request
    pub async fn handle_request(&self, request_path: &str) -> Result<Response> {
        // Remove prefix from path
        let file_path = if self.url_prefix.is_empty() {
            request_path.to_string()
        } else {
            request_path
                .strip_prefix(&self.url_prefix)
                .unwrap_or(request_path)
                .to_string()
        };

        self.server.serve_file(&file_path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_secure_static_file_server() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create test files
        fs::write(
            base_path.join("test.html"),
            "<html><body>Test</body></html>",
        )
        .unwrap();
        fs::write(base_path.join("style.css"), "body { color: red; }").unwrap();

        let config = SecurityConfig::default();
        let server = SecureStaticFileServer::new(base_path, &config).unwrap();

        // Test MIME type detection
        assert_eq!(
            server.detect_mime_type(&base_path.join("test.html")),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            server.detect_mime_type(&base_path.join("style.css")),
            "text/css; charset=utf-8"
        );
        assert_eq!(
            server.detect_mime_type(&base_path.join("unknown.xyz")),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_static_file_middleware() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        fs::write(base_path.join("app.js"), "console.log('test');").unwrap();

        let config = SecurityConfig::default();
        let middleware = StaticFileMiddleware::new(base_path, "/static", &config).unwrap();

        // Test path matching
        assert!(middleware.should_handle("/static/app.js"));
        assert!(!middleware.should_handle("/api/data"));
        assert!(!middleware.should_handle("/static"));
    }

    #[tokio::test]
    async fn test_path_traversal_prevention() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        fs::write(base_path.join("safe.txt"), "safe content").unwrap();

        let config = SecurityConfig::default();
        let server = SecureStaticFileServer::new(base_path, &config).unwrap();

        // Test that path traversal attempts are blocked
        assert!(server.serve_file("../../../etc/passwd").await.is_err());
        assert!(server
            .serve_file("..\\..\\windows\\system32\\config\\sam")
            .await
            .is_err());
        assert!(server
            .serve_file("safe.txt/../../../etc/passwd")
            .await
            .is_err());

        // Test that safe files can be served
        assert!(server.serve_file("safe.txt").await.is_ok());
    }

    #[test]
    fn test_http_date_formatting() {
        // Test with known timestamp: 1994-11-06 08:49:37 GMT
        let dt_1994 = chrono::Utc
            .with_ymd_and_hms(1994, 11, 6, 8, 49, 37)
            .unwrap();
        let timestamp_1994 = dt_1994.timestamp() as u64;
        let formatted = SecureStaticFileServer::format_http_date(timestamp_1994);
        assert_eq!(formatted, "Sun, 06 Nov 1994 08:49:37 GMT");

        // Test with Unix epoch
        let formatted_epoch = SecureStaticFileServer::format_http_date(0);
        assert_eq!(formatted_epoch, "Thu, 01 Jan 1970 00:00:00 GMT");

        // Test with leap year date: 2000-02-29 12:00:00 GMT
        let dt_leap = chrono::Utc.with_ymd_and_hms(2000, 2, 29, 12, 0, 0).unwrap();
        let leap_year_timestamp = dt_leap.timestamp() as u64;
        let formatted_leap = SecureStaticFileServer::format_http_date(leap_year_timestamp);
        assert_eq!(formatted_leap, "Tue, 29 Feb 2000 12:00:00 GMT");

        // Test with recent date: 2023-12-25 15:30:45 GMT
        let dt_recent = chrono::Utc
            .with_ymd_and_hms(2023, 12, 25, 15, 30, 45)
            .unwrap();
        let recent_timestamp = dt_recent.timestamp() as u64;
        let formatted_recent = SecureStaticFileServer::format_http_date(recent_timestamp);
        assert_eq!(formatted_recent, "Mon, 25 Dec 2023 15:30:45 GMT");
    }

    #[test]
    fn test_http_date_parsing() {
        // Calculate correct expected timestamp
        let expected_dt = chrono::Utc
            .with_ymd_and_hms(1994, 11, 6, 8, 49, 37)
            .unwrap();
        let expected_timestamp = expected_dt.timestamp() as u64;

        // Test RFC 7231 format: "Sun, 06 Nov 1994 08:49:37 GMT"
        let rfc7231_date = "Sun, 06 Nov 1994 08:49:37 GMT";
        let parsed_rfc7231 = SecureStaticFileServer::parse_http_date(rfc7231_date);
        assert_eq!(parsed_rfc7231, Some(expected_timestamp));

        // Test RFC 850 format: "Sunday, 06-Nov-94 08:49:37 GMT"
        let rfc850_date = "Sunday, 06-Nov-94 08:49:37 GMT";
        let parsed_rfc850 = SecureStaticFileServer::parse_http_date(rfc850_date);
        assert_eq!(parsed_rfc850, Some(expected_timestamp));

        // Test asctime() format: "Sun Nov  6 08:49:37 1994"
        let asctime_date = "Sun Nov  6 08:49:37 1994";
        let parsed_asctime = SecureStaticFileServer::parse_http_date(asctime_date);
        assert_eq!(parsed_asctime, Some(expected_timestamp));

        // Test Unix epoch
        let epoch_date = "Thu, 01 Jan 1970 00:00:00 GMT";
        let parsed_epoch = SecureStaticFileServer::parse_http_date(epoch_date);
        assert_eq!(parsed_epoch, Some(0));

        // Test invalid date formats
        assert_eq!(
            SecureStaticFileServer::parse_http_date("invalid date"),
            None
        );
        assert_eq!(SecureStaticFileServer::parse_http_date(""), None);
        assert_eq!(
            SecureStaticFileServer::parse_http_date("Not a date at all"),
            None
        );

        // Test date before Unix epoch (should return None)
        let pre_epoch_date = "Wed, 31 Dec 1969 23:59:59 GMT";
        assert_eq!(
            SecureStaticFileServer::parse_http_date(pre_epoch_date),
            None
        );
    }

    #[test]
    fn test_http_date_roundtrip() {
        // Test that formatting and parsing are consistent
        let test_datetimes = [
            chrono::Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap(), // Unix epoch
            chrono::Utc
                .with_ymd_and_hms(1994, 11, 6, 8, 49, 37)
                .unwrap(), // 1994-11-06 08:49:37 GMT
            chrono::Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap(), // 2000-01-01 00:00:00 GMT
            chrono::Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap(), // 2020-01-01 00:00:00 GMT
            chrono::Utc
                .with_ymd_and_hms(2023, 12, 25, 15, 30, 45)
                .unwrap(), // 2023-12-25 15:30:45 GMT
        ];

        for dt in &test_datetimes {
            let timestamp = dt.timestamp() as u64;
            let formatted = SecureStaticFileServer::format_http_date(timestamp);
            let parsed = SecureStaticFileServer::parse_http_date(&formatted);
            assert_eq!(
                parsed,
                Some(timestamp),
                "Roundtrip failed for timestamp {}: formatted='{}', parsed={:?}",
                timestamp,
                formatted,
                parsed
            );
        }
    }

    #[test]
    fn test_http_date_edge_cases() {
        // Test leap year dates with correct timestamps
        let leap_years = [
            (
                "Sat, 29 Feb 1992 12:00:00 GMT",
                chrono::Utc.with_ymd_and_hms(1992, 2, 29, 12, 0, 0).unwrap(),
            ),
            (
                "Tue, 29 Feb 2000 12:00:00 GMT",
                chrono::Utc.with_ymd_and_hms(2000, 2, 29, 12, 0, 0).unwrap(),
            ),
            (
                "Mon, 29 Feb 2016 12:00:00 GMT",
                chrono::Utc.with_ymd_and_hms(2016, 2, 29, 12, 0, 0).unwrap(),
            ),
        ];

        for (date_str, expected_dt) in &leap_years {
            let expected_timestamp = expected_dt.timestamp() as u64;
            let parsed = SecureStaticFileServer::parse_http_date(date_str);
            assert_eq!(
                parsed,
                Some(expected_timestamp),
                "Failed to parse leap year date: {}",
                date_str
            );

            let formatted = SecureStaticFileServer::format_http_date(expected_timestamp);
            assert_eq!(
                &formatted, date_str,
                "Formatting mismatch for timestamp {}",
                expected_timestamp
            );
        }

        // Test century boundary dates
        let dt_1999 = chrono::Utc
            .with_ymd_and_hms(1999, 12, 31, 23, 59, 59)
            .unwrap();
        let formatted_1999 = SecureStaticFileServer::format_http_date(dt_1999.timestamp() as u64);
        assert_eq!(formatted_1999, "Fri, 31 Dec 1999 23:59:59 GMT");

        let dt_2000 = chrono::Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();
        let formatted_2000 = SecureStaticFileServer::format_http_date(dt_2000.timestamp() as u64);
        assert_eq!(formatted_2000, "Sat, 01 Jan 2000 00:00:00 GMT");
    }
}
