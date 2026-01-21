use anyhow::Result;
use tracing::info;
use std::sync::Arc;

mod config;
mod error;
mod jobs;
mod protocol;
mod rpc;
mod server;
mod session;
mod template;
mod validator;

use jobs::JobManager;
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
    info!("Server: {}", config.server.bind_addr);

    let session_manager = Arc::new(SessionManager::new(config.server.max_connections_per_ip));
    let job_manager = Arc::new(JobManager::new(config.jobs.stale_job_grace_ms));
    let validator = Arc::new(SubmissionValidator::new());
    
    let mut template_manager = TemplateManager::new(&config)?;
    let template_rx = template_manager.subscribe();
    let rpc_client = template_manager.client();

    tokio::spawn(async move {
        template_manager.run().await;
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

    server::run(config, template_rx, rpc_client, session_manager, job_manager, validator).await?;

    Ok(())
}
