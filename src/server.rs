use axum::{
    Router,
    routing::get,
    response::IntoResponse,
    extract::{
        ws::{WebSocket, WebSocketUpgrade, Message},
        State, ConnectInfo,
    },
    http::StatusCode,
};
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use std::net::{SocketAddr, IpAddr};
use std::sync::Arc;
use anyhow::Result;
use tokio::sync::watch;

use crate::config::Config;
use crate::protocol::{ClientMessage, ServerMessage, ErrorCode};
use crate::rpc::MonerodClient;
use crate::session::SessionManager;
use crate::template::TemplateState;

#[derive(Clone)]
pub struct AppState {
    pub template_rx: watch::Receiver<Option<TemplateState>>,
    pub rpc_client: Arc<MonerodClient>,
    pub session_manager: Arc<SessionManager>,
    pub config: Config,
}

pub async fn run(
    config: Config,
    template_rx: watch::Receiver<Option<TemplateState>>,
    rpc_client: Arc<MonerodClient>,
    session_manager: Arc<SessionManager>,
) -> Result<()> {
    let state = AppState {
        template_rx,
        rpc_client,
        session_manager,
        config: config.clone(),
    };

    let ws_path = config.server.ws_path.clone();
    
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/stats", get(stats_handler))
        .route(&ws_path, get(ws_handler))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(state);

    let addr: SocketAddr = config.server.bind_addr.parse()?;
    info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

async fn stats_handler(State(state): State<AppState>) -> impl IntoResponse {
    let count = state.session_manager.active_count();
    (StatusCode::OK, format!("{{\"active_sessions\":{}}}", count))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let ip = addr.ip();
    ws.on_upgrade(move |socket| handle_socket(socket, state, ip))
}

async fn handle_socket(mut socket: WebSocket, state: AppState, ip: IpAddr) {
    let session = match state.session_manager.create_session(ip) {
        Some(s) => s,
        None => {
            warn!("Connection limit exceeded for IP: {}", ip);
            let msg = ServerMessage::error(None, ErrorCode::RateLimit, "Too many connections");
            let _ = socket.send(Message::Text(serde_json::to_string(&msg).unwrap())).await;
            return;
        }
    };

    let session_id = session.id.clone();
    info!("Session created: {} from {}", session_id, ip);

    let mut template_rx = state.template_rx.clone();

    loop {
        tokio::select! {
            result = template_rx.changed() => {
                if result.is_err() {
                    break;
                }
                // Template updated - would send job here
                state.session_manager.update_session(&session_id, |s| s.touch());
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(client_msg) => {
                                if let Some(response) = handle_message(&state, &session_id, client_msg).await {
                                    let json = serde_json::to_string(&response).unwrap();
                                    if socket.send(Message::Text(json)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Invalid message: {}", e);
                                let msg = ServerMessage::error(None, ErrorCode::BadFormat, "Invalid message format");
                                let _ = socket.send(Message::Text(serde_json::to_string(&msg).unwrap())).await;
                            }
                        }
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

    state.session_manager.remove_session(&session_id);
    info!("Session closed: {}", session_id);
}

async fn handle_message(
    state: &AppState,
    session_id: &str,
    msg: ClientMessage,
) -> Option<ServerMessage> {
    match msg {
        ClientMessage::Hello { client_version, threads, .. } => {
            state.session_manager.update_session(session_id, |s| {
                s.set_ready(client_version.clone(), threads);
            });
            Some(ServerMessage::Stats {
                id: None,
                session_id: session_id.to_string(),
                submits_per_minute: state.config.limits.submits_per_minute,
                messages_per_second: state.config.limits.messages_per_second,
            })
        }
        ClientMessage::Ping { id } => {
            state.session_manager.update_session(session_id, |s| s.touch());
            Some(ServerMessage::Pong { id })
        }
        ClientMessage::Submit { id, .. } => {
            // Placeholder - full validation in Part 4
            Some(ServerMessage::SubmitResult {
                id,
                status: crate::protocol::SubmitStatus::Accepted,
                message: Some("Placeholder".to_string()),
            })
        }
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.expect("Failed to install signal handler");
    info!("Shutdown signal received");
}
