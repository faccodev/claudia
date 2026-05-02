pub mod api;
pub mod auth;
pub mod state;

use anyhow::Result;
use axum::Router;
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    log::info!("Starting Claudia Server...");

    let state = state::AppState::new().await?;
    let shared_state = std::sync::Arc::new(state);

    let cors = CorsLayer::permissive();

    let app = Router::new()
        .nest("/api", api::routes(shared_state.clone()))
        .route_service("/", ServeDir::new("static"))
        .layer(ServiceBuilder::new().layer(cors))
        .with_state(());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    log::info!("Claudia Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}