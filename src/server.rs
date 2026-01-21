use axum::{
    Router,
    routing::get,
    response::IntoResponse,
    extract::{
        ws::{WebSocket, WebSocketUpgrade, Message},
        State,
    },
    http::StatusCode,
};
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use std::net::SocketAddr;
use std::sync::Arc;
use anyhow::Result;
use tokio::sync::watch;

use crate::config::Config;
use crate::rpc::MonerodClient;
use crate::template::TemplateState;

#[derive(Clone)]
pub struct AppState {
    pub template_rx: watch::Receiver<Option<TemplateState>>,
    pub rpc_client: Arc<MonerodClient>,
}

pub async fn run(
    config: Config,
    template_rx: watch::Receiver<Option<TemplateState>>,
    rpc_client: Arc<MonerodClient>,
) -> Result<()> {
    let state = AppState {
        template_rx,
        rpc_client,
    };

    let ws_path = config.server.ws_path.clone();
    
    let app = Router::new()
        .route("/health", get(health_check))
        .route(&ws_path, get(ws_handler))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(state);

    let addr: SocketAddr = config.server.bind_addr.parse()?;
    info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    info!("WebSocket connection established");
    
    let mut template_rx = state.template_rx.clone();
    
    loop {
        tokio::select! {
            // Watch for template updates
            result = template_rx.changed() => {
                if result.is_err() {
                    break;
                }
                let template_opt = template_rx.borrow().clone();
                if let Some(template) = template_opt {
                    let msg = format!(
                        "{{\"type\":\"template_update\",\"height\":{},\"template_id\":{}}}",
                        template.height, template.template_id
                    );
                    if socket.send(Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
            }
            // Handle incoming messages
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        info!("Received: {}", text);
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
    
    info!("WebSocket connection closed");
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.expect("Failed to install signal handler");
    info!("Shutdown signal received");
}
