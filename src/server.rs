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
use crate::jobs::JobManager;
use crate::metrics::Metrics;
use crate::protocol::{ClientMessage, ServerMessage, ErrorCode, SubmitStatus};
use crate::rpc::MonerodClient;
use crate::session::{SessionManager, SessionState};
use crate::template::TemplateState;
use crate::validator::SubmissionValidator;

#[derive(Clone)]
pub struct AppState {
    pub template_rx: watch::Receiver<Option<TemplateState>>,
    pub rpc_client: Arc<MonerodClient>,
    pub session_manager: Arc<SessionManager>,
    pub job_manager: Arc<JobManager>,
    pub validator: Arc<SubmissionValidator>,
    pub metrics: Arc<Metrics>,
    pub config: Config,
}

pub async fn run(
    config: Config,
    template_rx: watch::Receiver<Option<TemplateState>>,
    rpc_client: Arc<MonerodClient>,
    session_manager: Arc<SessionManager>,
    job_manager: Arc<JobManager>,
    validator: Arc<SubmissionValidator>,
    metrics: Arc<Metrics>,
) -> Result<()> {
    let state = AppState {
        template_rx, rpc_client, session_manager, job_manager, validator, metrics,
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
            let msg = ServerMessage::error(None, ErrorCode::RateLimit, "Too many connections from this IP address");
            let _ = socket.send(Message::Text(serde_json::to_string(&msg).unwrap())).await;
            return;
        }
    };

    let session_id = session.id.clone();
    info!("Session created: {} from {}", session_id, ip);

    state.metrics.inc_connections();

    let mut template_rx = state.template_rx.clone();

    loop {
        tokio::select! {
            result = template_rx.changed() => {
                if result.is_err() {
                    break;
                }
                
                // Send new job when template updates
                let template_opt = template_rx.borrow().clone();
                if let Some(template) = template_opt {
                    if let Some(sess) = state.session_manager.get_session(&session_id) {
                        if sess.state == SessionState::Ready {
                            let job = state.job_manager.create_job(&template, &session_id);
                            state.metrics.inc_jobs();
                            state.session_manager.update_session(&session_id, |s| {
                                s.update_job(job.job_id.clone(), job.reserved_value.clone());
                            });
                            
                            let msg = ServerMessage::Job {
                                job_id: job.job_id,
                                blob_hex: job.blob_hex,
                                reserved_offset: job.reserved_offset,
                                reserved_value_hex: hex::encode(&job.reserved_value),
                                target_hex: job.target_hex,
                                height: job.height,
                                seed_hash: job.seed_hash,
                            };
                            if socket.send(Message::Text(serde_json::to_string(&msg).unwrap())).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // Check message rate limit
                        if !state.session_manager.check_message_limit(&session_id) {
                            state.metrics.inc_rate_limits();
                            let msg = ServerMessage::error(None, ErrorCode::RateLimit, "Message rate exceeded");
                            let _ = socket.send(Message::Text(serde_json::to_string(&msg).unwrap())).await;
                            continue;
                        }
                        state.metrics.inc_messages();

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

    state.metrics.dec_connections();
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
            
            // Send initial job if template available
            let template_opt = state.template_rx.borrow().clone();
            if let Some(template) = template_opt {
                let job = state.job_manager.create_job(&template, session_id);
                state.metrics.inc_jobs();
                state.session_manager.update_session(session_id, |s| {
                    s.update_job(job.job_id.clone(), job.reserved_value.clone());
                });
                return Some(ServerMessage::Job {
                    job_id: job.job_id,
                    blob_hex: job.blob_hex,
                    reserved_offset: job.reserved_offset,
                    reserved_value_hex: hex::encode(&job.reserved_value),
                    target_hex: job.target_hex,
                    height: job.height,
                    seed_hash: job.seed_hash,
                });
            }
            
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
        ClientMessage::Submit { id, job_id, blob_hex } => {
            // Check submit rate limit
            if !state.session_manager.check_submit_limit(session_id) {
                state.metrics.inc_rate_limits();
                return Some(ServerMessage::SubmitResult {
                    id, status: SubmitStatus::Error,
                    message: Some("Submit rate exceeded".into()),
                });
            }
            state.metrics.inc_submissions();

            let job = match state.job_manager.get_job(&job_id) {
                Some(j) => j,
                None => {
                    state.metrics.inc_rejected();
                    return Some(ServerMessage::SubmitResult {
                        id, status: SubmitStatus::Rejected,
                        message: Some("Unknown job".into()),
                    });
                }
            };

            // Check if stale
            let current_template_id = {
                let template_ref = state.template_rx.borrow();
                template_ref.as_ref().map(|t| t.template_id).unwrap_or(0)
            };
            
            if state.job_manager.is_stale(&job, current_template_id) {
                state.metrics.inc_stale();
                return Some(ServerMessage::SubmitResult {
                    id, status: SubmitStatus::Stale,
                    message: Some("Job expired".into()),
                });
            }

            // Validate blob
            if let Err(e) = state.validator.validate_blob(&blob_hex, &job) {
                state.metrics.inc_rejected();
                return Some(ServerMessage::SubmitResult {
                    id, status: SubmitStatus::Rejected,
                    message: Some(e.to_string()),
                });
            }

            info!("Valid submission for job {}", job_id);
            state.metrics.inc_accepted();
            Some(ServerMessage::SubmitResult {
                id, status: SubmitStatus::Accepted,
                message: None,
            })
        }
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.expect("Failed to install signal handler");
    info!("Shutdown signal received");
}
