use lifetrace_cloud::{app, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();
    config.validate().map_err(|message| {
        eprintln!("[lifetrace-cloud] invalid configuration: {message}");
        message
    })?;
    let state = lifetrace_cloud::AppState::new(config.clone());

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    let address = listener.local_addr().unwrap_or(config.bind_addr);
    let storage = if config.database_url.is_some() {
        "postgresql"
    } else {
        "in-memory"
    };
    println!(
        "[lifetrace-cloud] env={} storage={storage} listening on http://{address}",
        config.environment
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
    println!("[lifetrace-cloud] shutting down");
}
