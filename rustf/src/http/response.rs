use crate::error::Result;
use hyper::StatusCode;
use serde::Serialize;
use std::path::Path;

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
        let path = path.as_ref();

        // Read file contents
        let contents = std::fs::read(path).map_err(crate::error::Error::Io)?;

        // Determine content type from file extension
        let content_type = Self::guess_content_type(path);

        // Set filename for download
        let filename = download_name
            .or_else(|| path.file_name().and_then(|n| n.to_str()))
            .unwrap_or("download");

        Ok(Self::ok()
            .with_header("Content-Type", &content_type)
            .with_header(
                "Content-Disposition",
                &format!("attachment; filename=\"{}\"", filename),
            )
            .with_header("Content-Length", &contents.len().to_string())
            .with_body(contents))
    }

    /// Send inline file response (view in browser)
    pub fn file_inline<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        // Read file contents
        let contents = std::fs::read(path).map_err(crate::error::Error::Io)?;

        // Determine content type from file extension
        let content_type = Self::guess_content_type(path);

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
            response = response.with_header(
                "Content-Disposition",
                &format!("attachment; filename=\"{}\"", filename),
            );
        }

        response
    }

    /// Send streaming response (Total.js: controller.stream)
    /// Note: This is a simplified version - real streaming would need async support
    pub fn stream(data: Vec<u8>, content_type: &str, download_name: Option<&str>) -> Self {
        // For now, this is the same as binary - would need proper streaming in a full implementation
        Self::binary(data, content_type, download_name).with_header("Transfer-Encoding", "chunked")
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

    pub fn redirect(location: &str) -> Self {
        Self::new(StatusCode::FOUND).with_header("Location", location)
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
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    /// Add a header to an existing response (mutable)
    pub fn add_header(&mut self, name: &str, value: &str) {
        self.headers.push((name.to_string(), value.to_string()));
    }

    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    /// Get the body size in bytes
    pub fn body_size(&self) -> usize {
        self.body.len()
    }

    pub fn into_hyper(self) -> hyper::Response<hyper::Body> {
        let mut builder = hyper::Response::builder().status(self.status);

        for (name, value) in self.headers {
            builder = builder.header(name, value);
        }

        builder
            .body(hyper::Body::from(self.body))
            .unwrap_or_else(|_| hyper::Response::new(hyper::Body::empty()))
    }
}
