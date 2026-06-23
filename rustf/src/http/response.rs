use crate::error::Result;
use bytes::Bytes;
use http_body_util::Full;
use hyper::header::{HeaderName, HeaderValue};
use hyper::StatusCode;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use serde::Serialize;
use std::path::{Path, PathBuf};

const RFC5987_ATTR_CHARS: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'%')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b',')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'{')
    .add(b'}');

#[derive(Debug, Clone)]
pub struct Response {
    pub status: StatusCode,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn ok() -> Self {
        Self::new(StatusCode::OK)
    }

    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND).with_body("Not Found".as_bytes().to_vec())
    }

    pub fn internal_error() -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR)
            .with_body("Internal Server Error".as_bytes().to_vec())
    }

    // Total.js-style HTTP error responses

    /// 400 Bad Request
    pub fn bad_request(message: Option<&str>) -> Self {
        let body = message.unwrap_or("Bad Request");
        Self::new(StatusCode::BAD_REQUEST)
            .with_header("Content-Type", "text/plain; charset=utf-8")
            .with_body(body.as_bytes().to_vec())
    }

    /// 401 Unauthorized
    pub fn unauthorized(message: Option<&str>) -> Self {
        let body = message.unwrap_or("Unauthorized");
        Self::new(StatusCode::UNAUTHORIZED)
            .with_header("Content-Type", "text/plain; charset=utf-8")
            .with_body(body.as_bytes().to_vec())
    }

    /// 403 Forbidden
    pub fn forbidden(message: Option<&str>) -> Self {
        let body = message.unwrap_or("Forbidden");
        Self::new(StatusCode::FORBIDDEN)
            .with_header("Content-Type", "text/plain; charset=utf-8")
            .with_body(body.as_bytes().to_vec())
    }

    /// 404 Not Found (with custom message)
    pub fn not_found_with_message(message: Option<&str>) -> Self {
        let body = message.unwrap_or("Not Found");
        Self::new(StatusCode::NOT_FOUND)
            .with_header("Content-Type", "text/plain; charset=utf-8")
            .with_body(body.as_bytes().to_vec())
    }

    /// 409 Conflict
    pub fn conflict(message: Option<&str>) -> Self {
        let body = message.unwrap_or("Conflict");
        Self::new(StatusCode::CONFLICT)
            .with_header("Content-Type", "text/plain; charset=utf-8")
            .with_body(body.as_bytes().to_vec())
    }

    /// 500 Internal Server Error (with custom message)
    pub fn internal_server_error(message: Option<&str>) -> Self {
        let body = message.unwrap_or("Internal Server Error");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR)
            .with_header("Content-Type", "text/plain; charset=utf-8")
            .with_body(body.as_bytes().to_vec())
    }

    /// 501 Not Implemented
    pub fn not_implemented(message: Option<&str>) -> Self {
        let body = message.unwrap_or("Not Implemented");
        Self::new(StatusCode::NOT_IMPLEMENTED)
            .with_header("Content-Type", "text/plain; charset=utf-8")
            .with_body(body.as_bytes().to_vec())
    }

    /// 204 No Content
    pub fn no_content() -> Self {
        Self::new(StatusCode::NO_CONTENT)
    }

    /// 304 Not Modified
    pub fn not_modified() -> Self {
        Self::new(StatusCode::NOT_MODIFIED)
    }

    /// Generic success response with optional data
    pub fn success<T: Serialize>(data: Option<T>) -> Result<Self> {
        match data {
            Some(data) => Self::json(data),
            None => Ok(Self::new(StatusCode::OK)
                .with_header("Content-Type", "application/json")
                .with_body(b"{\"success\":true}".to_vec())),
        }
    }

    // Total.js-style file and binary responses

    /// Send file download response (Total.js: controller.file)
    pub fn file_download<P: AsRef<Path>>(path: P, download_name: Option<&str>) -> Result<Self> {
        let base_dir = Self::default_private_base_dir()?;
        Self::file_download_from(base_dir, path, download_name)
    }

    /// Send file download response constrained to a base directory.
    pub fn file_download_from<B: AsRef<Path>, P: AsRef<Path>>(
        base_dir: B,
        path: P,
        download_name: Option<&str>,
    ) -> Result<Self> {
        let path = path.as_ref();
        let safe_path = Self::resolve_contained_file(base_dir.as_ref(), path)?;

        // Read file contents
        let contents = std::fs::read(&safe_path).map_err(crate::error::Error::Io)?;

        // Determine content type from file extension
        let content_type = Self::guess_content_type(&safe_path);

        // Set filename for download
        let filename = download_name
            .or_else(|| safe_path.file_name().and_then(|n| n.to_str()))
            .unwrap_or("download");
        let content_disposition = Self::build_attachment_content_disposition(filename);

        Ok(Self::ok()
            .with_header("Content-Type", &content_type)
            .with_header("Content-Disposition", &content_disposition)
            .with_header("Content-Length", &contents.len().to_string())
            .with_body(contents))
    }

    /// Send inline file response (view in browser)
    pub fn file_inline<P: AsRef<Path>>(path: P) -> Result<Self> {
        let base_dir = Self::default_private_base_dir()?;
        Self::file_inline_from(base_dir, path)
    }

    /// Send inline file response constrained to a base directory.
    pub fn file_inline_from<B: AsRef<Path>, P: AsRef<Path>>(base_dir: B, path: P) -> Result<Self> {
        let path = path.as_ref();
        let safe_path = Self::resolve_contained_file(base_dir.as_ref(), path)?;

        // Read file contents
        let contents = std::fs::read(&safe_path).map_err(crate::error::Error::Io)?;

        // Determine content type from file extension
        let content_type = Self::guess_content_type(&safe_path);

        Ok(Self::ok()
            .with_header("Content-Type", &content_type)
            .with_header("Content-Length", &contents.len().to_string())
            .with_body(contents))
    }

    /// Send binary data response (Total.js: controller.binary)
    pub fn binary(data: Vec<u8>, content_type: &str, download_name: Option<&str>) -> Self {
        let mut response = Self::ok()
            .with_header("Content-Type", content_type)
            .with_header("Content-Length", &data.len().to_string())
            .with_body(data);

        // Add download headers if filename provided
        if let Some(filename) = download_name {
            let content_disposition = Self::build_attachment_content_disposition(filename);
            response = response.with_header("Content-Disposition", &content_disposition);
        }

        response
    }

    /// Send streaming response (Total.js: controller.stream)
    /// Note: This is a simplified version - real streaming would need async support
    pub fn stream(data: Vec<u8>, content_type: &str, download_name: Option<&str>) -> Self {
        // For now, this is the same as binary - would need proper streaming in a full implementation
        Self::binary(data, content_type, download_name).with_header("Transfer-Encoding", "chunked")
    }

    fn resolve_contained_file(base_dir: &Path, path: &Path) -> Result<PathBuf> {
        let canonical_base = base_dir.canonicalize().map_err(crate::error::Error::Io)?;
        let candidate = if path.is_absolute() {
            path.to_path_buf()
        } else {
            canonical_base.join(path)
        };
        let canonical_path = candidate.canonicalize().map_err(crate::error::Error::Io)?;

        if !canonical_path.starts_with(&canonical_base) {
            return Err(crate::error::Error::InvalidInput(
                "File path escapes the allowed base directory".to_string(),
            ));
        }

        let metadata = std::fs::metadata(&canonical_path).map_err(crate::error::Error::Io)?;
        if !metadata.is_file() {
            return Err(crate::error::Error::InvalidInput(
                "Requested path is not a regular file".to_string(),
            ));
        }

        Ok(canonical_path)
    }

    fn default_private_base_dir() -> Result<PathBuf> {
        if let Some(private_dir) = crate::configuration::CONF::get_string("private.directory") {
            return Ok(PathBuf::from(private_dir));
        }

        Ok(std::env::current_dir()
            .map_err(crate::error::Error::Io)?
            .join("private"))
    }

    fn is_safe_local_redirect(location: &str) -> bool {
        location.starts_with('/')
            && !location.starts_with("//")
            && !location.contains("://")
            && !location.contains('\\')
    }

    fn build_attachment_content_disposition(filename: &str) -> String {
        let fallback = Self::sanitize_download_filename(filename);
        let encoded = utf8_percent_encode(filename, RFC5987_ATTR_CHARS).to_string();
        format!(
            "attachment; filename=\"{}\"; filename*=UTF-8''{}",
            fallback, encoded
        )
    }

    fn sanitize_download_filename(filename: &str) -> String {
        let sanitized: String = filename
            .chars()
            .filter(|&ch| !matches!(ch, '\r' | '\n' | '"' | '\\' | '\0'))
            .map(|ch| if ch == '/' { '_' } else { ch })
            .collect();

        let trimmed = sanitized.trim();
        if trimmed.is_empty() {
            "download".to_string()
        } else {
            trimmed.to_string()
        }
    }

    fn redirect_with_status(location: &str, permanent: bool) -> Result<Self> {
        if !Self::is_safe_local_redirect(location) {
            return Err(crate::error::Error::InvalidInput(
                "Redirect target must be a local absolute path".to_string(),
            ));
        }

        Ok(Self::redirect_external_with_status(location, permanent))
    }

    fn redirect_external_with_status(location: &str, permanent: bool) -> Self {
        let status = if permanent {
            StatusCode::MOVED_PERMANENTLY
        } else {
            StatusCode::FOUND
        };

        Self::new(status).with_header("Location", location)
    }

    /// Guess content type from file extension
    fn guess_content_type(path: &Path) -> String {
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            match extension.to_lowercase().as_str() {
                // Images
                "jpg" | "jpeg" => "image/jpeg",
                "png" => "image/png",
                "gif" => "image/gif",
                "webp" => "image/webp",
                "svg" => "image/svg+xml",
                "ico" => "image/x-icon",

                // Documents
                "pdf" => "application/pdf",
                "doc" => "application/msword",
                "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                "xls" => "application/vnd.ms-excel",
                "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                "ppt" => "application/vnd.ms-powerpoint",
                "pptx" => {
                    "application/vnd.openxmlformats-officedocument.presentationml.presentation"
                }

                // Text
                "txt" => "text/plain",
                "html" | "htm" => "text/html",
                "css" => "text/css",
                "js" => "application/javascript",
                "json" => "application/json",
                "xml" => "application/xml",
                "csv" => "text/csv",

                // Audio
                "mp3" => "audio/mpeg",
                "wav" => "audio/wav",
                "ogg" => "audio/ogg",
                "m4a" => "audio/mp4",

                // Video
                "mp4" => "video/mp4",
                "avi" => "video/x-msvideo",
                "mov" => "video/quicktime",
                "wmv" => "video/x-ms-wmv",
                "webm" => "video/webm",

                // Archives
                "zip" => "application/zip",
                "rar" => "application/vnd.rar",
                "7z" => "application/x-7z-compressed",
                "tar" => "application/x-tar",
                "gz" => "application/gzip",

                _ => "application/octet-stream",
            }
        } else {
            "application/octet-stream"
        }
        .to_string()
    }

    pub fn redirect(location: &str) -> Result<Self> {
        Self::redirect_with_status(location, false)
    }

    pub fn redirect_permanent(location: &str) -> Result<Self> {
        Self::redirect_with_status(location, true)
    }

    pub fn redirect_external(location: &str) -> Self {
        Self::redirect_external_with_status(location, false)
    }

    pub fn redirect_external_permanent(location: &str) -> Self {
        Self::redirect_external_with_status(location, true)
    }

    pub fn json<T: Serialize>(data: T) -> Result<Self> {
        let json_string = serde_json::to_string(&data)?;
        Ok(Self::ok()
            .with_header("Content-Type", "application/json")
            .with_body(json_string.into_bytes()))
    }

    pub fn html(content: impl Into<String>) -> Self {
        Self::ok()
            .with_header("Content-Type", "text/html; charset=utf-8")
            .with_body(content.into().into_bytes())
    }

    pub fn text(content: impl Into<String>) -> Self {
        Self::ok()
            .with_header("Content-Type", "text/plain; charset=utf-8")
            .with_body(content.into().into_bytes())
    }

    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.add_header(name, value);
        self
    }

    /// Add a header to an existing response (mutable)
    pub fn add_header(&mut self, name: &str, value: &str) {
        if Self::validate_header(name, value).is_some() {
            self.headers.push((name.to_string(), value.to_string()));
        }
    }

    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    /// Get the body size in bytes
    pub fn body_size(&self) -> usize {
        self.body.len()
    }

    pub fn into_hyper(self) -> hyper::Response<Full<Bytes>> {
        let mut response = hyper::Response::new(Full::new(Bytes::from(self.body)));
        *response.status_mut() = self.status;

        for (name, value) in self.headers {
            match (
                HeaderName::try_from(name.as_str()),
                HeaderValue::from_str(&value),
            ) {
                (Ok(header_name), Ok(header_value)) => {
                    response.headers_mut().append(header_name, header_value);
                }
                _ => {
                    log::warn!("Skipping invalid response header '{}'", name);
                }
            }
        }

        response
    }

    fn validate_header(name: &str, value: &str) -> Option<()> {
        match (HeaderName::try_from(name), HeaderValue::from_str(value)) {
            (Ok(_), Ok(_)) => Some(()),
            _ => {
                log::warn!("Rejected invalid response header '{}'", name);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Response;
    use crate::error::Error;
    use hyper::StatusCode;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rustf-response-{name}-{unique}"))
    }

    #[test]
    fn file_download_from_allows_file_under_base_dir() {
        let base_dir = temp_test_dir("allowed");
        let nested_dir = base_dir.join("uploads");
        fs::create_dir_all(&nested_dir).unwrap();
        let file_path = nested_dir.join("report.txt");
        fs::write(&file_path, b"hello").unwrap();

        let response = Response::file_download_from(&base_dir, "uploads/report.txt", None).unwrap();

        assert_eq!(response.body, b"hello");
        assert!(response
            .headers
            .iter()
            .any(|(k, v)| k == "Content-Disposition" && v.contains("report.txt")));

        fs::remove_dir_all(&base_dir).unwrap();
    }

    #[test]
    fn file_download_from_rejects_path_traversal() {
        let root = temp_test_dir("traversal");
        let base_dir = root.join("base");
        let secret_path = root.join("secret.txt");
        fs::create_dir_all(&base_dir).unwrap();
        fs::write(&secret_path, b"secret").unwrap();

        let err = Response::file_download_from(&base_dir, "../secret.txt", None).unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn file_inline_from_rejects_absolute_path_outside_base_dir() {
        let root = temp_test_dir("absolute");
        let base_dir = root.join("base");
        let outside_dir = root.join("outside");
        fs::create_dir_all(&base_dir).unwrap();
        fs::create_dir_all(&outside_dir).unwrap();
        let outside_file = outside_dir.join("secret.txt");
        fs::write(&outside_file, b"secret").unwrap();

        let err = Response::file_inline_from(&base_dir, &outside_file).unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn invalid_header_does_not_replace_response_with_blank_200() {
        let response = Response::text("hello").with_header("X-Test", "ok\r\nbad");
        let hyper_response = response.into_hyper();

        assert_eq!(hyper_response.status(), StatusCode::OK);
        assert!(hyper_response.headers().get("X-Test").is_none());
    }

    #[test]
    fn redirect_drops_invalid_location_header_value() {
        let response = Response::redirect_external("https://example.com\r\nbad");
        let hyper_response = response.into_hyper();

        assert_eq!(hyper_response.status(), StatusCode::FOUND);
        assert!(hyper_response.headers().get("Location").is_none());
    }

    #[test]
    fn redirect_rejects_external_targets() {
        let err = Response::redirect("https://evil.example/phish").unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));
    }

    #[test]
    fn redirect_permanent_uses_301() {
        let response = Response::redirect_permanent("/moved").unwrap();
        let hyper_response = response.into_hyper();

        assert_eq!(hyper_response.status(), StatusCode::MOVED_PERMANENTLY);
        assert_eq!(hyper_response.headers().get("Location").unwrap(), "/moved");
    }

    #[test]
    fn file_download_escapes_content_disposition_filename() {
        let base_dir = temp_test_dir("filename");
        fs::create_dir_all(&base_dir).unwrap();
        let file_path = base_dir.join("report.txt");
        fs::write(&file_path, b"hello").unwrap();

        let response =
            Response::file_download_from(&base_dir, "report.txt", Some("bad\"name\r\n.txt"))
                .unwrap();
        let content_disposition = response
            .headers
            .iter()
            .find(|(k, _)| k == "Content-Disposition")
            .map(|(_, v)| v.clone())
            .unwrap();

        assert!(!content_disposition.contains('\r'));
        assert!(!content_disposition.contains('\n'));
        assert!(content_disposition.contains("filename=\"badname.txt\""));
        assert!(content_disposition.contains("filename*=UTF-8''"));

        fs::remove_dir_all(&base_dir).unwrap();
    }
}
