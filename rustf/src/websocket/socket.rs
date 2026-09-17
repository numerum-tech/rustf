//! The raw per-connection WebSocket wrapper.
//!
//! [`WebSocket`] is the building block the room layer is written on, and the
//! escape hatch for handlers that want to drive one connection themselves via
//! [`Context::websocket`](crate::Context::websocket).

use crate::error::{Error, Result};
use crate::websocket::message::{CloseCode, Message};
use futures::{SinkExt, StreamExt};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval_at, Instant, Interval};
use tokio_tungstenite::tungstenite::error::ProtocolError;
use tokio_tungstenite::tungstenite::protocol::frame::CloseFrame;
use tokio_tungstenite::tungstenite::Error as TungError;
use tokio_tungstenite::tungstenite::protocol::{Role, WebSocketConfig};
use tokio_tungstenite::tungstenite::Message as TungMessage;
use tokio_tungstenite::WebSocketStream;

/// The upgraded connection, as tungstenite sees it.
pub(crate) type UpgradedStream = WebSocketStream<TokioIo<Upgraded>>;

/// Counts frames received on a connection, so the writing half can tell
/// whether the peer is still answering.
///
/// A TCP write to a peer that has vanished can succeed for a long time — the
/// kernel buffers it and retransmits — so "the ping was sent" says nothing
/// about whether anyone is there. Only an inbound frame does, and after a
/// split, inbound frames are the other half's business. This counter is the
/// smallest thing that can cross that gap.
#[derive(Clone, Default)]
pub(crate) struct PeerActivity(Arc<AtomicU64>);

impl PeerActivity {
    fn record(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    /// A value that changes whenever something arrives from the peer.
    pub(crate) fn mark(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// Set while the reader is deliberately not reading — because it is awaiting a
/// message handler.
///
/// The keepalive concludes a peer is gone when nothing arrives between two
/// pings. That inference is only valid while someone is actually reading: a
/// handler that outlasts a ping interval would otherwise get its own healthy
/// client disconnected.
#[derive(Clone, Default)]
pub(crate) struct ReaderBusy(Arc<std::sync::atomic::AtomicBool>);

impl ReaderBusy {
    pub(crate) fn is_busy(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    /// Mark the reader busy until the returned guard is dropped.
    pub(crate) fn enter(&self) -> ReaderBusyGuard<'_> {
        self.0.store(true, Ordering::Release);
        ReaderBusyGuard(self)
    }
}

/// Clears the busy flag however the handler ends, panic included.
pub(crate) struct ReaderBusyGuard<'a>(&'a ReaderBusy);

impl Drop for ReaderBusyGuard<'_> {
    fn drop(&mut self) {
        self.0 .0.store(false, Ordering::Release);
    }
}

/// Translate a tungstenite error into a RustF one.
///
/// The variant is flattened to a message on purpose: leaking
/// `tungstenite::Error` through `rustf::Error` would put a private dependency
/// in the public API and tie RustF's semver to tungstenite's.
pub(crate) fn map_ws_error(error: tokio_tungstenite::tungstenite::Error) -> Error {
    Error::WebSocket(error.to_string())
}

/// How long to wait for the peer's half of a close handshake we initiated.
///
/// RFC 6455 §7.1.1 lets an endpoint stop waiting and drop the TCP connection;
/// this bounds that wait so a peer that never replies cannot hold a task open.
pub(crate) const CLOSE_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// Build a close frame, keeping it inside what RFC 6455 allows on the wire.
///
/// Two limits, both of which turn a well-meant close into a protocol error if
/// ignored: a control frame's payload may not exceed 125 bytes, which leaves
/// 123 for the reason after the two-byte code; and 1005/1006/1015 are
/// *local* status codes that must never be sent. A caller that asks for one
/// gets `1000`, because dropping the connection instead would be worse than
/// closing it with a slightly less specific code.
pub(crate) fn close_frame(code: CloseCode, reason: &str) -> CloseFrame {
    use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode as TungCode;

    const MAX_REASON_BYTES: usize = 123;

    // `is_allowed` is tungstenite's implementation of the RFC 6455 §7.4 table:
    // it rejects the local-only codes (1005/1006/1015), the unassigned ones
    // (1004, 1016..2999 outside a registered range) and anything out of range.
    // Reproducing that table here would only let the two drift apart.
    let code = if TungCode::from(code.as_u16()).is_allowed() {
        code
    } else {
        log::warn!("WebSocket close code {code} may not be sent on the wire; using 1000 instead");
        CloseCode::NORMAL
    };

    // Truncate on a character boundary: a reason is a courtesy, and half a
    // sentence beats an invalid frame.
    let reason = if reason.len() > MAX_REASON_BYTES {
        let mut end = MAX_REASON_BYTES;
        while end > 0 && !reason.is_char_boundary(end) {
            end -= 1;
        }
        &reason[..end]
    } else {
        reason
    };

    CloseFrame {
        code: code.into(),
        reason: reason.to_string().into(),
    }
}

/// What a read produced. Richer than `Option<Result<Message>>` because the
/// room layer has to report the peer's actual close code to `on_close`.
#[derive(Debug)]
pub(crate) enum Received {
    /// A data message.
    Message(Message),
    /// The peer sent a close frame, with the code it chose (RFC 6455 allows
    /// a close with no status, which is `None`).
    Closed(Option<CloseCode>),
    /// The stream ended without a close frame — the peer vanished.
    Eof,
    /// The connection failed, with the close code RFC 6455 gives this class of
    /// failure — `1009` for an oversize message, `1007` for bad UTF-8, `1002`
    /// for a protocol violation, `1006` for a transport reset.
    ///
    /// Without the code, every read failure would be reported to `on_close` as
    /// a generic protocol error, and a route's `[max]` limit would look like
    /// something else entirely.
    Failed(Error, CloseCode),
}

/// The close code RFC 6455 §7.4.1 assigns to this class of failure.
pub(crate) fn close_code_for(error: &TungError) -> CloseCode {
    match error {
        TungError::Capacity(_) => CloseCode::MESSAGE_TOO_BIG,
        TungError::Utf8(_) => CloseCode::INVALID_PAYLOAD,
        TungError::Protocol(ProtocolError::ResetWithoutClosingHandshake) => {
            CloseCode::from_u16(1006)
        }
        TungError::Protocol(_) => CloseCode::PROTOCOL_ERROR,
        // A transport failure: nothing was negotiated, the connection just
        // ended. That is exactly what 1006 means.
        TungError::Io(_) | TungError::ConnectionClosed | TungError::AlreadyClosed => {
            CloseCode::from_u16(1006)
        }
        _ => CloseCode::INTERNAL_ERROR,
    }
}

/// Tell the peer why the connection is ending, using the close code RFC 6455
/// §7.4.1 defines for this class of failure.
///
/// Best effort: a connection that failed on read is often already gone, and a
/// failure to send the close frame changes nothing the caller can act on.
async fn close_for_error(stream: &mut UpgradedStream, error: &TungError) {
    // The connection is already gone; there is nobody to tell.
    if matches!(
        error,
        TungError::ConnectionClosed | TungError::AlreadyClosed | TungError::Io(_)
    ) {
        return;
    }

    let code = close_code_for(error);
    send_close(stream, code, reason_for(code)).await;
}

/// Close a `[json]` route's connection with `1003`, completing the handshake.
///
/// The drain matters: without it the socket is dropped the moment `recv`
/// returns `None`, and the peer sees a TCP reset instead of the status code
/// telling it what it did wrong.
async fn reject_non_json(stream: &mut UpgradedStream) {
    if send_close(
        stream,
        CloseCode::UNSUPPORTED_DATA,
        "this endpoint accepts JSON only",
    )
    .await
    {
        await_close_reply(stream).await;
    }
}

/// Write a close frame and flush it, giving up if the peer is not reading.
///
/// The write itself has to be bounded, not just the wait for the reply: a peer
/// that has filled the send window blocks `send`/`flush` indefinitely, and the
/// close timeout below would never be reached. `false` means the frame could
/// not be delivered, and the caller should stop using the stream.
pub(crate) async fn send_close(
    stream: &mut UpgradedStream,
    code: CloseCode,
    reason: &str,
) -> bool {
    let write = async {
        let _ = stream
            .send(TungMessage::Close(Some(close_frame(code, reason))))
            .await;
        let _ = SinkExt::flush(stream).await;
    };

    if tokio::time::timeout(CLOSE_HANDSHAKE_TIMEOUT, write).await.is_err() {
        log::debug!("peer is not reading; abandoning the close frame");
        return false;
    }
    true
}

/// Read until the peer answers our close frame, or the timeout expires.
pub(crate) async fn await_close_reply(stream: &mut UpgradedStream) {
    let wait = async {
        while let Some(frame) = stream.next().await {
            match frame {
                Ok(TungMessage::Close(_)) | Err(_) => break,
                Ok(_) => continue,
            }
        }
    };

    if tokio::time::timeout(CLOSE_HANDSHAKE_TIMEOUT, wait).await.is_err() {
        log::debug!("peer did not answer the close handshake within the timeout");
    }
}

/// A short, non-leaky reason string for a close frame.
fn reason_for(code: CloseCode) -> &'static str {
    match code {
        CloseCode::MESSAGE_TOO_BIG => "message exceeds the configured limit",
        CloseCode::INVALID_PAYLOAD => "text frame was not valid UTF-8",
        CloseCode::PROTOCOL_ERROR => "WebSocket protocol violation",
        _ => "internal error",
    }
}

/// A live WebSocket connection.
///
/// Control frames are handled by the runtime: pings are answered with pongs
/// automatically and never surface, and a close frame ends [`recv`] rather
/// than appearing as a message. Handlers therefore only ever see the text and
/// binary messages the peer actually sent.
///
/// [`recv`]: WebSocket::recv
pub struct WebSocket {
    stream: UpgradedStream,
    /// Server-side keepalive ticker, from `[websocket] ping_interval`.
    ///
    /// Driven from [`WebSocket::recv`] rather than a separate task: `recv` is
    /// where a handler spends its time, and a ping written from there needs no
    /// second handle on the stream. A handler that only ever sends, or that
    /// has [`split`](WebSocket::split) the socket, gets no automatic pings.
    ping: Option<Interval>,
    /// The route's `[json]` flag: reject text that is not valid JSON.
    json_only: bool,
    /// Bumped on every inbound frame; read by the writing half after a split.
    activity: PeerActivity,
    /// The activity mark taken when the last keepalive ping went out, while we
    /// are still waiting to see anything back.
    awaiting_pong: Option<u64>,
}

impl WebSocket {
    /// Wrap an upgraded hyper connection.
    ///
    /// `ping_interval` of `None`, or of zero, disables the keepalive.
    pub(crate) async fn from_upgraded(
        upgraded: Upgraded,
        config: WebSocketConfig,
        ping_interval: Option<Duration>,
        json_only: bool,
    ) -> Self {
        Self {
            json_only,
            activity: PeerActivity::default(),
            awaiting_pong: None,
            stream: WebSocketStream::from_raw_socket(
                TokioIo::new(upgraded),
                Role::Server,
                Some(config),
            )
            .await,
            ping: ping_interval.filter(|d| !d.is_zero()).map(|period| {
                // Start one period out, not immediately: `Interval`'s first
                // tick resolves at once, which would ping every fresh socket
                // before it had said anything.
                interval_at(Instant::now() + period, period)
            }),
        }
    }

    /// Receive the next message.
    ///
    /// Returns `None` once the peer has closed the connection — cleanly or
    /// otherwise. Ping, pong and close frames are consumed internally, and
    /// keepalive pings are sent from here while waiting.
    pub async fn recv(&mut self) -> Option<Result<Message>> {
        let Self {
            stream,
            ping,
            json_only,
            activity,
            awaiting_pong,
        } = self;
        let json_only = *json_only;

        loop {
            let message = match ping {
                Some(interval) => {
                    tokio::select! {
                        // Bias towards the socket: when a message and a ping
                        // are both ready, the peer's data goes first.
                        biased;
                        message = stream.next() => message,
                        _ = interval.tick() => {
                            // Nothing at all since the previous ping means the
                            // peer is accepting TCP but no longer speaking
                            // WebSocket — the half-open connection the
                            // keepalive exists to find.
                            let mark = activity.mark();
                            if *awaiting_pong == Some(mark) {
                                log::debug!(
                                    "WebSocket peer stopped answering keepalive pings; closing"
                                );
                                send_close(
                                    stream,
                                    CloseCode::GOING_AWAY,
                                    "no response to keepalive",
                                )
                                .await;
                                return None;
                            }

                            // A failed keepalive ping means the connection is
                            // already gone; the next read reports how.
                            if stream.send(TungMessage::Ping(Vec::new().into())).await.is_err() {
                                return None;
                            }
                            *awaiting_pong = Some(mark);
                            continue;
                        }
                    }
                }
                None => stream.next().await,
            };

            let message = message?;
            activity.record();

            match message {
                Ok(TungMessage::Text(text)) => {
                    let message = Message::Text(text.as_str().to_string());
                    // `[json]` on the route: the framework rejects non-JSON so
                    // the handler never has to. RFC 6455 §7.4.1 gives 1003 for
                    // "data of a type I cannot accept", which is exactly this.
                    // Binary frames are checked too — a route that promises
                    // JSON must not let one through just because it arrived
                    // with a different opcode.
                    if json_only && !is_json(&message) {
                        reject_non_json(stream).await;
                        return None;
                    }
                    return Some(Ok(message));
                }
                Ok(TungMessage::Binary(bytes)) => {
                    let message = Message::Binary(bytes);
                    if json_only && !is_json(&message) {
                        reject_non_json(stream).await;
                        return None;
                    }
                    return Some(Ok(message));
                }
                // Control frames: tungstenite has already queued the pong and
                // flushed it during the read, so there is nothing to do but
                // keep waiting for real data.
                Ok(TungMessage::Ping(_)) | Ok(TungMessage::Pong(_)) => continue,
                Ok(TungMessage::Close(_)) => {
                    // The peer wants to close. tungstenite has already queued
                    // the reply frame, but only writes it on the next read —
                    // which never comes, because the handler's loop ends here
                    // and drops the socket. Flushing sends it, so the peer
                    // sees a completed close handshake instead of a reset.
                    //
                    // It must be `flush`, not `close`: `WebSocketStream::close`
                    // sends a *new* Close message, and tungstenite refuses to
                    // send anything once the connection is no longer active.
                    let _ = SinkExt::flush(stream).await;
                    return None;
                }
                // `Frame` is documented as never produced while reading.
                Ok(TungMessage::Frame(_)) => continue,
                Err(error) => {
                    // tungstenite enforces the size limits but does not tell
                    // the peer why it stopped. Send the close frame RFC 6455
                    // §7.4.1 calls for, so an oversize message reads as 1009
                    // on the client rather than a bare disconnect, then hand
                    // the error on so the handler can log it.
                    close_for_error(stream, &error).await;
                    return Some(Err(map_ws_error(error)));
                }
            }
        }
    }

    /// Send a message.
    pub async fn send(&mut self, message: Message) -> Result<()> {
        self.stream
            .send(message.into())
            .await
            .map_err(map_ws_error)
    }

    /// Send a text message.
    pub async fn send_text<S: Into<String>>(&mut self, text: S) -> Result<()> {
        self.send(Message::text(text)).await
    }

    /// Send a binary message.
    pub async fn send_binary<B: Into<bytes::Bytes>>(&mut self, bytes: B) -> Result<()> {
        self.send(Message::binary(bytes)).await
    }

    /// Serialise `value` as JSON and send it as text.
    pub async fn send_json<T: serde::Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        self.send(Message::json(value)?).await
    }

    /// Close the connection, completing the RFC 6455 closing handshake.
    ///
    /// Sends the close frame, then reads until the peer's own close arrives or
    /// [`CLOSE_HANDSHAKE_TIMEOUT`] passes — a peer that never replies must not
    /// hold the task open, but one that replies promptly should see a clean
    /// close rather than a TCP reset.
    ///
    /// Consumes the socket: after a close there is nothing left to do with it.
    pub async fn close(mut self, code: CloseCode, reason: &str) -> Result<()> {
        // Bounded: a peer that has stopped reading would otherwise block the
        // write, and the wait for its reply below would never be reached.
        let send = async {
            let result = self
                .stream
                .send(TungMessage::Close(Some(close_frame(code, reason))))
                .await;

            match result {
                // `send` queued the close frame; `flush` puts it on the wire.
                Ok(()) => match SinkExt::flush(&mut self.stream).await {
                    Ok(())
                    | Err(TungError::ConnectionClosed)
                    | Err(TungError::AlreadyClosed) => Some(Ok(())),
                    Err(error) => Some(Err(map_ws_error(error))),
                },
                // The peer disappeared before we could say goodbye. That is how
                // connections often end; it is not an error worth propagating.
                Err(TungError::ConnectionClosed) | Err(TungError::AlreadyClosed) => None,
                // The peer had already started the close handshake, so
                // tungstenite refuses a second close frame. Flushing still
                // delivers the reply it queued for us, which completes it.
                Err(TungError::Protocol(ProtocolError::SendAfterClosing)) => {
                    let _ = SinkExt::flush(&mut self.stream).await;
                    None
                }
                Err(error) => Some(Err(map_ws_error(error))),
            }
        };

        let outcome = match tokio::time::timeout(CLOSE_HANDSHAKE_TIMEOUT, send).await {
            Ok(Some(result)) => result,
            // Nothing more to say, or nothing more that can be said.
            Ok(None) => return Ok(()),
            Err(_) => {
                log::debug!("peer is not reading; abandoning the close frame");
                return Ok(());
            }
        };

        // Our close frame is out. Give the peer its turn before the socket is
        // dropped and the TCP connection with it.
        await_close_reply(&mut self.stream).await;

        outcome
    }

    /// Split into independent sending and receiving halves.
    ///
    /// Needed whenever one task pushes messages while another reads them —
    /// a notification feed, say, that must keep draining the socket so pings
    /// are answered.
    ///
    /// The halves are plumbing, not policy: keepalive pings and the route's
    /// `[json]` check both live on the whole socket and do not survive a
    /// split. A split socket answers the peer's pings (tungstenite does that
    /// on read) but sends none of its own, and its close handshake must be
    /// completed by calling [`WebSocketSender::close`] once `recv` ends.
    pub fn split(self) -> (WebSocketSender, WebSocketReceiver) {
        let (sink, stream) = self.stream.split();
        (
            WebSocketSender {
                sink,
                activity: self.activity.clone(),
                awaiting_pong: None,
            },
            WebSocketReceiver {
                stream,
                activity: self.activity,
            },
        )
    }

}

/// The sending half of a split [`WebSocket`].
pub struct WebSocketSender {
    sink: futures::stream::SplitSink<UpgradedStream, TungMessage>,
    activity: PeerActivity,
    /// The activity mark taken when the last keepalive ping went out, if we
    /// are still waiting to see anything back.
    awaiting_pong: Option<u64>,
}

impl WebSocketSender {
    /// Send a message.
    pub async fn send(&mut self, message: Message) -> Result<()> {
        self.sink.send(message.into()).await.map_err(map_ws_error)
    }

    /// Send a text message.
    pub async fn send_text<S: Into<String>>(&mut self, text: S) -> Result<()> {
        self.send(Message::text(text)).await
    }

    /// Send a binary message.
    pub async fn send_binary<B: Into<bytes::Bytes>>(&mut self, bytes: B) -> Result<()> {
        self.send(Message::binary(bytes)).await
    }

    /// Serialise `value` as JSON and send it as text.
    pub async fn send_json<T: serde::Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        self.send(Message::json(value)?).await
    }

    /// Send a keepalive ping, and report whether the peer answered the last one.
    ///
    /// Returns `Ok(false)` when nothing at all has arrived since the previous
    /// ping: the peer is accepting bytes at the TCP level but is no longer
    /// speaking WebSocket, which is exactly the half-open connection the
    /// keepalive exists to find. The caller closes it.
    pub(crate) async fn ping_and_check(&mut self, reader_busy: &ReaderBusy) -> Result<bool> {
        let mark = self.activity.mark();

        // Only trust "nothing arrived" when someone was actually reading. A
        // message handler that outlasts a ping interval would otherwise get its
        // own healthy client disconnected.
        if reader_busy.is_busy() {
            self.awaiting_pong = None;
        } else if let Some(previous) = self.awaiting_pong {
            if previous == mark {
                return Ok(false);
            }
        }

        self.sink
            .send(TungMessage::Ping(bytes::Bytes::new()))
            .await
            .map_err(map_ws_error)?;
        self.awaiting_pong = Some(mark);
        Ok(true)
    }

    /// Send a close frame and flush it, completing the close handshake.
    pub async fn close(&mut self, code: CloseCode, reason: &str) -> Result<()> {
        let result = self
            .sink
            .send(TungMessage::Close(Some(close_frame(code, reason))))
            .await;

        match result {
            Ok(()) => match SinkExt::flush(&mut self.sink).await {
                Ok(()) | Err(TungError::ConnectionClosed) | Err(TungError::AlreadyClosed) => Ok(()),
                Err(error) => Err(map_ws_error(error)),
            },
            Err(TungError::ConnectionClosed) | Err(TungError::AlreadyClosed) => Ok(()),
            // The peer closed first: flush delivers the reply tungstenite
            // queued, completing the handshake.
            Err(TungError::Protocol(ProtocolError::SendAfterClosing)) => {
                let _ = SinkExt::flush(&mut self.sink).await;
                Ok(())
            }
            Err(error) => Err(map_ws_error(error)),
        }
    }
}

/// The receiving half of a split [`WebSocket`].
pub struct WebSocketReceiver {
    stream: futures::stream::SplitStream<UpgradedStream>,
    activity: PeerActivity,
}

impl WebSocketReceiver {
    /// Receive the next message, skipping control frames.
    ///
    /// `None` once the peer has closed the connection.
    pub async fn recv(&mut self) -> Option<Result<Message>> {
        match self.receive().await {
            Received::Message(message) => Some(Ok(message)),
            Received::Failed(error, _) => Some(Err(error)),
            Received::Closed(_) | Received::Eof => None,
        }
    }

    /// The same read, but reporting *how* the connection ended.
    ///
    /// The room layer needs this: `on_close` is supposed to hand the handler
    /// the code the peer actually sent, which `recv`'s `None` cannot carry.
    ///
    /// Unlike `WebSocket::recv`, this half cannot write, so the reply frame
    /// tungstenite queues on close is flushed by whoever holds the matching
    /// [`WebSocketSender`].
    pub(crate) async fn receive(&mut self) -> Received {
        loop {
            let Some(next) = self.stream.next().await else {
                return Received::Eof;
            };

            // Anything at all from the peer — data, pong, close — proves it is
            // still there, which is what the keepalive check reads.
            self.activity.record();

            match next {
                Ok(TungMessage::Text(text)) => {
                    return Received::Message(Message::Text(text.as_str().to_string()))
                }
                Ok(TungMessage::Binary(bytes)) => {
                    return Received::Message(Message::Binary(bytes))
                }
                Ok(TungMessage::Ping(_)) | Ok(TungMessage::Pong(_)) => continue,
                Ok(TungMessage::Close(frame)) => {
                    return Received::Closed(frame.map(|frame| frame.code.into()))
                }
                Ok(TungMessage::Frame(_)) => continue,
                Err(TungError::ConnectionClosed) | Err(TungError::AlreadyClosed) => {
                    return Received::Eof
                }
                Err(error) => {
                    let code = close_code_for(&error);
                    return Received::Failed(map_ws_error(error), code);
                }
            }
        }
    }
}

/// Is this payload valid JSON? Used by the route-level `[json]` flag.
pub(crate) fn is_json(message: &Message) -> bool {
    serde_json::from_slice::<serde_json::Value>(message.as_bytes()).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reason_longer_than_the_control_frame_allows_is_truncated() {
        // A control frame carries at most 125 bytes; two go to the code.
        let frame = close_frame(CloseCode::NORMAL, &"x".repeat(500));
        assert_eq!(frame.reason.as_str().len(), 123);
    }

    #[test]
    fn truncation_lands_on_a_character_boundary() {
        // 62 two-byte characters is 124 bytes, one over the limit, so the cut
        // falls mid-character unless it is walked back.
        let reason = "é".repeat(62);
        let frame = close_frame(CloseCode::NORMAL, &reason);
        assert_eq!(frame.reason.as_str().len(), 122);
        assert!(frame.reason.as_str().chars().all(|c| c == 'é'));
    }

    #[test]
    fn a_short_reason_is_left_alone() {
        let frame = close_frame(CloseCode::GOING_AWAY, "server shutting down");
        assert_eq!(frame.reason.as_str(), "server shutting down");
        assert_eq!(u16::from(frame.code), 1001);
    }

    #[test]
    fn codes_that_may_not_be_sent_become_1000() {
        // 1005, 1006 and 1015 are local status codes: RFC 6455 §7.4.1 forbids
        // them on the wire, and a peer is entitled to fail the connection.
        for reserved in [1005u16, 1006, 1015] {
            let frame = close_frame(CloseCode::from_u16(reserved), "");
            assert_eq!(
                u16::from(frame.code),
                1000,
                "{reserved} must not be sent as-is"
            );
        }
    }

    #[test]
    fn application_close_codes_pass_through() {
        let frame = close_frame(CloseCode::from_u16(4001), "app specific");
        assert_eq!(u16::from(frame.code), 4001);
    }

    #[test]
    fn json_detection_covers_both_frame_types() {
        assert!(is_json(&Message::text(r#"{"a":1}"#)));
        assert!(is_json(&Message::binary(br#"[1,2]"#.to_vec())));
        assert!(!is_json(&Message::text("not json")));
        assert!(!is_json(&Message::binary(vec![0xff, 0xfe])));
    }

    #[test]
    fn peer_activity_changes_only_when_something_arrives() {
        let activity = PeerActivity::default();
        let before = activity.mark();
        assert_eq!(activity.mark(), before, "reading must not count as activity");

        activity.record();
        assert_ne!(activity.mark(), before);
    }
}
