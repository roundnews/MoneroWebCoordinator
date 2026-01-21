use axum::{Router, routing::get};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

use crate::config::MetricsConfig;

#[derive(Default)]
pub struct Metrics {
    pub connections_total: AtomicU64,
    pub connections_active: AtomicU64,
    pub messages_received: AtomicU64,
    pub submissions_total: AtomicU64,
    pub submissions_accepted: AtomicU64,
    pub submissions_rejected: AtomicU64,
    pub submissions_stale: AtomicU64,
    pub jobs_created: AtomicU64,
    pub templates_received: AtomicU64,
    pub rate_limits_hit: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inc_connections(&self) {
        self.connections_total.fetch_add(1, Ordering::Relaxed);
        self.connections_active.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec_connections(&self) {
        self.connections_active.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn inc_messages(&self) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_submissions(&self) {
        self.submissions_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_accepted(&self) {
        self.submissions_accepted.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_rejected(&self) {
        self.submissions_rejected.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_stale(&self) {
        self.submissions_stale.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_jobs(&self) {
        self.jobs_created.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_templates(&self) {
        self.templates_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_rate_limits(&self) {
        self.rate_limits_hit.fetch_add(1, Ordering::Relaxed);
    }

    fn format_prometheus(&self) -> String {
        format!(
            "# HELP coordinator_connections_total Total connections\n\
             # TYPE coordinator_connections_total counter\n\
             coordinator_connections_total {}\n\
             # HELP coordinator_connections_active Active connections\n\
             # TYPE coordinator_connections_active gauge\n\
             coordinator_connections_active {}\n\
             # HELP coordinator_messages_received Total messages received\n\
             # TYPE coordinator_messages_received counter\n\
             coordinator_messages_received {}\n\
             # HELP coordinator_submissions_total Total submissions\n\
             # TYPE coordinator_submissions_total counter\n\
             coordinator_submissions_total {}\n\
             # HELP coordinator_submissions_accepted Accepted submissions\n\
             # TYPE coordinator_submissions_accepted counter\n\
             coordinator_submissions_accepted {}\n\
             # HELP coordinator_submissions_rejected Rejected submissions\n\
             # TYPE coordinator_submissions_rejected counter\n\
             coordinator_submissions_rejected {}\n\
             # HELP coordinator_submissions_stale Stale submissions\n\
             # TYPE coordinator_submissions_stale counter\n\
             coordinator_submissions_stale {}\n\
             # HELP coordinator_jobs_created Jobs created\n\
             # TYPE coordinator_jobs_created counter\n\
             coordinator_jobs_created {}\n\
             # HELP coordinator_templates_received Templates received\n\
             # TYPE coordinator_templates_received counter\n\
             coordinator_templates_received {}\n\
             # HELP coordinator_rate_limits_hit Rate limits triggered\n\
             # TYPE coordinator_rate_limits_hit counter\n\
             coordinator_rate_limits_hit {}\n",
            self.connections_total.load(Ordering::Relaxed),
            self.connections_active.load(Ordering::Relaxed),
            self.messages_received.load(Ordering::Relaxed),
            self.submissions_total.load(Ordering::Relaxed),
            self.submissions_accepted.load(Ordering::Relaxed),
            self.submissions_rejected.load(Ordering::Relaxed),
            self.submissions_stale.load(Ordering::Relaxed),
            self.jobs_created.load(Ordering::Relaxed),
            self.templates_received.load(Ordering::Relaxed),
            self.rate_limits_hit.load(Ordering::Relaxed),
        )
    }
}

pub async fn run_metrics_server(config: MetricsConfig, metrics: Arc<Metrics>) {
    if !config.enable {
        return;
    }

    let path = config.path.clone();
    let app = Router::new()
        .route(&path, get(move || {
            let m = metrics.clone();
            async move { m.format_prometheus() }
        }));

    let addr: std::net::SocketAddr = match config.bind_addr.parse() {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Invalid metrics bind address: {}", e);
            return;
        }
    };

    info!("Metrics server listening on {}{}", addr, path);

    if let Ok(listener) = TcpListener::bind(addr).await {
        let _ = axum::serve(listener, app).await;
    }
}
