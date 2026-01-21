use anyhow::Result;
use tracing::info;
use std::sync::Arc;

mod config;
mod error;
mod jobs;
mod metrics;
mod protocol;
mod ratelimit;
mod rpc;
mod server;
mod session;
mod template;
mod validator;

use jobs::JobManager;
use metrics::Metrics;
use session::SessionManager;
use template::TemplateManager;
use validator::SubmissionValidator;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("monero_web_coordinator=info".parse()?)
        )
        .init();

    info!("Starting Coordinator");

    let config = config::load_config()?;
    info!("Configuration loaded");

    let metrics = Arc::new(Metrics::new());

    let session_manager = Arc::new(SessionManager::new(
        config.server.max_connections_per_ip,
        config.server.max_connections,
        config.limits.messages_per_second,
        config.limits.submits_per_minute,
    ));
    let job_manager = Arc::new(JobManager::new(config.jobs.stale_job_grace_ms));
    let validator = Arc::new(SubmissionValidator::new());
    
    let mut template_manager = TemplateManager::new(&config)?;
    let template_rx = template_manager.subscribe();
    let rpc_client = template_manager.client();

    // Start metrics server
    let metrics_config = config.metrics.clone();
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        metrics::run_metrics_server(metrics_config, metrics_clone).await;
    });

    // Template manager
    let metrics_tpl = metrics.clone();
    tokio::spawn(async move {
        template_manager.run(metrics_tpl).await;
    });

    // Periodic job cleanup
    let job_mgr_clone = job_manager.clone();
    let job_ttl = config.jobs.job_ttl_ms;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            job_mgr_clone.cleanup_old_jobs(job_ttl);
        }
    });

    server::run(config, template_rx, rpc_client, session_manager, job_manager, validator, metrics).await?;

    Ok(())
}
