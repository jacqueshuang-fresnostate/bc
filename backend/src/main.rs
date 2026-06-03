mod app;
mod domain;
mod error;
mod response;
mod routes;
mod services;

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "bc_backend=debug,tower_http=debug".into()),
        )
        .init();

    let port = std::env::var("PORT").unwrap_or_else(|_| "18080".to_string());
    let address = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&address).await?;

    tracing::info!(address = %address, "后台接口服务已开始监听");
    axum::serve(listener, app::router_from_env().await?).await?;

    Ok(())
}
