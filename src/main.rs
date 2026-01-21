use anyhow::Result;
use tracing::info;

mod config;
mod error;
mod rpc;
mod server;
mod template;

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

    info!("Starting Monero Web Coordinator");

    let config = config::load_config()?;
    info!("Configuration loaded");
    info!("Server bind address: {}", config.server.bind_addr);
    info!("Monerod RPC: {}", config.monerod.rpc_url);

    // Create template manager
    let mut template_manager = TemplateManager::new(&config);
    let template_rx = template_manager.subscribe();
    let rpc_client = template_manager.client();

    // Spawn template manager task
    tokio::spawn(async move {
        template_manager.run().await;
    });

    // Start server with template subscription
    server::run(config, template_rx, rpc_client).await?;

    Ok(())
}
