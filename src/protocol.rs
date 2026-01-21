use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Hello {
        v: u8,
        client_version: String,
        threads: u8,
        #[serde(skip_serializing_if = "Option::is_none")]
        site_token: Option<String>,
    },
    Submit {
        id: String,
        job_id: String,
        blob_hex: String,
    },
    Ping {
        id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Stats {
        id: Option<String>,
        session_id: String,
        submits_per_minute: u32,
        messages_per_second: u32,
    },
    Job {
        job_id: String,
        blob_hex: String,
        reserved_offset: usize,
        reserved_value_hex: String,
        target_hex: String,
        height: u64,
        seed_hash: String,
    },
    SubmitResult {
        id: String,
        status: SubmitStatus,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    Error {
        id: Option<String>,
        code: ErrorCode,
        message: String,
    },
    Pong {
        id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SubmitStatus {
    Accepted,
    Rejected,
    Stale,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    BadFormat,
    RateLimit,
    StaleJob,
    InvalidData,
    InternalError,
    NotReady,
}

impl ServerMessage {
    pub fn error(id: Option<String>, code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Error {
            id,
            code,
            message: message.into(),
        }
    }
}
