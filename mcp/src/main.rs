mod server;

#[cfg(all(feature = "cli", feature = "http"))]
compile_error!("Les features 'cli' et 'http' sont mutuellement exclusives.");

#[cfg(all(not(test), not(any(feature = "cli", feature = "http"))))]
compile_error!("Choisissez une feature : --features cli  ou  --features http");

#[cfg(feature = "cli")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use rmcp::transport::stdio;
    use rmcp::ServiceExt;
    use server::BoldifyServer;

    let service = BoldifyServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

#[cfg(feature = "http")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use rmcp::transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    };
    use server::BoldifyServer;

    let service = StreamableHttpService::new(
        || Ok(BoldifyServer::new()),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default(),
    );

    let router = axum::Router::new().nest_service("/mcp", service);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.unwrap();
        })
        .await?;
    Ok(())
}

#[cfg(not(any(feature = "cli", feature = "http")))]
fn main() {}
