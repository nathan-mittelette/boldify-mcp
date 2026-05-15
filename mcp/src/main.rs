mod server;

#[cfg(all(feature = "cli", feature = "http"))]
compile_error!("Les features 'cli' et 'http' sont mutuellement exclusives.");

#[cfg(all(not(test), not(any(feature = "cli", feature = "http"))))]
compile_error!("Choisissez une feature : --features cli  ou  --features http");

#[cfg(feature = "cli")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use rmcp::transport::stdio;
    use rmcp::ServiceExt;
    use server::BoldifyServer;

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let service = BoldifyServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

#[cfg(feature = "http")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use rmcp::transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    };
    use server::BoldifyServer;

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let service = StreamableHttpService::new(
        || Ok(BoldifyServer::new()),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default()
            .disable_allowed_hosts()
            .with_stateful_mode(false)
            .with_json_response(true),
    );

    let router = axum::Router::new()
        .route(
            "/health",
            axum::routing::get(|| async {
                tracing::info!("health check");
                axum::Json(serde_json::json!({ "status": "up" }))
            }),
        )
        .nest_service("/mcp", service);
    let port = std::env::var("AWS_LWA_PORT").unwrap_or_else(|_| "3000".to_string());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!(port, "HTTP server listening");
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.unwrap();
        })
        .await?;
    Ok(())
}

#[cfg(not(any(feature = "cli", feature = "http")))]
fn main() {}
