//! The session cookie's `Secure` flag follows the application's configured
//! environment — not the build profile, and not unrelated variables such as
//! `NODE_ENV`.
//!
//! One server for the whole binary: the framework's configuration is a
//! process-global singleton.

use rustf::config::{AppConfig, Environment};
use rustf::prelude::*;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

async fn remember(ctx: &mut Context) -> rustf::Result<()> {
    ctx.session_set("visited", true)?;
    ctx.plain("ok")
}

#[tokio::test]
async fn a_production_app_issues_secure_session_cookies() {
    // A container may carry another tool's convention; it must not matter.
    std::env::set_var("NODE_ENV", "development");
    std::env::remove_var("RUSTF_ENV");

    let config = AppConfig {
        environment: Environment::Production,
        ..AppConfig::default()
    };
    let app = RustF::with_config(config).controllers(routes![GET "/remember" => remember]);
    let running = app.serve_with_handle("127.0.0.1:0").await.unwrap();

    let mut stream = TcpStream::connect(running.local_addr).await.unwrap();
    stream
        .write_all(b"GET /remember HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut raw = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut raw))
        .await
        .expect("response in time")
        .unwrap();
    let head = String::from_utf8_lossy(&raw).to_lowercase();

    let cookie = head
        .lines()
        .find(|line| line.starts_with("set-cookie:") && line.contains("rustf_session"))
        .unwrap_or_else(|| panic!("no session cookie in:\n{head}"));
    assert!(cookie.contains("; secure"), "{cookie}");
    assert!(cookie.contains("; httponly"), "{cookie}");

    std::env::remove_var("NODE_ENV");
    running.handle.shutdown().await.unwrap();
}
