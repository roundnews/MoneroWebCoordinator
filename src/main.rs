use anyhow::Result;
use tracing::info;

mod config;
mod error;
mod server;

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

    // Load configuration
    let config = config::load_config()?;
    info!("Configuration loaded successfully");
    info!("Server will bind to: {}", config.server.bind_addr);
    info!("WebSocket path: {}", config.server.ws_path);
    info!("Monerod RPC URL: {}", config.monerod.rpc_url);

    // Start the server
    server::run(config).await?;

    Ok(())
}
