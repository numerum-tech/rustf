//! Server-Sent Events (SSE).
//!
//! SSE is a one-way push channel from server to browser over a plain HTTP
//! response whose body never ends. The browser opens it with `EventSource`,
//! reconnects on its own when the connection drops, and resumes from the last
//! event id it saw. That makes it the right tool for live dashboards,
//! progress bars, notifications, and log tails — anything where the server
//! talks and the client listens. For two-way traffic use WebSocket.
//!
//! The wire format is text: each event is a block of `field: value` lines
//! terminated by a blank line. [`SseEvent`] builds one block; a
//! [`Stream`] of them becomes a response through
//! [`Response::sse`](crate::http::Response::sse) or `ctx.sse(...)`.
//!
//! ```rust,no_run
//! use rustf::prelude::*;
//! use rustf::http::SseEvent;
//! use tokio_stream::wrappers::ReceiverStream;
//!
//! async fn progress(ctx: &mut Context) -> rustf::Result<()> {
//!     let (tx, rx) = tokio::sync::mpsc::channel(16);
//!
//!     tokio::spawn(async move {
//!         for pct in (0..=100).step_by(10) {
//!             let event = SseEvent::new(pct.to_string()).event("progress");
//!             if tx.send(event).await.is_err() {
//!                 break; // client went away
//!             }
//!             tokio::time::sleep(std::time::Duration::from_millis(200)).await;
//!         }
//!     });
//!
//!     ctx.sse(ReceiverStream::new(rx))
//! }
//! ```
//!
//! # Fallible sources
//!
//! The stream may yield either `SseEvent` or `rustf::Result<SseEvent>` (see
//! [`SseItem`]). An `Err` aborts the response the same way a failing
//! [`Body`](crate::http::Body) stream does: it is logged at `error` level and
//! the connection is dropped, so `EventSource` reconnects. Use it for
//! transport failures (a database notification channel going away), not for
//! application errors — those belong in an event the client can read.
//!
//! # Idle connections
//!
//! Proxies and load balancers close connections that carry no bytes for a
//! while (commonly 30–60 s). A feed that is quiet for longer than that gets
//! cut and the browser reconnects in a loop. Wrap the stream with
//! [`keep_alive`] to emit a comment line once per interval while nothing
//! else is being sent; comments are invisible to `EventSource`.

use crate::error::Result;
use bytes::Bytes;
use futures::stream::Stream;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};
use std::time::Duration;

/// One Server-Sent Event.
///
/// Only `data` is required. `id` lets a reconnecting client tell the server
/// where it left off (`Last-Event-ID` request header); `event` routes the
/// message to a named `addEventListener` on the client instead of `onmessage`;
/// `retry` overrides the client's reconnect delay.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SseEvent {
    id: Option<String>,
    event: Option<String>,
    data: Option<String>,
    retry: Option<Duration>,
    comment: Option<String>,
}

impl SseEvent {
    /// An event carrying `data`.
    ///
    /// Multi-line data is sent as one `data:` line per line and reassembled by
    /// the client with `\n` separators, so line breaks survive the trip.
    pub fn new(data: impl Into<String>) -> Self {
        Self {
            data: Some(data.into()),
            ..Self::default()
        }
    }

    /// An event whose data is `value` serialised as JSON.
    ///
    /// JSON never contains a raw newline, so the payload always fits one
    /// `data:` line and `JSON.parse(event.data)` on the client round-trips it.
    pub fn json<T: serde::Serialize>(value: &T) -> Result<Self> {
        Ok(Self::new(serde_json::to_string(value)?))
    }

    /// A comment line, ignored by clients.
    ///
    /// Its only purpose is to put bytes on the wire so an idle connection is
    /// not closed by an intermediary. [`keep_alive`] emits these for you.
    pub fn comment(text: impl Into<String>) -> Self {
        Self {
            comment: Some(text.into()),
            ..Self::default()
        }
    }

    /// Set the event id, echoed back by the client as `Last-Event-ID` on
    /// reconnect.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the event name, dispatched to `addEventListener(name, ..)` on the
    /// client.
    pub fn event(mut self, name: impl Into<String>) -> Self {
        self.event = Some(name.into());
        self
    }

    /// Tell the client how long to wait before reconnecting after a drop.
    ///
    /// Sub-millisecond precision is lost; the protocol carries whole
    /// milliseconds.
    pub fn retry(mut self, delay: Duration) -> Self {
        self.retry = Some(delay);
        self
    }

    /// Encode this event in the SSE wire format.
    ///
    /// Every field value is split on the three line endings the event-stream
    /// parser recognises — `\r\n`, bare `\n`, and bare `\r` — and each
    /// fragment is written as its own line under the same field name. A value
    /// can therefore never smuggle a second field in: user input in `id`,
    /// `event` or `data` cannot forge a `retry:` or `data:` line, whatever
    /// line ending it uses.
    pub fn to_bytes(&self) -> Bytes {
        let mut out = String::new();

        if let Some(comment) = &self.comment {
            let lines = field_lines(comment);
            if lines.is_empty() {
                out.push_str(":\n");
            }
            for line in lines {
                out.push(':');
                out.push_str(line);
                out.push('\n');
            }
        }

        if let Some(id) = &self.id {
            // Per spec an id containing U+0000 is ignored by the client; there
            // is no valid way to encode it, so strip it rather than emit a
            // field the client will throw away.
            for line in field_lines(id) {
                out.push_str("id: ");
                push_without_nul(&mut out, line);
                out.push('\n');
            }
        }

        if let Some(event) = &self.event {
            for line in field_lines(event) {
                out.push_str("event: ");
                push_without_nul(&mut out, line);
                out.push('\n');
            }
        }

        if let Some(retry) = &self.retry {
            out.push_str("retry: ");
            out.push_str(&retry.as_millis().to_string());
            out.push('\n');
        }

        if let Some(data) = &self.data {
            let lines = field_lines(data);
            if lines.is_empty() {
                // "data: \n\n" dispatches an event with empty data; a bare
                // blank line would be a no-op the client silently drops.
                out.push_str("data: \n");
            }
            for line in lines {
                out.push_str("data: ");
                out.push_str(line);
                out.push('\n');
            }
        }

        // Blank line terminates the event and makes the client dispatch it.
        out.push('\n');
        Bytes::from(out)
    }
}

/// Split a field value on every line ending the SSE parser treats as one:
/// `\r\n`, `\n`, and a bare `\r`.
///
/// A trailing line ending does not produce an empty final line, so
/// `"x\n"` encodes as one `data: x` line rather than two. Splitting on ASCII
/// bytes cannot land inside a multi-byte UTF-8 sequence.
fn field_lines(value: &str) -> Vec<&str> {
    let bytes = value.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                lines.push(&value[start..i]);
                i += 1;
                start = i;
            }
            b'\r' => {
                lines.push(&value[start..i]);
                i += 1;
                if i < bytes.len() && bytes[i] == b'\n' {
                    i += 1;
                }
                start = i;
            }
            _ => i += 1,
        }
    }

    if start < bytes.len() {
        lines.push(&value[start..]);
    }
    lines
}

fn push_without_nul(out: &mut String, line: &str) {
    if line.contains('\0') {
        out.push_str(&line.replace('\0', ""));
    } else {
        out.push_str(line);
    }
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::SseEvent {}
    impl Sealed for crate::error::Result<super::SseEvent> {}
}

/// An item an SSE source stream may yield: a plain [`SseEvent`], or a
/// `rustf::Result<SseEvent>` for sources that can fail mid-feed.
///
/// Implemented for exactly those two types; the trait is sealed so the set
/// can grow without breaking callers.
pub trait SseItem: sealed::Sealed + Send + 'static {
    /// Convert into the event to send, or the error that aborts the stream.
    fn into_event(self) -> Result<SseEvent>;

    /// Wrap a framework-generated event (a keep-alive comment) as this item
    /// type.
    fn from_event(event: SseEvent) -> Self;
}

impl SseItem for SseEvent {
    fn into_event(self) -> Result<SseEvent> {
        Ok(self)
    }

    fn from_event(event: SseEvent) -> Self {
        event
    }
}

impl SseItem for Result<SseEvent> {
    fn into_event(self) -> Result<SseEvent> {
        self
    }

    fn from_event(event: SseEvent) -> Self {
        Ok(event)
    }
}

/// Emit a comment whenever `events` has been idle for `interval`.
///
/// The first comment goes out one full `interval` after the stream starts,
/// and the timer restarts on every real event — so a busy stream never sees
/// a comment and a quiet one sees exactly one per `interval`. The stream ends
/// when `events` ends.
///
/// Pick an interval below the shortest idle timeout on the path to the
/// client; 15 s clears the common 30 s proxy default with margin.
pub fn keep_alive<S>(events: S, interval: Duration) -> KeepAlive<S>
where
    S: Stream,
    S::Item: SseItem,
{
    // `interval_at` rather than `interval`: the latter's first tick fires
    // immediately, which would put a comment on the wire before the feed has
    // been idle for any time at all.
    let mut timer = tokio::time::interval_at(tokio::time::Instant::now() + interval, interval);
    // A missed tick (the task was busy) should not fire a burst of comments;
    // one is enough to prove the connection alive.
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    KeepAlive {
        events: Box::pin(events),
        timer,
        done: false,
    }
}

/// Stream returned by [`keep_alive`].
pub struct KeepAlive<S> {
    events: Pin<Box<S>>,
    timer: tokio::time::Interval,
    done: bool,
}

impl<S> Stream for KeepAlive<S>
where
    S: Stream,
    S::Item: SseItem,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<S::Item>> {
        if self.done {
            return Poll::Ready(None);
        }

        match self.events.as_mut().poll_next(cx) {
            Poll::Ready(Some(item)) => {
                // Real traffic proves liveness; push the next comment out by a
                // full interval from now.
                self.timer.reset();
                Poll::Ready(Some(item))
            }
            Poll::Ready(None) => {
                self.done = true;
                Poll::Ready(None)
            }
            Poll::Pending => match self.timer.poll_tick(cx) {
                Poll::Ready(_) => Poll::Ready(Some(S::Item::from_event(SseEvent::comment("")))),
                Poll::Pending => Poll::Pending,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    fn text(event: &SseEvent) -> String {
        String::from_utf8(event.to_bytes().to_vec()).unwrap()
    }

    #[test]
    fn data_only_event_is_one_data_line_and_a_blank_line() {
        assert_eq!(text(&SseEvent::new("hello")), "data: hello\n\n");
    }

    #[test]
    fn all_fields_are_written_in_spec_order() {
        let event = SseEvent::new("payload")
            .id("42")
            .event("update")
            .retry(Duration::from_millis(2500));

        assert_eq!(
            text(&event),
            "id: 42\nevent: update\nretry: 2500\ndata: payload\n\n"
        );
    }

    #[test]
    fn multi_line_data_becomes_one_data_line_per_line() {
        assert_eq!(
            text(&SseEvent::new("line one\nline two")),
            "data: line one\ndata: line two\n\n"
        );
        // A trailing newline in the data does not add an empty data line,
        // which would make the client see an extra "\n" on reassembly.
        assert_eq!(text(&SseEvent::new("trailing\n")), "data: trailing\n\n");
        // Every line ending the SSE parser recognises is a line break here
        // too, so the client never sees a stray '\r'.
        assert_eq!(text(&SseEvent::new("a\r\nb")), "data: a\ndata: b\n\n");
        assert_eq!(text(&SseEvent::new("a\rb")), "data: a\ndata: b\n\n");
        // An interior blank line survives as an empty data line.
        assert_eq!(
            text(&SseEvent::new("a\n\nb")),
            "data: a\ndata: \ndata: b\n\n"
        );
    }

    #[test]
    fn empty_data_still_dispatches_an_event() {
        // "data: \n\n" dispatches an event with empty data on the client;
        // "\n" alone would be a no-op that the client silently drops.
        assert_eq!(text(&SseEvent::new("")), "data: \n\n");
    }

    #[test]
    fn line_breaks_in_any_field_cannot_forge_other_fields() {
        // An attacker-controlled value must not be able to inject "retry: 1"
        // or a "data:" line of its own — with any line ending.
        let event = SseEvent::new("x")
            .id("7\nretry: 1")
            .event("a\ndata: forged");
        assert_eq!(
            text(&event),
            "id: 7\nid: retry: 1\nevent: a\nevent: data: forged\ndata: x\n\n"
        );

        // Bare CR is a line terminator for the event-stream parser, so it
        // must be split exactly like LF.
        let cr = SseEvent::new("ok\rretry: 0")
            .id("1\rretry: 0")
            .event("e\rretry: 0");
        assert_eq!(
            text(&cr),
            "id: 1\nid: retry: 0\nevent: e\nevent: retry: 0\ndata: ok\ndata: retry: 0\n\n"
        );

        let comment = SseEvent::comment("c\rdata: forged");
        assert_eq!(text(&comment), ":c\n:data: forged\n\n");
    }

    #[test]
    fn nul_is_stripped_from_id_and_event() {
        let event = SseEvent::new("x").id("a\0b").event("e\0v");
        assert_eq!(text(&event), "id: ab\nevent: ev\ndata: x\n\n");
    }

    #[test]
    fn json_event_serialises_the_value() {
        let event = SseEvent::json(&serde_json::json!({"n": 1, "s": "a\nb"})).unwrap();
        assert_eq!(text(&event), "data: {\"n\":1,\"s\":\"a\\nb\"}\n\n");
    }

    #[test]
    fn comment_is_a_colon_line() {
        assert_eq!(text(&SseEvent::comment("")), ":\n\n");
        assert_eq!(text(&SseEvent::comment("ping")), ":ping\n\n");
    }

    #[tokio::test(start_paused = true)]
    async fn keep_alive_emits_comments_only_after_a_full_idle_interval() {
        let (tx, rx) = tokio::sync::mpsc::channel::<SseEvent>(4);
        let mut stream = keep_alive(
            tokio_stream::wrappers::ReceiverStream::new(rx),
            Duration::from_secs(15),
        );

        // Nothing has happened yet: no comment before one interval elapses.
        assert!(
            tokio::time::timeout(Duration::from_secs(14), stream.next())
                .await
                .is_err(),
            "no keep-alive may be sent before the interval has elapsed"
        );

        // A real event arrives before the timer: it is passed through and
        // the timer restarts.
        tx.send(SseEvent::new("real")).await.unwrap();
        assert_eq!(stream.next().await, Some(SseEvent::new("real")));

        // Nothing for a full interval → one comment.
        let next = tokio::time::timeout(Duration::from_secs(16), stream.next())
            .await
            .expect("keep-alive must fire within one interval");
        assert_eq!(next, Some(SseEvent::comment("")));

        // Closing the source ends the stream instead of ticking forever.
        drop(tx);
        assert_eq!(stream.next().await, None);
    }

    #[tokio::test(start_paused = true)]
    async fn keep_alive_wraps_comments_in_the_source_item_type() {
        // A fallible source yields Result<SseEvent>; the injected comment
        // must come back as Ok(comment), and errors pass through untouched.
        let source = futures::stream::iter(vec![
            Ok(SseEvent::new("one")),
            Err(crate::error::Error::internal("channel closed")),
        ])
        .chain(futures::stream::pending());
        let mut stream = keep_alive(source, Duration::from_secs(1));

        assert!(matches!(stream.next().await, Some(Ok(e)) if e == SseEvent::new("one")));
        assert!(matches!(stream.next().await, Some(Err(_))));
        let next = tokio::time::timeout(Duration::from_secs(2), stream.next())
            .await
            .expect("keep-alive must fire once the source idles");
        assert!(matches!(next, Some(Ok(e)) if e == SseEvent::comment("")));
    }
}
