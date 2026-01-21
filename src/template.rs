use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::watch;
use tokio::time::interval;
use tracing::{info, warn, error};

use crate::config::Config;
use crate::rpc::{MonerodClient, BlockTemplate, RpcError};

#[derive(Clone, Debug)]
pub struct TemplateState {
    pub template_id: u64,
    pub height: u64,
    pub prev_hash: String,
    pub blocktemplate_blob: String,
    pub blockhashing_blob: String,
    pub difficulty: u64,
    pub reserved_offset: usize,
    pub reserve_size: u8,
    pub seed_hash: String,
    pub created_at: Instant,
}

impl TemplateState {
    pub fn from_rpc(template: BlockTemplate, template_id: u64, reserve_size: u8) -> Self {
        Self {
            template_id,
            height: template.height,
            prev_hash: template.prev_hash,
            blocktemplate_blob: template.blocktemplate_blob,
            blockhashing_blob: template.blockhashing_blob,
            difficulty: template.difficulty,
            reserved_offset: template.reserved_offset,
            reserve_size,
            seed_hash: template.seed_hash,
            created_at: Instant::now(),
        }
    }
}

pub struct TemplateManager {
    client: Arc<MonerodClient>,
    wallet_address: String,
    reserve_size: u8,
    refresh_interval: Duration,
    sender: watch::Sender<Option<TemplateState>>,
    receiver: watch::Receiver<Option<TemplateState>>,
    template_counter: u64,
}

impl TemplateManager {
    pub fn new(config: &Config) -> Result<Self, RpcError> {
        let client = Arc::new(MonerodClient::new(
            config.monerod.rpc_url.clone(),
            config.monerod.rpc_timeout_ms,
        )?);

        let (sender, receiver) = watch::channel(None);

        Ok(Self {
            client,
            wallet_address: config.monerod.wallet_address.clone(),
            reserve_size: config.monerod.reserve_size,
            refresh_interval: Duration::from_millis(config.jobs.template_refresh_interval_ms),
            sender,
            receiver,
            template_counter: 0,
        })
    }

    pub fn subscribe(&self) -> watch::Receiver<Option<TemplateState>> {
        self.receiver.clone()
    }

    pub fn client(&self) -> Arc<MonerodClient> {
        self.client.clone()
    }

    pub async fn run(&mut self, metrics: Arc<crate::metrics::Metrics>) {
        info!("Template manager starting");
        
        if let Err(e) = self.refresh_template().await {
            error!("Initial template fetch failed: {}", e);
        } else {
            metrics.inc_templates();
        }

        let mut ticker = interval(self.refresh_interval);
        let mut last_height: u64 = 0;

        loop {
            ticker.tick().await;

            match self.client.get_info().await {
                Ok(info) => {
                    if info.height != last_height {
                        info!("New block at height {}", info.height);
                        last_height = info.height;
                        if self.refresh_template().await.is_ok() {
                            metrics.inc_templates();
                        }
                    }
                }
                Err(e) => {
                    warn!("Daemon info failed: {}", e);
                }
            }
        }
    }

    async fn refresh_template(&mut self) -> Result<(), RpcError> {
        let template = self
            .client
            .get_block_template(&self.wallet_address, self.reserve_size)
            .await?;

        self.template_counter += 1;
        let state = TemplateState::from_rpc(template, self.template_counter, self.reserve_size);
        
        info!(
            "New template: id={}, height={}, difficulty={}",
            state.template_id, state.height, state.difficulty
        );

        let _ = self.sender.send(Some(state));
        Ok(())
    }
}
