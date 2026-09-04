//! Response body representation.
//!
//! A response body is either fully buffered in memory ([`Body::Full`]) or
//! produced incrementally as a stream of chunks ([`Body::Stream`]).
//!
//! Buffered is the default and covers almost every response: JSON payloads,
//! rendered templates, small static assets. Streaming exists for the cases
//! where buffering is wrong:
//!
//! - Payloads too large to hold in memory (multi-gigabyte downloads, exports)
//! - Payloads of unknown total length (Server-Sent Events, log tails)
//! - Payloads produced incrementally, where time-to-first-byte matters
//!
//! Handlers rarely construct a `Body` directly. `ctx.json()`, `ctx.view()` and
//! friends produce buffered bodies; `ctx.file_download_stream()`,
//! `ctx.file_inline_stream()`, `ctx.sse()` and `ctx.stream()` produce
//! streaming ones. `Body` implements `From` for the common owned byte
//! containers, so `Response::with_body(vec)` and `response.body = vec.into()`
//! both work.
//!
//! # Middleware
//!
//! Outbound middleware that inspects or rewrites the body must handle both
//! variants. A stream has not been produced yet at the time middleware runs —
//! its bytes do not exist in memory and cannot be read without consuming them.
//! Use [`Body::as_slice`] / [`Body::as_mut_vec`], which return `None` for
//! streams, and skip the transformation in that case.

use crate::error::{Error, Result};
use bytes::{Bytes, BytesMut};
use futures::stream::{BoxStream, Stream, StreamExt};
use std::fmt;
use tokio::io::AsyncReadExt;

/// Bytes read per chunk when streaming a file.
///
/// This is the peak memory a streamed file download holds, regardless of how
/// large the file is. 64 KiB is comfortably above the typical 16 KiB TLS
/// record and socket buffer, so the syscall overhead stays amortised without
/// making per-connection memory meaningful at high concurrency.
pub const FILE_CHUNK_SIZE: usize = 64 * 1024;

/// A stream of body chunks, with an optional known total size.
///
/// The size hint, when present, becomes the `Content-Length` header. Set it
/// whenever the total length is known up front (a file on disk, a blob of
/// known size) — it lets the client render a real progress bar instead of an
/// indeterminate spinner. Leave it unset for genuinely open-ended streams;
/// the response is then sent with chunked transfer encoding.
pub struct BodyStream {
    inner: BoxStream<'static, Result<Bytes>>,
    size: Option<u64>,
}

impl BodyStream {
    /// Wrap a stream of chunks.
    ///
    /// An `Err` item aborts the response mid-flight: hyper drops the
    /// connection and the error is logged at `error` level. There is no way
    /// to turn it into a 500 — depending on how much hyper had already
    /// flushed, the client sees either a reset before any bytes or a
    /// truncated body under the status already sent. It never sees a
    /// terminating chunk, so a truncated download is not mistaken for a
    /// complete one. Validate before the first chunk, not during.
    pub fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<Bytes>> + Send + 'static,
    {
        Self {
            inner: stream.boxed(),
            size: None,
        }
    }

    /// Declare the total byte length, emitted as `Content-Length`.
    ///
    /// The value must match the total of all chunks. A mismatch is a protocol
    /// violation: too few bytes and the client hangs waiting for the rest, too
    /// many and the connection is poisoned for keep-alive reuse.
    pub fn with_size(mut self, bytes: u64) -> Self {
        self.size = Some(bytes);
        self
    }

    /// Total byte length, if known.
    pub fn size(&self) -> Option<u64> {
        self.size
    }

    /// Consume this wrapper, yielding the underlying chunk stream.
    ///
    /// The declared size, if any, is not enforced here; see
    /// [`Self::into_enforced_stream`].
    pub fn into_stream(self) -> BoxStream<'static, Result<Bytes>> {
        self.inner
    }

    /// Consume this wrapper, yielding a chunk stream that honours the
    /// declared size.
    ///
    /// Without a declared size this is [`Self::into_stream`]. With one, the
    /// stream is capped at exactly that many bytes: a chunk that would
    /// overshoot is truncated (a file that grew after its length was read,
    /// say) and the stream ends there, and a source that ends short yields an
    /// `Err` so the connection is dropped rather than left hanging with a
    /// `Content-Length` the client is still waiting on. Both cases are logged.
    pub fn into_enforced_stream(self) -> BoxStream<'static, Result<Bytes>> {
        match self.size {
            Some(size) => enforce_size(self.inner, size).boxed(),
            None => self.inner,
        }
    }
}

/// Cap `stream` at `declared` bytes and fail if it ends before reaching them.
fn enforce_size(
    stream: BoxStream<'static, Result<Bytes>>,
    declared: u64,
) -> impl Stream<Item = Result<Bytes>> + Send + 'static {
    // Nothing was promised, so nothing is sent: the source is never polled,
    // which also means a misbehaving source cannot yield a stray empty chunk.
    if declared == 0 {
        return futures::stream::empty().boxed();
    }

    struct State {
        inner: BoxStream<'static, Result<Bytes>>,
        remaining: u64,
        declared: u64,
        finished: bool,
    }

    futures::stream::unfold(
        State {
            inner: stream,
            remaining: declared,
            declared,
            finished: false,
        },
        |mut state| async move {
            if state.finished {
                return None;
            }

            match state.inner.next().await {
                Some(Ok(mut chunk)) => {
                    let len = chunk.len() as u64;
                    if len > state.remaining {
                        log::warn!(
                            "Response body stream produced more than its declared {} bytes; \
                             truncating to the declared length",
                            state.declared
                        );
                        chunk.truncate(state.remaining as usize);
                    }
                    state.remaining -= chunk.len() as u64;
                    if state.remaining == 0 {
                        // Exactly full: stop polling the source so an
                        // over-producing stream is never observed by hyper.
                        state.finished = true;
                    }
                    Some((Ok(chunk), state))
                }
                Some(Err(error)) => {
                    state.finished = true;
                    Some((Err(error), state))
                }
                None => {
                    state.finished = true;
                    if state.remaining > 0 {
                        let sent = state.declared - state.remaining;
                        Some((
                            Err(Error::internal(format!(
                                "response body stream ended after {} of {} declared bytes",
                                sent, state.declared
                            ))),
                            state,
                        ))
                    } else {
                        None
                    }
                }
            }
        },
    )
    .boxed()
}

impl fmt::Debug for BodyStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BodyStream")
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

/// The body of a [`Response`](crate::http::Response).
///
/// Defaults to an empty buffered body.
#[derive(Debug)]
pub enum Body {
    /// Fully buffered bytes.
    Full(Vec<u8>),
    /// Chunks produced incrementally.
    Stream(BodyStream),
}

impl Body {
    /// An empty buffered body.
    pub fn empty() -> Self {
        Body::Full(Vec::new())
    }

    /// Build a streaming body from a stream of chunks.
    pub fn from_stream<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<Bytes>> + Send + 'static,
    {
        Body::Stream(BodyStream::new(stream))
    }

    /// Build a streaming body of known total length.
    pub fn from_sized_stream<S>(stream: S, size: u64) -> Self
    where
        S: Stream<Item = Result<Bytes>> + Send + 'static,
    {
        Body::Stream(BodyStream::new(stream).with_size(size))
    }

    /// Stream an already-open file in [`FILE_CHUNK_SIZE`] chunks.
    ///
    /// Memory stays flat at one chunk no matter how large the file is. `size`
    /// is emitted as `Content-Length`, so read it from the file's metadata
    /// rather than guessing.
    ///
    /// The file is not re-checked for truncation or growth while streaming. If
    /// it shrinks mid-transfer the response ends short of the declared length
    /// and the client sees a truncated download; serve files that are not
    /// being rewritten underneath you.
    pub fn from_file(file: tokio::fs::File, size: u64) -> Self {
        Self::from_sized_stream(file_chunks(file, FILE_CHUNK_SIZE), size)
    }

    /// Whether this body is streamed rather than buffered.
    pub fn is_stream(&self) -> bool {
        matches!(self, Body::Stream(_))
    }

    /// Byte length, when known without consuming the body.
    ///
    /// Returns `Some` for buffered bodies and for streams with a declared
    /// size, `None` for open-ended streams. Middleware that needs to decide
    /// based on payload size — a minimum threshold before compressing, say —
    /// should treat `None` as "do not transform".
    pub fn len_hint(&self) -> Option<usize> {
        match self {
            Body::Full(bytes) => Some(bytes.len()),
            // `try_from` rather than `as`: on a 32-bit target a declared size
            // above `usize::MAX` must read as unknown, not as a wrapped number.
            Body::Stream(stream) => stream.size().and_then(|n| usize::try_from(n).ok()),
        }
    }

    /// Whether this body is known to carry no bytes.
    ///
    /// A stream is never reported as empty: it may yield nothing, but that is
    /// not knowable until it has been consumed.
    pub fn is_empty(&self) -> bool {
        match self {
            Body::Full(bytes) => bytes.is_empty(),
            Body::Stream(_) => false,
        }
    }

    /// Borrow the buffered bytes, or `None` if this body is streamed.
    pub fn as_slice(&self) -> Option<&[u8]> {
        match self {
            Body::Full(bytes) => Some(bytes),
            Body::Stream(_) => None,
        }
    }

    /// Mutably borrow the buffered bytes, or `None` if this body is streamed.
    pub fn as_mut_vec(&mut self) -> Option<&mut Vec<u8>> {
        match self {
            Body::Full(bytes) => Some(bytes),
            Body::Stream(_) => None,
        }
    }

    /// Take ownership of the buffered bytes, or `None` if this body is streamed.
    pub fn into_vec(self) -> Option<Vec<u8>> {
        match self {
            Body::Full(bytes) => Some(bytes),
            Body::Stream(_) => None,
        }
    }
}

/// Read a file into a stream of fixed-size chunks.
///
/// Each poll reads at most `chunk_size` bytes and yields them, so only one
/// chunk is resident at a time. A short read is yielded as-is rather than
/// looped until full — the client gets bytes sooner, and the next poll picks
/// up where this one stopped.
fn file_chunks(
    file: tokio::fs::File,
    chunk_size: usize,
) -> impl Stream<Item = Result<Bytes>> + Send + 'static {
    futures::stream::try_unfold(file, move |mut file| async move {
        let mut buf = BytesMut::zeroed(chunk_size);
        let read = file.read(&mut buf).await.map_err(Error::Io)?;
        if read == 0 {
            return Ok(None);
        }
        buf.truncate(read);
        Ok(Some((buf.freeze(), file)))
    })
}

impl Default for Body {
    fn default() -> Self {
        Body::empty()
    }
}

impl From<Vec<u8>> for Body {
    fn from(bytes: Vec<u8>) -> Self {
        Body::Full(bytes)
    }
}

impl From<&[u8]> for Body {
    fn from(bytes: &[u8]) -> Self {
        Body::Full(bytes.to_vec())
    }
}

impl From<Bytes> for Body {
    fn from(bytes: Bytes) -> Self {
        Body::Full(bytes.to_vec())
    }
}

impl From<String> for Body {
    fn from(text: String) -> Self {
        Body::Full(text.into_bytes())
    }
}

impl From<&str> for Body {
    fn from(text: &str) -> Self {
        Body::Full(text.as_bytes().to_vec())
    }
}

impl From<BodyStream> for Body {
    fn from(stream: BodyStream) -> Self {
        Body::Stream(stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;

    #[test]
    fn full_body_reports_length_and_emptiness() {
        let body = Body::from("hello");
        assert!(!body.is_stream());
        assert_eq!(body.len_hint(), Some(5));
        assert!(!body.is_empty());
        assert_eq!(body.as_slice(), Some(&b"hello"[..]));

        assert!(Body::empty().is_empty());
        assert_eq!(Body::empty().len_hint(), Some(0));
    }

    #[test]
    fn unsized_stream_hides_its_length() {
        let body = Body::from_stream(futures::stream::iter(vec![
            Ok(Bytes::from_static(b"a")),
            Ok(Bytes::from_static(b"b")),
        ]));

        assert!(body.is_stream());
        // Length is unknowable without consuming the stream, so middleware
        // gating on size must skip it rather than assume zero.
        assert_eq!(body.len_hint(), None);
        assert!(!body.is_empty());
        assert_eq!(body.as_slice(), None);
        assert_eq!(body.into_vec(), None);
    }

    #[test]
    fn sized_stream_exposes_declared_length() {
        let body = Body::from_sized_stream(
            futures::stream::iter(vec![Ok(Bytes::from_static(b"12345"))]),
            5,
        );

        assert!(body.is_stream());
        assert_eq!(body.len_hint(), Some(5));
    }

    #[test]
    fn as_mut_vec_edits_buffered_and_refuses_streamed() {
        let mut buffered = Body::from("hi");
        buffered
            .as_mut_vec()
            .expect("buffered body is editable")
            .extend_from_slice(b"!");
        assert_eq!(buffered.as_slice(), Some(&b"hi!"[..]));

        let mut streamed = Body::from_stream(futures::stream::empty());
        assert!(streamed.as_mut_vec().is_none());
    }

    #[tokio::test]
    async fn stream_yields_its_chunks_in_order() {
        let body = Body::from_stream(futures::stream::iter(vec![
            Ok(Bytes::from_static(b"one")),
            Ok(Bytes::from_static(b"two")),
        ]));

        let Body::Stream(stream) = body else {
            panic!("expected a streaming body");
        };
        let chunks: Vec<Bytes> = stream
            .into_stream()
            .map(|chunk| chunk.expect("chunk should be Ok"))
            .collect()
            .await;

        assert_eq!(
            chunks,
            vec![Bytes::from_static(b"one"), Bytes::from_static(b"two")]
        );
    }

    #[tokio::test]
    async fn enforced_stream_passes_an_exact_size_through_unchanged() {
        let body = Body::from_sized_stream(
            futures::stream::iter(vec![
                Ok(Bytes::from_static(b"abc")),
                Ok(Bytes::from_static(b"de")),
            ]),
            5,
        );
        let Body::Stream(stream) = body else {
            panic!("expected a streaming body");
        };
        let chunks: Vec<Result<Bytes>> = stream.into_enforced_stream().collect().await;
        let bytes: Vec<u8> = chunks
            .into_iter()
            .flat_map(|c| c.expect("no error for an exact size"))
            .collect();
        assert_eq!(bytes, b"abcde");
    }

    #[tokio::test]
    async fn enforced_stream_truncates_an_over_producing_source() {
        // Declared 4, source has 7: the client must receive exactly the 4
        // bytes promised by Content-Length and the stream must end there.
        let body = Body::from_sized_stream(
            futures::stream::iter(vec![
                Ok(Bytes::from_static(b"abc")),
                Ok(Bytes::from_static(b"defg")),
                Ok(Bytes::from_static(b"never polled")),
            ]),
            4,
        );
        let Body::Stream(stream) = body else {
            panic!("expected a streaming body");
        };
        let chunks: Vec<Result<Bytes>> = stream.into_enforced_stream().collect().await;
        assert_eq!(chunks.len(), 2);
        let bytes: Vec<u8> = chunks
            .into_iter()
            .flat_map(|c| c.expect("truncation is not an error"))
            .collect();
        assert_eq!(bytes, b"abcd");
    }

    #[tokio::test]
    async fn enforced_stream_with_zero_declared_size_never_polls_the_source() {
        let polled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = polled.clone();
        let source = futures::stream::poll_fn(move |_| {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
            std::task::Poll::Ready(Some(Ok(Bytes::from_static(b"unexpected"))))
        });
        let body = Body::from_sized_stream(source, 0);
        let Body::Stream(stream) = body else {
            panic!("expected a streaming body");
        };
        let chunks: Vec<Result<Bytes>> = stream.into_enforced_stream().collect().await;
        assert!(chunks.is_empty());
        assert!(!polled.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn enforced_stream_fails_when_the_source_ends_short() {
        // Declared 10, source has 3: leaving the connection open would hang
        // the client waiting for 7 more bytes, so the stream must error.
        let body = Body::from_sized_stream(
            futures::stream::iter(vec![Ok(Bytes::from_static(b"abc"))]),
            10,
        );
        let Body::Stream(stream) = body else {
            panic!("expected a streaming body");
        };
        let chunks: Vec<Result<Bytes>> = stream.into_enforced_stream().collect().await;
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].as_ref().unwrap(), &Bytes::from_static(b"abc"));
        let message = chunks[1].as_ref().unwrap_err().to_string();
        assert!(
            message.contains("3 of 10"),
            "error should say how short it fell: {message}"
        );
    }

    #[tokio::test]
    async fn stream_errors_surface_to_the_consumer() {
        let body = Body::from_stream(futures::stream::iter(vec![
            Ok(Bytes::from_static(b"partial")),
            Err(Error::internal("disk read failed")),
        ]));

        let Body::Stream(stream) = body else {
            panic!("expected a streaming body");
        };
        let results: Vec<Result<Bytes>> = stream.into_stream().collect().await;

        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_err());
    }
}
