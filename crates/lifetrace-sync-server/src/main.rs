use lifetrace_sync_server::{app, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();
    let state = lifetrace_sync_server::AppState::new(config.clone());

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    let address = listener.local_addr().unwrap_or(config.bind_addr);
    println!(
        "[lifetrace-sync-server] listening on http://{address} (in-memory storage prototype)"
    );

    axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    println!("[lifetrace-sync-server] shutting down");
}
