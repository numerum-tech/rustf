//! WebSocket messages and close codes.
//!
//! [`Message`] is deliberately narrower than tungstenite's: it carries only
//! the two data frames an application ever writes. Ping, Pong and Close are
//! control frames the runtime answers on its own, so they never reach handler
//! code and cannot be sent by mistake.

use crate::error::{Error, Result};
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

/// A WebSocket data message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// A UTF-8 text message.
    Text(String),
    /// A binary message.
    Binary(Bytes),
}

impl Message {
    /// Build a text message.
    pub fn text<S: Into<String>>(text: S) -> Self {
        Message::Text(text.into())
    }

    /// Build a binary message.
    pub fn binary<B: Into<Bytes>>(bytes: B) -> Self {
        Message::Binary(bytes.into())
    }

    /// Serialise `value` as JSON and send it as a text message.
    pub fn json<T: Serialize + ?Sized>(value: &T) -> Result<Self> {
        serde_json::to_string(value)
            .map(Message::Text)
            .map_err(|e| Error::internal(format!("failed to serialise WebSocket message: {e}")))
    }

    /// Parse the payload as JSON.
    ///
    /// Works for both text and binary messages: a client that sends JSON as a
    /// binary frame is unusual but not wrong, and failing on it would be a
    /// surprise.
    pub fn parse<T: DeserializeOwned>(&self) -> Result<T> {
        serde_json::from_slice(self.as_bytes())
            .map_err(|e| Error::Validation(format!("failed to parse WebSocket message: {e}")))
    }

    /// The payload as text, or `None` for a binary message.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Message::Text(text) => Some(text),
            Message::Binary(_) => None,
        }
    }

    /// The raw payload bytes.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Message::Text(text) => text.as_bytes(),
            Message::Binary(bytes) => bytes,
        }
    }

    /// Payload length in bytes.
    pub fn len(&self) -> usize {
        self.as_bytes().len()
    }

    /// Is the payload empty?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Is this a text message?
    pub fn is_text(&self) -> bool {
        matches!(self, Message::Text(_))
    }

    /// Is this a binary message?
    pub fn is_binary(&self) -> bool {
        matches!(self, Message::Binary(_))
    }
}

impl From<String> for Message {
    fn from(text: String) -> Self {
        Message::Text(text)
    }
}

impl From<&str> for Message {
    fn from(text: &str) -> Self {
        Message::Text(text.to_string())
    }
}

impl From<Vec<u8>> for Message {
    fn from(bytes: Vec<u8>) -> Self {
        Message::Binary(bytes.into())
    }
}

impl From<Message> for tokio_tungstenite::tungstenite::Message {
    fn from(message: Message) -> Self {
        match message {
            Message::Text(text) => tokio_tungstenite::tungstenite::Message::Text(text.into()),
            Message::Binary(bytes) => tokio_tungstenite::tungstenite::Message::Binary(bytes),
        }
    }
}

/// A WebSocket close code (RFC 6455 §7.4).
///
/// A newtype rather than an enum so that codes outside the registered set —
/// application codes in 4000–4999, or anything a future revision adds — round
/// -trip unchanged instead of collapsing into a catch-all variant. It also
/// keeps tungstenite out of RustF's public API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CloseCode(u16);

impl CloseCode {
    /// 1000 — the purpose the connection was opened for has been fulfilled.
    pub const NORMAL: Self = Self(1000);
    /// 1001 — the endpoint is going away (server shutting down, page closing).
    pub const GOING_AWAY: Self = Self(1001);
    /// 1002 — protocol error.
    pub const PROTOCOL_ERROR: Self = Self(1002);
    /// 1003 — a message of a type the endpoint cannot accept.
    pub const UNSUPPORTED_DATA: Self = Self(1003);
    /// 1007 — payload inconsistent with the message type (e.g. bad UTF-8).
    pub const INVALID_PAYLOAD: Self = Self(1007);
    /// 1008 — the message violated a policy of the endpoint.
    pub const POLICY_VIOLATION: Self = Self(1008);
    /// 1009 — the message is too big to process.
    pub const MESSAGE_TOO_BIG: Self = Self(1009);
    /// 1011 — the server hit an unexpected condition.
    pub const INTERNAL_ERROR: Self = Self(1011);

    /// Wrap a raw close code.
    pub const fn from_u16(code: u16) -> Self {
        Self(code)
    }

    /// The raw close code.
    pub const fn as_u16(self) -> u16 {
        self.0
    }
}

impl From<u16> for CloseCode {
    fn from(code: u16) -> Self {
        Self(code)
    }
}

impl From<CloseCode> for u16 {
    fn from(code: CloseCode) -> Self {
        code.0
    }
}

impl std::fmt::Display for CloseCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<CloseCode> for tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode {
    fn from(code: CloseCode) -> Self {
        code.0.into()
    }
}

impl From<tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode> for CloseCode {
    fn from(code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode) -> Self {
        Self(code.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Payload {
        kind: String,
        count: u32,
    }

    #[test]
    fn text_and_binary_expose_the_same_bytes() {
        assert_eq!(Message::text("hi").as_bytes(), b"hi");
        assert_eq!(Message::binary(vec![1, 2, 3]).as_bytes(), &[1, 2, 3]);
        assert_eq!(Message::text("hi").as_text(), Some("hi"));
        assert_eq!(Message::binary(vec![1]).as_text(), None);
    }

    #[test]
    fn len_and_is_empty_count_bytes_not_chars() {
        // Four bytes, one character.
        let message = Message::text("🦀");
        assert_eq!(message.len(), 4);
        assert!(!message.is_empty());
        assert!(Message::text("").is_empty());
    }

    #[test]
    fn json_round_trips() {
        let payload = Payload {
            kind: "chat".into(),
            count: 3,
        };
        let message = Message::json(&payload).unwrap();
        assert!(message.is_text());
        assert_eq!(message.parse::<Payload>().unwrap(), payload);
    }

    #[test]
    fn json_parses_out_of_a_binary_frame_too() {
        let message = Message::binary(br#"{"kind":"chat","count":1}"#.to_vec());
        assert_eq!(message.parse::<Payload>().unwrap().count, 1);
    }

    #[test]
    fn parse_failure_is_a_validation_error_not_an_internal_one() {
        let err = Message::text("not json").parse::<Payload>().unwrap_err();
        assert!(
            matches!(err, Error::Validation(_)),
            "a malformed client message is the client's fault: {err:?}"
        );
    }

    #[test]
    fn close_codes_round_trip_through_u16() {
        assert_eq!(CloseCode::NORMAL.as_u16(), 1000);
        assert_eq!(CloseCode::GOING_AWAY.as_u16(), 1001);
        assert_eq!(CloseCode::MESSAGE_TOO_BIG.as_u16(), 1009);
        // An application-defined code survives the trip unchanged.
        assert_eq!(CloseCode::from_u16(4001).as_u16(), 4001);
        assert_eq!(u16::from(CloseCode::from(4042u16)), 4042);
    }

    #[test]
    fn converts_into_tungstenite_messages() {
        use tokio_tungstenite::tungstenite::Message as Tung;

        assert_eq!(Tung::from(Message::text("hi")), Tung::Text("hi".into()));
        match Tung::from(Message::binary(vec![7u8])) {
            Tung::Binary(bytes) => assert_eq!(&bytes[..], &[7u8]),
            other => panic!("expected a binary frame, got {other:?}"),
        }
    }
}
