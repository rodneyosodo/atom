//! Process shutdown signaling shared by the HTTP and gRPC servers.

/// Resolves when the process receives SIGINT (Ctrl-C) or, on Unix, SIGTERM.
///
/// Both the HTTP and gRPC servers await this so in-flight requests drain on
/// shutdown instead of being dropped. Each server installs its own listener;
/// tokio delivers a single OS signal to every registered handler, so one
/// SIGTERM wakes both. On non-Unix platforms only Ctrl-C is awaited.
pub async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::error!("failed to install Ctrl-C handler: {err}");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(err) => tracing::error!("failed to install SIGTERM handler: {err}"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }

    tracing::info!("shutdown signal received");
}
