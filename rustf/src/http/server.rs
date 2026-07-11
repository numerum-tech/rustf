use crate::app::RustF;
use crate::error::{Error, ErrorChain, Result};
use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as AutoBuilder;
use hyper_util::server::graceful::GracefulShutdown;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

pub struct Server {
    app: Arc<RustF>,
}

pub struct ServerHandle {
    local_addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: JoinHandle<Result<()>>,
}

pub struct RunningServer {
    pub local_addr: SocketAddr,
    pub handle: ServerHandle,
}

impl ServerHandle {
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub async fn shutdown(mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        self.task
            .await
            .map_err(|e| Error::internal(format!("Server task failed: {}", e)))?
    }
}

impl Server {
    pub fn new(app: RustF) -> Self {
        Self { app: Arc::new(app) }
    }

    pub async fn serve(self, addr: &str) -> Result<()> {
        let listener = Self::bind_listener(addr).await?;
        self.run(listener, Self::spawn_signal_handler(), None).await
    }

    pub async fn serve_with_handle(self, addr: &str) -> Result<RunningServer> {
        let listener = Self::bind_listener(addr).await?;
        self.serve_on_listener(listener).await
    }

    pub async fn serve_on_listener(self, listener: TcpListener) -> Result<RunningServer> {
        let local_addr = listener.local_addr().map_err(|e| {
            Error::internal(format!("Failed to read listener local address: {}", e))
        })?;
        log::info!("RustF server listening on {}", local_addr);

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let (started_tx, started_rx) = oneshot::channel::<()>();

        let task =
            tokio::spawn(async move { self.run(listener, shutdown_rx, Some(started_tx)).await });

        started_rx
            .await
            .map_err(|_| Error::internal("Hosted server failed to start".to_string()))?;

        Ok(RunningServer {
            local_addr,
            handle: ServerHandle {
                local_addr,
                shutdown_tx: Some(shutdown_tx),
                task,
            },
        })
    }

    async fn bind_listener(addr: &str) -> Result<TcpListener> {
        let addr: SocketAddr = addr
            .parse()
            .map_err(|e| Error::internal(format!("Invalid address: {}", e)))?;
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| Error::internal(format!("Failed to bind {}: {}", addr, e)))?;
        let local_addr = listener.local_addr().map_err(|e| {
            Error::internal(format!("Failed to read listener local address: {}", e))
        })?;
        log::info!("RustF server listening on {}", local_addr);
        Ok(listener)
    }

    fn spawn_signal_handler() -> oneshot::Receiver<()> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

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

            let _ = shutdown_tx.send(());
        });

        shutdown_rx
    }

    async fn run(
        self,
        listener: TcpListener,
        shutdown_rx: oneshot::Receiver<()>,
        started_tx: Option<oneshot::Sender<()>>,
    ) -> Result<()> {
        let app_ref = Arc::clone(&self.app);

        let builder = AutoBuilder::new(TokioExecutor::new());
        let graceful = GracefulShutdown::new();
        let mut shutdown_rx = std::pin::pin!(shutdown_rx);

        if let Some(started_tx) = started_tx {
            let _ = started_tx.send(());
        }

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
                                    log::error!(
                                        "Request handling error: {}",
                                        ErrorChain::new(&e).format_for_log()
                                    );
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
