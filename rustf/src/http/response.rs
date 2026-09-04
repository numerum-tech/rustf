use crate::error::{Error, Result};
use crate::http::body::{Body, BodyStream};
use crate::http::sse::SseItem;
use bytes::Bytes;
use futures::TryStreamExt;
use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::Frame;
use hyper::header::{HeaderName, HeaderValue};
use hyper::StatusCode;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
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

/// An HTTP response.
///
/// The body is either buffered or streamed — see [`Body`]. `Response` is
/// deliberately not `Clone`: a streaming body is a one-shot source of bytes
/// and cannot be duplicated.
#[derive(Debug)]
pub struct Response {
    pub status: StatusCode,
    pub headers: Vec<(String, String)>,
    pub body: Body,
    /// Length to advertise as `Content-Length` when the body is empty on
    /// purpose — a `HEAD` response, or a `304` — so the client learns the
    /// size of the representation it is not receiving. Private, and kept off
    /// `headers`, so it cannot be confused with a caller-set framing header.
    advertised_len: Option<u64>,
}

impl Response {
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Body::empty(),
            advertised_len: None,
        }
    }

    /// Advertise `len` as the `Content-Length` of an empty-bodied response.
    ///
    /// Used for `HEAD` (and valid for `304`): the body is deliberately empty
    /// but the client should see the length a `GET` would have sent. Ignored
    /// unless the body is empty when the response is sent.
    pub fn advertise_content_length(mut self, len: u64) -> Self {
        self.advertised_len = Some(len);
        self
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

    /// 403 Forbidden
    pub fn forbidden(message: Option<&str>) -> Self {
        let body = message.unwrap_or("Forbidden");
        Self::new(StatusCode::FORBIDDEN)
            .with_header("Content-Type", "text/plain; charset=utf-8")
            .with_body(body.as_bytes().to_vec())
    }

    /// 304 Not Modified
    pub fn not_modified() -> Self {
        Self::new(StatusCode::NOT_MODIFIED)
    }

    // Total.js-style file and binary responses

    /// Send file download response (Total.js: controller.file)
    pub async fn file_download<P: AsRef<Path>>(
        path: P,
        download_name: Option<&str>,
    ) -> Result<Self> {
        let base_dir = Self::default_private_base_dir()?;
        Self::file_download_from(base_dir, path, download_name).await
    }

    /// Send file download response constrained to a base directory.
    pub async fn file_download_from<B: AsRef<Path>, P: AsRef<Path>>(
        base_dir: B,
        path: P,
        download_name: Option<&str>,
    ) -> Result<Self> {
        let path = path.as_ref();
        let safe_path = Self::resolve_contained_file(base_dir.as_ref(), path).await?;

        // Read file contents off the blocking pool so the worker thread stays
        // free to drive other connections.
        let contents = tokio::fs::read(&safe_path)
            .await
            .map_err(crate::error::Error::Io)?;

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
    pub async fn file_inline<P: AsRef<Path>>(path: P) -> Result<Self> {
        let base_dir = Self::default_private_base_dir()?;
        Self::file_inline_from(base_dir, path).await
    }

    /// Send inline file response constrained to a base directory.
    pub async fn file_inline_from<B: AsRef<Path>, P: AsRef<Path>>(
        base_dir: B,
        path: P,
    ) -> Result<Self> {
        let path = path.as_ref();
        let safe_path = Self::resolve_contained_file(base_dir.as_ref(), path).await?;

        // Read file contents off the blocking pool so the worker thread stays
        // free to drive other connections.
        let contents = tokio::fs::read(&safe_path)
            .await
            .map_err(crate::error::Error::Io)?;

        // Determine content type from file extension
        let content_type = Self::guess_content_type(&safe_path);

        Ok(Self::ok()
            .with_header("Content-Type", &content_type)
            .with_header("Content-Length", &contents.len().to_string())
            .with_body(contents))
    }

    /// Stream a file download instead of buffering it in memory.
    ///
    /// Identical headers to [`Self::file_download`], but memory stays flat at
    /// one 64 KiB chunk regardless of file size. Prefer this for anything
    /// large enough that holding it entirely in memory — once per concurrent
    /// request — would matter.
    ///
    /// Because the body is streamed, the response is committed as soon as the
    /// first chunk goes out. An I/O error partway through cannot become a 500;
    /// the client receives a truncated download under the `200 OK` already
    /// sent.
    pub async fn file_download_stream<P: AsRef<Path>>(
        path: P,
        download_name: Option<&str>,
    ) -> Result<Self> {
        let base_dir = Self::default_private_base_dir()?;
        Self::file_download_stream_from(base_dir, path, download_name).await
    }

    /// Stream a file download, constrained to a base directory.
    pub async fn file_download_stream_from<B: AsRef<Path>, P: AsRef<Path>>(
        base_dir: B,
        path: P,
        download_name: Option<&str>,
    ) -> Result<Self> {
        let safe_path = Self::resolve_contained_file(base_dir.as_ref(), path.as_ref()).await?;
        let (file, size) = Self::open_sized(&safe_path).await?;

        let content_type = Self::guess_content_type(&safe_path);
        let filename = download_name
            .or_else(|| safe_path.file_name().and_then(|n| n.to_str()))
            .unwrap_or("download");
        let content_disposition = Self::build_attachment_content_disposition(filename);

        // `Content-Length` comes from the stream's declared size in
        // `into_hyper`, so it is not set here.
        Ok(Self::ok()
            .with_header("Content-Type", &content_type)
            .with_header("Content-Disposition", &content_disposition)
            .with_body(Body::from_file(file, size)))
    }

    /// Stream a file for inline display instead of buffering it in memory.
    pub async fn file_inline_stream<P: AsRef<Path>>(path: P) -> Result<Self> {
        let base_dir = Self::default_private_base_dir()?;
        Self::file_inline_stream_from(base_dir, path).await
    }

    /// Stream a file for inline display, constrained to a base directory.
    pub async fn file_inline_stream_from<B: AsRef<Path>, P: AsRef<Path>>(
        base_dir: B,
        path: P,
    ) -> Result<Self> {
        let safe_path = Self::resolve_contained_file(base_dir.as_ref(), path.as_ref()).await?;
        let (file, size) = Self::open_sized(&safe_path).await?;
        let content_type = Self::guess_content_type(&safe_path);

        Ok(Self::ok()
            .with_header("Content-Type", &content_type)
            .with_body(Body::from_file(file, size)))
    }

    /// Open a file and read its length, rejecting anything that is not a
    /// regular file.
    ///
    /// Directories and special files (FIFOs, devices) report a length that
    /// does not describe a readable byte count, which would produce a
    /// `Content-Length` the body never satisfies and hang the client.
    async fn open_sized(path: &Path) -> Result<(tokio::fs::File, u64)> {
        let file = tokio::fs::File::open(path)
            .await
            .map_err(crate::error::Error::Io)?;
        let metadata = file.metadata().await.map_err(crate::error::Error::Io)?;

        if !metadata.is_file() {
            return Err(crate::error::Error::InvalidInput(format!(
                "Not a regular file: {}",
                path.display()
            )));
        }

        Ok((file, metadata.len()))
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

    /// Send a streaming response (Total.js: controller.stream).
    ///
    /// Accepts anything that converts into a [`Body`]. A `Vec<u8>` or `String`
    /// is sent buffered, exactly like [`Self::binary`]. A [`BodyStream`] or
    /// `Body::from_stream(..)` is sent as its chunks are produced: a declared
    /// size becomes `Content-Length`, an open-ended stream goes out with
    /// chunked transfer encoding. Framing is decided in `into_hyper`, so no
    /// `Transfer-Encoding` header is set here — setting one by hand next to a
    /// `Content-Length` is a protocol violation.
    ///
    /// ```rust,no_run
    /// use rustf::http::{Body, Response};
    /// use bytes::Bytes;
    ///
    /// let rows = futures::stream::iter(vec![
    ///     Ok(Bytes::from_static(b"id,name\n")),
    ///     Ok(Bytes::from_static(b"1,alice\n")),
    /// ]);
    /// let response = Response::stream(Body::from_stream(rows), "text/csv", Some("users.csv"));
    /// ```
    pub fn stream(body: impl Into<Body>, content_type: &str, download_name: Option<&str>) -> Self {
        let body = body.into();
        let mut response = Self::ok().with_header("Content-Type", content_type);

        // Buffered bodies keep the explicit header `binary` has always sent.
        // Sized streams get theirs in `into_hyper`; unsized ones have none.
        if let Body::Full(bytes) = &body {
            response = response.with_header("Content-Length", &bytes.len().to_string());
        }

        if let Some(filename) = download_name {
            let content_disposition = Self::build_attachment_content_disposition(filename);
            response = response.with_header("Content-Disposition", &content_disposition);
        }

        response.with_body(body)
    }

    /// Send a Server-Sent Events stream.
    ///
    /// `events` yields [`SseEvent`](crate::http::SseEvent)s, or `rustf::Result<SseEvent>` for a
    /// source that can fail (see [`SseItem`]). Each event is encoded in the
    /// SSE wire format as it arrives; the response ends when `events` ends,
    /// or is aborted on the first `Err`. Sets `Content-Type:
    /// text/event-stream`, disables caching, and tells nginx-style proxies
    /// not to buffer (`X-Accel-Buffering: no`) so events reach the browser
    /// immediately.
    ///
    /// If the feed can go quiet for longer than the idle timeout of any proxy
    /// in front of the server, use [`Self::sse_with_keep_alive`] instead.
    pub fn sse<S>(events: S) -> Self
    where
        S: futures::Stream + Send + 'static,
        S::Item: SseItem,
    {
        use futures::StreamExt;

        Self::ok()
            .with_header("Content-Type", "text/event-stream")
            .with_header("Cache-Control", "no-cache")
            .with_header("X-Accel-Buffering", "no")
            .with_body(Body::from_stream(
                events.map(|item| item.into_event().map(|event| event.to_bytes())),
            ))
    }

    /// Send a Server-Sent Events stream, emitting a comment line whenever the
    /// feed has been idle for `keep_alive`.
    ///
    /// See [`crate::http::sse::keep_alive`] for how the interval behaves and
    /// how to choose it.
    pub fn sse_with_keep_alive<S>(events: S, keep_alive: std::time::Duration) -> Self
    where
        S: futures::Stream + Send + 'static,
        S::Item: SseItem,
    {
        Self::sse(crate::http::sse::keep_alive(events, keep_alive))
    }

    /// Resolve `path` inside `base_dir`, rejecting anything that escapes it
    /// (through `..` or a symlink) or is not a regular file.
    ///
    /// All filesystem calls go through `tokio::fs` so a slow or network mount
    /// cannot stall the worker thread that is driving other connections.
    async fn resolve_contained_file(base_dir: &Path, path: &Path) -> Result<PathBuf> {
        let canonical_base = tokio::fs::canonicalize(base_dir)
            .await
            .map_err(crate::error::Error::Io)?;
        let candidate = if path.is_absolute() {
            path.to_path_buf()
        } else {
            canonical_base.join(path)
        };
        let canonical_path = tokio::fs::canonicalize(&candidate)
            .await
            .map_err(crate::error::Error::Io)?;

        if !canonical_path.starts_with(&canonical_base) {
            return Err(crate::error::Error::InvalidInput(
                "File path escapes the allowed base directory".to_string(),
            ));
        }

        let metadata = tokio::fs::metadata(&canonical_path)
            .await
            .map_err(crate::error::Error::Io)?;
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

    /// Set the body.
    ///
    /// Accepts anything convertible into a [`Body`] — `Vec<u8>`, `String`,
    /// `&str`, `Bytes`, or a [`BodyStream`] for streamed responses.
    pub fn with_body(mut self, body: impl Into<Body>) -> Self {
        self.body = body.into();
        self
    }

    /// Stream the body from a chunk stream instead of buffering it.
    ///
    /// Headers and status are sent as soon as the first chunk is produced and
    /// cannot be changed afterwards. Decide status, redirects, and
    /// authorization before calling this.
    pub fn with_stream(mut self, stream: BodyStream) -> Self {
        self.body = Body::Stream(stream);
        self
    }

    /// Body size in bytes, when known without consuming the body.
    ///
    /// `None` for a stream that did not declare its total length.
    pub fn body_size(&self) -> Option<usize> {
        self.body.len_hint()
    }

    // `UnsyncBoxBody` rather than `BoxBody`: a body stream is `Send` but not
    // `Sync`, which is all hyper requires to drive a connection.
    ///
    /// # Framing headers
    ///
    /// This conversion is the single authority on message framing.
    /// `Transfer-Encoding` set on the response is always dropped — hyper
    /// picks chunked for an unsized HTTP/1.1 body and nothing for HTTP/2, and
    /// a hand-written value can only contradict it. A caller-set
    /// `Content-Length` is always dropped: hyper derives the header for a
    /// buffered body, a sized stream supplies its declared size, and an
    /// empty body that stands in for a representation (`HEAD`, `304`) gets
    /// the length recorded by [`Self::advertise_content_length`].
    pub fn into_hyper(self) -> hyper::Response<UnsyncBoxBody<Bytes, Error>> {
        let declared_len = match &self.body {
            Body::Stream(stream) => stream.size(),
            Body::Full(bytes) if bytes.is_empty() => self.advertised_len,
            Body::Full(_) => None,
        };

        let body = match self.body {
            Body::Full(bytes) => Full::new(Bytes::from(bytes))
                .map_err(|never| match never {})
                .boxed_unsync(),
            Body::Stream(stream) => {
                // An error here is the only place a mid-flight failure is
                // observable server-side. Hyper drops the connection so the
                // client sees a truncated body under the already-sent status;
                // without this log the failure would be invisible.
                let chunks = stream
                    .into_enforced_stream()
                    .inspect_err(|error| {
                        log::error!("Response body stream aborted mid-flight: {}", error);
                    })
                    .map_ok(Frame::data);
                StreamBody::new(chunks).boxed_unsync()
            }
        };

        let mut response = hyper::Response::new(body);
        *response.status_mut() = self.status;

        if let Some(len) = declared_len {
            response
                .headers_mut()
                .insert(hyper::header::CONTENT_LENGTH, HeaderValue::from(len));
        }

        for (name, value) in self.headers {
            if name.eq_ignore_ascii_case("transfer-encoding") {
                log::warn!(
                    "Dropping hand-set Transfer-Encoding header; framing is decided by the body"
                );
                continue;
            }
            if name.eq_ignore_ascii_case("content-length") {
                // The body (or `advertised_len`) is the only source of truth;
                // a caller value could only agree with it or be wrong.
                continue;
            }
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

    fn hyper_header(
        response: &hyper::Response<http_body_util::combinators::UnsyncBoxBody<bytes::Bytes, Error>>,
        name: &str,
    ) -> Option<String> {
        response
            .headers()
            .get(name)
            .map(|v| v.to_str().unwrap().to_string())
    }

    #[test]
    fn into_hyper_drops_stale_framing_headers_on_an_unsized_stream() {
        // A stale Content-Length on an open-ended stream would make the client
        // wait for bytes that never come; a hand-set Transfer-Encoding next to
        // it is a contradiction. Both must go.
        let response = Response::ok()
            .with_header("Content-Length", "123")
            .with_header("Transfer-Encoding", "chunked")
            .with_header("X-Keep", "yes")
            .with_body(crate::http::Body::from_stream(futures::stream::empty()));

        let hyper_response = response.into_hyper();
        assert_eq!(hyper_header(&hyper_response, "content-length"), None);
        assert_eq!(hyper_header(&hyper_response, "transfer-encoding"), None);
        assert_eq!(hyper_header(&hyper_response, "x-keep"), Some("yes".into()));
    }

    #[test]
    fn into_hyper_replaces_a_wrong_content_length_on_a_sized_stream() {
        let response = Response::ok()
            .with_header("Content-Length", "123")
            .with_body(crate::http::Body::from_sized_stream(
                futures::stream::iter(vec![Ok(bytes::Bytes::from_static(b"12345"))]),
                5,
            ));

        let hyper_response = response.into_hyper();
        assert_eq!(
            hyper_header(&hyper_response, "content-length"),
            Some("5".into())
        );
        assert_eq!(
            hyper_response
                .headers()
                .get_all("content-length")
                .iter()
                .count(),
            1,
            "exactly one Content-Length may be sent"
        );
    }

    #[test]
    fn into_hyper_drops_a_caller_content_length_on_a_buffered_body() {
        // hyper derives Content-Length from the buffered bytes; a caller value
        // that disagrees would be sent alongside it and corrupt the framing.
        let response = Response::ok()
            .with_header("Content-Length", "999")
            .with_body("four".to_string());

        let hyper_response = response.into_hyper();
        assert_eq!(hyper_header(&hyper_response, "content-length"), None);
        assert_eq!(
            hyper::body::Body::size_hint(hyper_response.body()).exact(),
            Some(4),
            "hyper takes the length from the body itself"
        );
    }

    #[test]
    fn into_hyper_drops_caller_content_length_even_on_an_empty_body() {
        // An empty 200 with a hand-set (and duplicated) Content-Length would
        // leave the client waiting for bytes that never come.
        let response = Response::new(StatusCode::OK)
            .with_header("Content-Length", "999")
            .with_header("Content-Length", "1000")
            .with_body(Vec::new());

        let hyper_response = response.into_hyper();
        assert_eq!(
            hyper_response
                .headers()
                .get_all("content-length")
                .iter()
                .count(),
            0,
            "hyper derives Content-Length: 0 itself; nothing from the caller"
        );
    }

    #[test]
    fn into_hyper_uses_the_advertised_length_for_an_empty_body() {
        // HEAD and 304 responses carry the length of a representation they
        // do not send; that comes from `advertise_content_length`, never
        // from a caller header.
        let response = Response::new(StatusCode::OK)
            .with_header("Content-Length", "999")
            .with_body(Vec::new())
            .advertise_content_length(1_048_576);

        let hyper_response = response.into_hyper();
        assert_eq!(
            hyper_header(&hyper_response, "content-length"),
            Some("1048576".into())
        );
        assert_eq!(
            hyper_response
                .headers()
                .get_all("content-length")
                .iter()
                .count(),
            1
        );
    }

    #[test]
    fn into_hyper_ignores_an_advertised_length_when_a_body_is_present() {
        let response = Response::ok()
            .with_body("four".to_string())
            .advertise_content_length(1_048_576);

        let hyper_response = response.into_hyper();
        assert_eq!(hyper_header(&hyper_response, "content-length"), None);
        assert_eq!(
            hyper::body::Body::size_hint(hyper_response.body()).exact(),
            Some(4)
        );
    }

    fn temp_test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rustf-response-{name}-{unique}"))
    }

    #[tokio::test]
    async fn file_download_from_allows_file_under_base_dir() {
        let base_dir = temp_test_dir("allowed");
        let nested_dir = base_dir.join("uploads");
        fs::create_dir_all(&nested_dir).unwrap();
        let file_path = nested_dir.join("report.txt");
        fs::write(&file_path, b"hello").unwrap();

        let response = Response::file_download_from(&base_dir, "uploads/report.txt", None)
            .await
            .unwrap();

        assert_eq!(response.body.as_slice(), Some(&b"hello"[..]));
        assert!(response
            .headers
            .iter()
            .any(|(k, v)| k == "Content-Disposition" && v.contains("report.txt")));

        fs::remove_dir_all(&base_dir).unwrap();
    }

    #[tokio::test]
    async fn file_download_from_rejects_path_traversal() {
        let root = temp_test_dir("traversal");
        let base_dir = root.join("base");
        let secret_path = root.join("secret.txt");
        fs::create_dir_all(&base_dir).unwrap();
        fs::write(&secret_path, b"secret").unwrap();

        let err = Response::file_download_from(&base_dir, "../secret.txt", None)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));

        fs::remove_dir_all(&root).unwrap();
    }

    #[tokio::test]
    async fn file_inline_from_rejects_absolute_path_outside_base_dir() {
        let root = temp_test_dir("absolute");
        let base_dir = root.join("base");
        let outside_dir = root.join("outside");
        fs::create_dir_all(&base_dir).unwrap();
        fs::create_dir_all(&outside_dir).unwrap();
        let outside_file = outside_dir.join("secret.txt");
        fs::write(&outside_file, b"secret").unwrap();

        let err = Response::file_inline_from(&base_dir, &outside_file)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn invalid_header_does_not_replace_response_with_blank_200() {
        let response = Response::ok()
            .with_body(b"hello".to_vec())
            .with_header("X-Test", "ok\r\nbad");
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

    #[tokio::test]
    async fn file_download_escapes_content_disposition_filename() {
        let base_dir = temp_test_dir("filename");
        fs::create_dir_all(&base_dir).unwrap();
        let file_path = base_dir.join("report.txt");
        fs::write(&file_path, b"hello").unwrap();

        let response =
            Response::file_download_from(&base_dir, "report.txt", Some("bad\"name\r\n.txt"))
                .await
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
