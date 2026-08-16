//! Gzip compression middleware for RustF
//!
//! This middleware compresses HTTP responses using gzip when the client
//! signals support via the `Accept-Encoding: gzip` request header.

use crate::context::Context;
use crate::error::Result;
use crate::middleware::OutboundMiddleware;
use async_trait::async_trait;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;

/// Minimum body size (in bytes) to bother compressing.
const MIN_COMPRESS_SIZE: usize = 256;

/// Content-type prefixes / substrings that are worth compressing.
/// Binary formats (images, video, zip, …) are excluded.
const COMPRESSIBLE_TYPES: &[&str] = &[
    "text/",
    "application/json",
    "application/javascript",
    "application/xml",
    "application/xhtml",
    "application/x-www-form-urlencoded",
    "image/svg",
];

/// Outbound middleware that gzip-compresses responses for clients that accept it.
///
/// # Usage
/// ```rust,ignore
/// let app = RustF::new()
///     .middleware_outbound("compression", CompressionMiddleware::new());
/// ```
///
/// Or via `with_compression()` builder helper on `RustF`.
#[derive(Clone)]
pub struct CompressionMiddleware {
    level: Compression,
    enabled: bool,
}

impl CompressionMiddleware {
    /// Default compression (level 6 — good balance of speed vs ratio).
    pub fn new() -> Self {
        Self {
            level: Compression::default(),
            enabled: true,
        }
    }

    /// Fast compression (level 1 — lowest CPU, modest ratio).
    pub fn fast() -> Self {
        Self {
            level: Compression::fast(),
            enabled: true,
        }
    }

    /// Best compression (level 9 — highest ratio, more CPU).
    pub fn best() -> Self {
        Self {
            level: Compression::best(),
            enabled: true,
        }
    }

    /// Disabled (pass-through, no compression applied).
    pub fn disabled() -> Self {
        Self {
            level: Compression::default(),
            enabled: false,
        }
    }

    fn client_accepts_gzip(ctx: &Context) -> bool {
        ctx.req.headers.iter().any(|(k, v)| {
            k.eq_ignore_ascii_case("accept-encoding") && v.to_lowercase().contains("gzip")
        })
    }

    fn content_type_compressible(headers: &[(String, String)]) -> bool {
        for (k, v) in headers {
            if k.eq_ignore_ascii_case("content-type") {
                let lower = v.to_lowercase();
                return COMPRESSIBLE_TYPES
                    .iter()
                    .any(|prefix| lower.contains(prefix));
            }
        }
        // No Content-Type → assume text (safe default)
        true
    }

    fn already_encoded(headers: &[(String, String)]) -> bool {
        headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("content-encoding"))
    }
}

impl Default for CompressionMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OutboundMiddleware for CompressionMiddleware {
    async fn process_response(&self, ctx: &mut Context) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        if !Self::client_accepts_gzip(ctx) {
            return Ok(());
        }

        if let Some(response) = ctx.res.as_mut() {
            // Streaming bodies pass through uncompressed. Their bytes do not
            // exist yet when outbound middleware runs, so there is nothing to
            // read; compressing them needs a flush-per-chunk encoder wrapped
            // around the stream, which is a separate change.
            let Some(plain) = response.body.as_slice() else {
                return Ok(());
            };

            if plain.len() < MIN_COMPRESS_SIZE {
                return Ok(());
            }

            if Self::already_encoded(&response.headers) {
                return Ok(());
            }

            if !Self::content_type_compressible(&response.headers) {
                return Ok(());
            }

            // Compress the body
            let mut encoder = GzEncoder::new(Vec::new(), self.level);
            encoder
                .write_all(plain)
                .map_err(|e| crate::error::Error::internal(format!("gzip write error: {}", e)))?;
            let compressed = encoder
                .finish()
                .map_err(|e| crate::error::Error::internal(format!("gzip finish error: {}", e)))?;

            let compressed_len = compressed.len();
            response.body = compressed.into();

            // Update headers
            // Strong ETags are representation-specific; once we rewrite the
            // body to gzip bytes, any pre-existing ETag for the identity
            // representation is no longer valid.
            response
                .headers
                .retain(|(k, _)| !k.eq_ignore_ascii_case("etag"));
            response
                .headers
                .push(("Content-Encoding".to_string(), "gzip".to_string()));
            response
                .headers
                .push(("Vary".to_string(), "Accept-Encoding".to_string()));

            // Update Content-Length to reflect compressed size
            let compressed_len = compressed_len.to_string();
            for (k, v) in response.headers.iter_mut() {
                if k.eq_ignore_ascii_case("content-length") {
                    *v = compressed_len.clone();
                    return Ok(());
                }
            }
            // Content-Length header wasn't present — add it
            response
                .headers
                .push(("Content-Length".to_string(), compressed_len));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::Request;
    use crate::views::ViewEngine;
    use std::sync::Arc;

    fn make_ctx(accept_encoding: Option<&str>) -> Context {
        let mut request = Request::default();
        if let Some(enc) = accept_encoding {
            request
                .headers
                .insert("accept-encoding".to_string(), enc.to_string());
        }
        let views = Arc::new(ViewEngine::from_directory("views"));
        Context::new(request, views)
    }

    #[tokio::test]
    async fn test_no_compression_when_disabled() {
        let middleware = CompressionMiddleware::disabled();
        let mut ctx = make_ctx(Some("gzip"));
        let body = b"Hello, world! This is a test response body that is long enough.".repeat(5);
        if let Some(res) = ctx.res.as_mut() {
            res.body = body.to_vec().into();
        }
        let original_len = ctx
            .res
            .as_ref()
            .map(|r| r.body.len_hint().unwrap_or(0))
            .unwrap_or(0);
        middleware.process_response(&mut ctx).await.unwrap();
        assert_eq!(
            ctx.res.as_ref().map(|r| r.body.len_hint().unwrap_or(0)),
            Some(original_len)
        );
    }

    #[tokio::test]
    async fn test_no_compression_without_accept_encoding() {
        let middleware = CompressionMiddleware::new();
        let mut ctx = make_ctx(None);
        let body = b"Hello, world!".repeat(100);
        if let Some(res) = ctx.res.as_mut() {
            res.body = body.to_vec().into();
            res.headers
                .push(("content-type".to_string(), "text/html".to_string()));
        }
        let original_len = ctx
            .res
            .as_ref()
            .map(|r| r.body.len_hint().unwrap_or(0))
            .unwrap_or(0);
        middleware.process_response(&mut ctx).await.unwrap();
        assert_eq!(
            ctx.res.as_ref().map(|r| r.body.len_hint().unwrap_or(0)),
            Some(original_len)
        );
    }

    #[tokio::test]
    async fn test_compression_applied_to_text() {
        let middleware = CompressionMiddleware::new();
        let mut ctx = make_ctx(Some("gzip, deflate"));
        let body = b"Hello, world! ".repeat(100);
        let original_len = body.len();
        if let Some(res) = ctx.res.as_mut() {
            res.body = body.to_vec().into();
            res.headers.push((
                "content-type".to_string(),
                "text/html; charset=utf-8".to_string(),
            ));
        }
        middleware.process_response(&mut ctx).await.unwrap();

        let res = ctx.res.as_ref().unwrap();
        // Compressed body must be smaller than original
        assert!(res.body.len_hint().unwrap_or(0) < original_len);
        // Content-Encoding header must be set
        let has_encoding = res
            .headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("content-encoding") && v == "gzip");
        assert!(has_encoding, "Content-Encoding: gzip header missing");
        // Vary header must be set
        let has_vary = res
            .headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("vary") && v.contains("Accept-Encoding"));
        assert!(has_vary, "Vary: Accept-Encoding header missing");
    }

    #[tokio::test]
    async fn test_no_double_encoding() {
        let middleware = CompressionMiddleware::new();
        let mut ctx = make_ctx(Some("gzip"));
        if let Some(res) = ctx.res.as_mut() {
            res.body = b"already compressed".repeat(100).to_vec().into();
            res.headers
                .push(("content-encoding".to_string(), "gzip".to_string()));
            res.headers
                .push(("content-type".to_string(), "text/plain".to_string()));
        }
        let original_len = ctx
            .res
            .as_ref()
            .map(|r| r.body.len_hint().unwrap_or(0))
            .unwrap_or(0);
        middleware.process_response(&mut ctx).await.unwrap();
        // Body unchanged — already encoded
        assert_eq!(
            ctx.res.as_ref().map(|r| r.body.len_hint().unwrap_or(0)),
            Some(original_len)
        );
    }

    #[tokio::test]
    async fn test_small_body_not_compressed() {
        let middleware = CompressionMiddleware::new();
        let mut ctx = make_ctx(Some("gzip"));
        if let Some(res) = ctx.res.as_mut() {
            res.body = b"tiny".to_vec().into();
            res.headers
                .push(("content-type".to_string(), "text/plain".to_string()));
        }
        let original_len = ctx
            .res
            .as_ref()
            .map(|r| r.body.len_hint().unwrap_or(0))
            .unwrap_or(0);
        middleware.process_response(&mut ctx).await.unwrap();
        assert_eq!(
            ctx.res.as_ref().map(|r| r.body.len_hint().unwrap_or(0)),
            Some(original_len)
        );
    }

    #[tokio::test]
    async fn test_image_not_compressed() {
        let middleware = CompressionMiddleware::new();
        let mut ctx = make_ctx(Some("gzip"));
        let body = b"\x89PNG\r\n\x1a\n".repeat(100);
        let original_len = body.len();
        if let Some(res) = ctx.res.as_mut() {
            res.body = body.to_vec().into();
            res.headers
                .push(("content-type".to_string(), "image/png".to_string()));
        }
        middleware.process_response(&mut ctx).await.unwrap();
        assert_eq!(
            ctx.res.as_ref().map(|r| r.body.len_hint().unwrap_or(0)),
            Some(original_len),
            "PNG body should not be compressed"
        );
    }
}
