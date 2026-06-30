use rustf::prelude::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

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
async fn serve_with_handle_binds_ephemeral_port_and_serves_requests() {
    let app = RustF::new().controllers(install_text_route());
    let running = app.serve_with_handle("127.0.0.1:0").await.unwrap();
    let addr = running.local_addr;

    assert!(addr.ip().is_loopback());
    assert_ne!(addr.port(), 0);
    assert_eq!(running.handle.local_addr(), addr);

    let response = http_get(addr, "/").await;
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.contains("\r\n\r\nhosted-ok"));

    running.handle.shutdown().await.unwrap();
    assert!(TcpStream::connect(addr).await.is_err());
}
