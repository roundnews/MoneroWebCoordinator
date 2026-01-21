use anyhow::Result;
use tracing::info;
use std::sync::Arc;

mod config;
mod error;
mod protocol;
mod rpc;
mod server;
mod session;
mod template;

use session::SessionManager;
use template::TemplateManager;

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
    
    let mut template_manager = TemplateManager::new(&config)?;
    let template_rx = template_manager.subscribe();
    let rpc_client = template_manager.client();

    tokio::spawn(async move {
        template_manager.run().await;
    });

    server::run(config, template_rx, rpc_client, session_manager).await?;

    Ok(())
}
