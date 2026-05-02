pub mod api;
pub mod auth;
pub mod state;

use anyhow::Result;
use axum::{Router, service};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing_subscriber;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    tracing_subscriber::fmt()
        .with_env_filter("claudia_server=debug,tower_http=debug")
        .init();

    info!("Starting Claudia Server...");

    let state = state::AppState::new().await?;
    let shared_state = std::sync::Arc::new(state);

    // Build CORS layer
    let cors = CorsLayer::permissive();

    // Build router
    let app = Router::new()
        .nest("/api", api::routes(shared_state.clone()))
        .route("/", service::ServeDir::new("static"))
        .layer(ServiceBuilder::new().layer(cors))
        .with_state(shared_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("Claudia Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
