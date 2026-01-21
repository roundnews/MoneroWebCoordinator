use axum::{
    Router,
    routing::get,
    response::IntoResponse,
    extract::ws::{WebSocket, WebSocketUpgrade},
    http::StatusCode,
};
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use std::net::SocketAddr;
use anyhow::Result;

use crate::config::Config;

pub async fn run(config: Config) -> Result<()> {
    let ws_path = config.server.ws_path.clone();
    
    let app = Router::new()
        .route("/health", get(health_check))
        .route(&ws_path, get(ws_handler))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        );

    let addr: SocketAddr = config.server.bind_addr.parse()?;
    info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");
    Ok(())
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    info!("New WebSocket connection established");
    
    // Placeholder: just echo messages back for now
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(msg) => {
                if socket.send(msg).await.is_err() {
                    warn!("Failed to send message, client disconnected");
                    break;
                }
            }
            Err(e) => {
                warn!("WebSocket error: {}", e);
                break;
            }
        }
    }
    
    info!("WebSocket connection closed");
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
    info!("Shutdown signal received");
}
