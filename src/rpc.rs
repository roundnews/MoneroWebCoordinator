use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RpcError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("RPC error: {code} - {message}")]
    Rpc { code: i32, message: String },
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

pub struct MonerodClient {
    client: Client,
    rpc_url: String,
}

#[derive(Serialize)]
struct JsonRpcRequest<T: Serialize> {
    jsonrpc: &'static str,
    id: &'static str,
    method: &'static str,
    params: T,
}

#[derive(Deserialize)]
struct JsonRpcResponse<T> {
    result: Option<T>,
    error: Option<RpcErrorResponse>,
}

#[derive(Deserialize)]
struct RpcErrorResponse {
    code: i32,
    message: String,
}

// get_block_template request/response
#[derive(Serialize)]
pub struct GetBlockTemplateParams {
    pub wallet_address: String,
    pub reserve_size: u8,
}

#[derive(Deserialize, Clone, Debug)]
pub struct BlockTemplate {
    pub blockhashing_blob: String,
    pub blocktemplate_blob: String,
    pub difficulty: u64,
    pub expected_reward: u64,
    pub height: u64,
    pub prev_hash: String,
    pub reserved_offset: usize,
    pub seed_hash: String,
    pub status: String,
}

// get_info response
#[derive(Deserialize, Debug)]
pub struct DaemonInfo {
    pub height: u64,
    pub top_block_hash: String,
    pub status: String,
    pub version: String,
}

impl MonerodClient {
    pub fn new(rpc_url: String, timeout_ms: u64) -> Result<Self, RpcError> {
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|e| RpcError::Http(e))?;
        
        Ok(Self { client, rpc_url })
    }

    async fn call<P: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        method: &'static str,
        params: P,
    ) -> Result<R, RpcError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: "0",
            method,
            params,
        };

        let response = self
            .client
            .post(&format!("{}/json_rpc", self.rpc_url))
            .json(&request)
            .send()
            .await?
            .json::<JsonRpcResponse<R>>()
            .await?;

        if let Some(err) = response.error {
            return Err(RpcError::Rpc {
                code: err.code,
                message: err.message,
            });
        }

        response
            .result
            .ok_or_else(|| RpcError::InvalidResponse("Missing result".into()))
    }

    pub async fn get_block_template(
        &self,
        wallet_address: &str,
        reserve_size: u8,
    ) -> Result<BlockTemplate, RpcError> {
        self.call(
            "get_block_template",
            GetBlockTemplateParams {
                wallet_address: wallet_address.to_string(),
                reserve_size,
            },
        )
        .await
    }

    pub async fn submit_block(&self, block_blob_hex: &str) -> Result<String, RpcError> {
        #[derive(Deserialize)]
        struct SubmitResponse {
            status: String,
        }
        
        let result: SubmitResponse = self
            .call("submit_block", vec![block_blob_hex])
            .await?;
        
        Ok(result.status)
    }

    pub async fn get_info(&self) -> Result<DaemonInfo, RpcError> {
        #[derive(Serialize)]
        struct Empty {}
        self.call("get_info", Empty {}).await
    }
}
