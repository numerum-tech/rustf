//! WebSocket support.
//!
//! Two layers, both reached from a `SOCKET` route:
//!
//! * [`Context::socket`](crate::Context::socket) — the Total.js model. The
//!   framework keeps one [`SocketRoom`] per matched URL, and the handler
//!   configures it with `on_open` / `on_message` / `on_close` closures.
//! * [`Context::websocket`](crate::Context::websocket) — the raw escape
//!   hatch. One [`WebSocket`] per connection, driven by your own loop.
//!
//! ```ignore
//! pub fn install() -> Vec<Route> {
//!     routes![
//!         SOCKET "/echo" => echo,
//!     ]
//! }
//!
//! async fn echo(ctx: &mut Context) -> rustf::Result<()> {
//!     ctx.websocket(|mut socket| async move {
//!         while let Some(message) = socket.recv().await {
//!             socket.send(message?).await?;
//!         }
//!         Ok(())
//!     })
//! }
//! ```
//!
//! WebSocket runs over HTTP/1.1 only. RFC 8441 (WebSocket over HTTP/2) is not
//! supported by hyper's server, and browsers open WebSocket over HTTP/1.1
//! regardless.

pub(crate) mod handshake;
pub mod message;
pub mod room;
pub mod socket;
pub(crate) mod upgrade;

pub use message::{CloseCode, Message};
pub use room::{ClientId, SocketClient, SocketRoom, SOCKET};
pub use socket::{WebSocket, WebSocketReceiver, WebSocketSender};
