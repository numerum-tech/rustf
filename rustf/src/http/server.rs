use crate::app::RustF;
use crate::error::Result;
use hyper::service::{make_service_fn, service_fn};
use hyper::Server as HyperServer;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

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

        let make_svc = make_service_fn(move |_conn| {
            let app = Arc::clone(&self.app);
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let app = Arc::clone(&app);
                    async move {
                        match app.handle_request(req).await {
                            Ok(response) => Ok::<hyper::Response<hyper::Body>, hyper::Error>(
                                response.into_hyper(),
                            ),
                            Err(e) => {
                                log::error!("Request handling error: {}", e);
                                Ok(crate::http::Response::internal_error().into_hyper())
                            }
                        }
                    }
                }))
            }
        });

        let server = HyperServer::bind(&addr)
            .serve(make_svc)
            .with_graceful_shutdown(async {
                shutdown_rx.await.ok();
            });

        // Wait for server to finish
        if let Err(e) = server.await {
            log::error!("Server error: {}", e);
        }

        // Server has stopped, trigger cleanup
        log::info!("Server stopped, initiating cleanup...");
        app_ref.cleanup().await?;

        Ok(())
    }
}
