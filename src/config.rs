use serde::Deserialize;
use std::fs;
use std::env;
use anyhow::{Context, Result};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub monerod: MonerodConfig,
    pub jobs: JobsConfig,
    pub limits: LimitsConfig,
    pub metrics: MetricsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub ws_path: String,
    pub max_connections: usize,
    pub max_connections_per_ip: usize,
    pub max_frame_bytes: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MonerodConfig {
    pub rpc_url: String,
    pub wallet_address: String,
    pub reserve_size: u8,
    pub rpc_timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JobsConfig {
    pub job_ttl_ms: u64,
    pub template_refresh_interval_ms: u64,
    pub stale_job_grace_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LimitsConfig {
    pub submits_per_minute: u32,
    pub shares_per_minute: u32,
    pub messages_per_second: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricsConfig {
    pub enable: bool,
    pub bind_addr: String,
    pub path: String,
}

pub fn load_config() -> Result<Config> {
    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string());
    
    let config_content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path))?;
    
    let config: Config = toml::from_str(&config_content)
        .with_context(|| "Failed to parse configuration")?;
    
    Ok(config)
}
