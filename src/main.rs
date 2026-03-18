use axum::Router;
use clap::Parser;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use cors_proxy_rs::config::Config;
use cors_proxy_rs::cors::CorsLayer;
use cors_proxy_rs::proxy::AppState;
use cors_proxy_rs::rate_limit::RateLimitLayer;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("cors_proxy_rs=info".parse().unwrap()),
        )
        .init();

    let config = Config::parse();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .expect("Failed to create HTTP client");

    let state = AppState {
        client,
        config: config.clone(),
    };

    let app = Router::new()
        .fallback(cors_proxy_rs::proxy::proxy_handler)
        .layer(CorsLayer::new(config.allowed_origins.clone()))
        .layer(RateLimitLayer::new(config.rate_limit_per_minute))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(addr).await.expect("Failed to bind");

    tracing::info!("cors-proxy-rs listening on http://{addr}");

    if !config.allowed_origins.is_empty() {
        tracing::info!("Allowed origins: {:?}", config.allowed_origins);
    } else {
        tracing::info!("All origins allowed");
    }

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("Server error");
}
