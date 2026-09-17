//! Chat — WebSocket controller.
//!
//! `SOCKET` registers a `GET` route that only accepts a WebSocket handshake:
//! a plain `GET` to it gets `426 Upgrade Required`, and the handler never
//! runs. The page itself is an ordinary `GET`.
//!
//! RustF keeps one `SocketRoom` per matched URL. The closure passed to
//! `ctx.socket` configures that room and runs for the first client to connect
//! to this path; everyone after joins the room it set up. Once per room, not
//! once per process — an empty room is dropped when its last client leaves, so
//! the next connection configures a new one.

use rustf::prelude::*;

pub fn install() -> Vec<Route> {
    routes![
        GET    "/chat"        => page,
        // `[json]` so the route matches the generated browser client, which
        // parses every message as JSON. Drop the flag to accept arbitrary
        // text and binary frames — and update the client to match.
        SOCKET "/chat/socket" => chat, [json],
    ]
}

/// The page that opens the socket.
async fn page(ctx: &mut Context) -> rustf::Result<()> {
    ctx.repository_set("title", "Chat");
    ctx.view("chat/index", json!({}))
}

/// The socket itself.
async fn chat(ctx: &mut Context) -> rustf::Result<()> {
    // Authorise here if the socket carries anything private — this runs on the
    // handshake, before any upgrade, so a rejection is an ordinary HTTP
    // response the browser reports as a failed connection:
    //
    //     if ctx.session().map(|s| s.is_authenticated()) != Some(true) {
    //         return ctx.throw401(Some("Sign in first"));
    //     }

    ctx.socket(|room| {
        room.on_open(|client| async move {
            client.send_json(&json!({
                "type": "welcome",
                "id": client.id(),
                "online": client.room().online(),
            }))
        });

        room.on_message(|client, message| async move {
            // Relay to everyone else in the room — Total.js `self.send(msg, sender)`.
            // The `[json]` flag means this is always valid JSON by the time it
            // gets here — the framework closes anything else with 1003.
            client.room().send_except(message, client.id())
        });

        room.on_close(|client, _code| async move {
            client.room().send_json(&json!({
                "type": "left",
                "id": client.id(),
            }))
        });
    })
}
