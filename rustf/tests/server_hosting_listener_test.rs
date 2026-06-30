use rustf::prelude::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

fn install_text_route() -> Vec<Route> {
    async fn index(ctx: &mut Context) -> rustf::Result<()> {
        ctx.text("hosted-ok")
    }

    routes![
        GET "/" => index,
    ]
}

async fn http_get(addr: std::net::SocketAddr, path: &str) -> String {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, addr
    );

    stream.write_all(request.as_bytes()).await.unwrap();

    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    String::from_utf8(response).unwrap()
}

#[tokio::test]
async fn serve_on_listener_uses_caller_listener_and_runs_shutdown_hooks() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let expected_addr = listener.local_addr().unwrap();
    let shutdown_seen = Arc::new(AtomicBool::new(false));

    let app = RustF::new()
        .controllers(install_text_route())
        .on("shutdown", {
            let shutdown_seen = Arc::clone(&shutdown_seen);
            move |_| {
                let shutdown_seen = Arc::clone(&shutdown_seen);
                Box::pin(async move {
                    shutdown_seen.store(true, Ordering::SeqCst);
                    Ok(())
                })
            }
        });

    let running = app.serve_on_listener(listener).await.unwrap();

    assert_eq!(running.local_addr, expected_addr);
    let response = http_get(expected_addr, "/").await;
    assert!(response.starts_with("HTTP/1.1 200"));

    running.handle.shutdown().await.unwrap();
    assert!(shutdown_seen.load(Ordering::SeqCst));
}
