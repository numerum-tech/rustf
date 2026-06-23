use crate::app::RustF;
use crate::error::Result;
use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as AutoBuilder;
use hyper_util::server::graceful::GracefulShutdown;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

pub struct Server {
    app: Arc<RustF>,
}

impl Server {
    pub fn new(app: RustF) -> Self {
        Self { app: Arc::new(app) }
    }

    pub async fn serve(self, addr: &str) -> Result<()> {
        let addr: SocketAddr = addr
            .parse()
            .map_err(|e| crate::error::Error::internal(format!("Invalid address: {}", e)))?;

        log::info!("RustF server listening on {}", addr);

        // Setup signal handling for graceful shutdown
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        // Spawn signal handler task
        tokio::spawn(async move {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};

                let mut sigterm = match signal(SignalKind::terminate()) {
                    Ok(sig) => sig,
                    Err(e) => {
                        log::error!("Failed to install SIGTERM handler: {}", e);
                        return;
                    }
                };

                let mut sigint = match signal(SignalKind::interrupt()) {
                    Ok(sig) => sig,
                    Err(e) => {
                        log::error!("Failed to install SIGINT handler: {}", e);
                        return;
                    }
                };

                tokio::select! {
                    _ = sigterm.recv() => {
                        log::info!("Received SIGTERM signal - initiating graceful shutdown");
                    }
                    _ = sigint.recv() => {
                        log::info!("Received SIGINT signal (Ctrl+C) - initiating graceful shutdown");
                    }
                }
            }

            #[cfg(not(unix))]
            {
                match tokio::signal::ctrl_c().await {
                    Ok(()) => {
                        log::info!("Received Ctrl+C signal - initiating graceful shutdown");
                    }
                    Err(e) => {
                        log::error!("Failed to listen for Ctrl+C signal: {}", e);
                        return;
                    }
                }
            }

            // Send shutdown signal
            let _ = shutdown_tx.send(());
        });

        // Keep reference to app for cleanup
        let app_ref = Arc::clone(&self.app);

        // Bind the listener (hyper 1.x: no high-level Server, we drive the
        // accept loop ourselves through hyper-util).
        let listener = TcpListener::bind(addr).await.map_err(|e| {
            crate::error::Error::internal(format!("Failed to bind {}: {}", addr, e))
        })?;

        // `auto::Builder` negotiates HTTP/1 and HTTP/2 (replaces the 0.14 "full"
        // auto-detection); `GracefulShutdown` tracks live connections.
        let builder = AutoBuilder::new(TokioExecutor::new());
        let graceful = GracefulShutdown::new();
        let mut shutdown_rx = std::pin::pin!(shutdown_rx);

        loop {
            tokio::select! {
                accept = listener.accept() => {
                    let (stream, peer_addr) = match accept {
                        Ok(conn) => conn,
                        Err(e) => {
                            // Transient accept errors (EMFILE, ECONNABORTED, ...)
                            // must not bring the whole server down.
                            log::error!("Accept error: {}", e);
                            continue;
                        }
                    };

                    let io = TokioIo::new(stream);
                    let app = Arc::clone(&self.app);

                    let service = service_fn(move |req| {
                        let app = Arc::clone(&app);
                        let peer_addr = peer_addr;
                        async move {
                            match app.handle_request_with_peer(req, Some(peer_addr)).await {
                                Ok(response) => Ok::<_, Infallible>(response.into_hyper()),
                                Err(e) => {
                                    log::error!("Request handling error: {}", e);
                                    Ok(crate::http::Response::internal_error().into_hyper())
                                }
                            }
                        }
                    });

                    // `.into_owned()` detaches the connection from the borrowed
                    // builder so it can be spawned and watched for graceful drain.
                    let conn = builder
                        .serve_connection_with_upgrades(io, service)
                        .into_owned();
                    let watched = graceful.watch(conn);

                    tokio::spawn(async move {
                        if let Err(e) = watched.await {
                            // Per-connection errors (client disconnects, broken
                            // pipes) are benign — log at debug, never propagate.
                            log::debug!("Connection error: {}", e);
                        }
                    });
                }

                _ = shutdown_rx.as_mut() => {
                    log::info!("Shutdown signal received - no longer accepting connections");
                    break;
                }
            }
        }

        // Stop accepting, then drain in-flight connections.
        log::info!("Waiting for in-flight connections to complete...");
        graceful.shutdown().await;

        // Server has stopped, trigger cleanup
        log::info!("Server stopped, initiating cleanup...");
        app_ref.cleanup().await?;

        Ok(())
    }
}
