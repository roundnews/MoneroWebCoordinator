use serde::{Deserialize, Serialize};

// Message envelope wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope<T> {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub v: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub payload: T,
}

impl<T> MessageEnvelope<T> {
    pub fn new(msg_type: impl Into<String>, payload: T, id: Option<String>) -> Self {
        Self {
            msg_type: msg_type.into(),
            v: 1,
            id,
            payload,
        }
    }
}

// Client message payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloPayload {
    pub miner_version: String,
    pub threads: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_binary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub randomx_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitPayload {
    pub job_id: String,
    pub nonce: String,  // 4-byte nonce as hex (8 chars)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharePayload {
    pub job_id: String,
    pub nonce: u32,
    pub result_hash_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingPayload {}

// Client message enum for deserialization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Hello {
        v: u8,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        payload: HelloPayload,
    },
    Submit {
        v: u8,
        id: String,
        payload: SubmitPayload,
    },
    Share {
        v: u8,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        payload: SharePayload,
    },
    Ping {
        v: u8,
        id: String,
        payload: PingPayload,
    },
}

// Server message payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyPayload {
    pub job_ttl_ms: u64,
    pub max_submits_per_min: u32,
    pub max_shares_per_min: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsPayload {
    pub session_id: String,
    pub submits_per_minute: u32,
    pub messages_per_second: u32,
    pub policy: PolicyPayload,
    pub server_time_ms: u64,
    pub tip_height: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobPayload {
    pub job_id: String,
    pub blob_hex: String,
    pub reserved_offset: usize,
    pub reserved_value_hex: String,
    pub target_hex: String,
    pub height: u64,
    pub seed_hash: String,
    pub expires_at_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share_target_hex: Option<String>,
    pub algo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitResultPayload {
    pub status: SubmitStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PongPayload {}

// Server message enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Stats {
        v: u8,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        payload: StatsPayload,
    },
    Job {
        v: u8,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        payload: JobPayload,
    },
    SubmitResult {
        v: u8,
        id: String,
        payload: SubmitResultPayload,
    },
    Error {
        v: u8,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        payload: ErrorPayload,
    },
    Pong {
        v: u8,
        id: String,
        payload: PongPayload,
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
    Unauthorized,
    RateLimit,
    StaleJob,
    BadJob,
    BadReserved,
    BadPow,
    Internal,
    RpcDown,
}

impl ServerMessage {
    pub fn error(id: Option<String>, code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Error {
            v: 1,
            id,
            payload: ErrorPayload {
                code,
                message: message.into(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_message_serialization() {
        let hello = ClientMessage::Hello {
            v: 1,
            id: Some("test-id".to_string()),
            payload: HelloPayload {
                miner_version: "1.0.0".to_string(),
                threads: 4,
                site_token: Some("token123".to_string()),
                user_agent_hint: Some("Chrome".to_string()),
                supports_binary: Some(false),
                randomx_mode: Some("fast".to_string()),
            },
        };

        let json = serde_json::to_string(&hello).unwrap();
        assert!(json.contains("\"type\":\"hello\""));
        assert!(json.contains("\"v\":1"));
        assert!(json.contains("\"id\":\"test-id\""));
        assert!(json.contains("\"miner_version\":\"1.0.0\""));
        assert!(json.contains("\"threads\":4"));
        assert!(json.contains("\"site_token\":\"token123\""));
        assert!(json.contains("\"user_agent_hint\":\"Chrome\""));
        assert!(json.contains("\"supports_binary\":false"));
        assert!(json.contains("\"randomx_mode\":\"fast\""));

        // Test deserialization
        let deserialized: ClientMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientMessage::Hello { v, id, payload } => {
                assert_eq!(v, 1);
                assert_eq!(id, Some("test-id".to_string()));
                assert_eq!(payload.miner_version, "1.0.0");
                assert_eq!(payload.threads, 4);
                assert_eq!(payload.site_token, Some("token123".to_string()));
                assert_eq!(payload.user_agent_hint, Some("Chrome".to_string()));
                assert_eq!(payload.supports_binary, Some(false));
                assert_eq!(payload.randomx_mode, Some("fast".to_string()));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_submit_message_serialization() {
        let submit = ClientMessage::Submit {
            v: 1,
            id: "submit-123".to_string(),
            payload: SubmitPayload {
                job_id: "job-456".to_string(),
                nonce: "12345678".to_string(),
            },
        };

        let json = serde_json::to_string(&submit).unwrap();
        assert!(json.contains("\"type\":\"submit\""));
        assert!(json.contains("\"v\":1"));
        assert!(json.contains("\"id\":\"submit-123\""));
        assert!(json.contains("\"job_id\":\"job-456\""));
        assert!(json.contains("\"nonce\":\"12345678\""));

        // Test deserialization
        let deserialized: ClientMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientMessage::Submit { v, id, payload } => {
                assert_eq!(v, 1);
                assert_eq!(id, "submit-123");
                assert_eq!(payload.job_id, "job-456");
                assert_eq!(payload.nonce, "12345678");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_share_message_serialization() {
        let share = ClientMessage::Share {
            v: 1,
            id: Some("share-789".to_string()),
            payload: SharePayload {
                job_id: "job-456".to_string(),
                nonce: 0x12345678,
                result_hash_hex: "abcdef0123456789".to_string(),
            },
        };

        let json = serde_json::to_string(&share).unwrap();
        assert!(json.contains("\"type\":\"share\""));
        assert!(json.contains("\"v\":1"));
        assert!(json.contains("\"id\":\"share-789\""));
        assert!(json.contains("\"job_id\":\"job-456\""));
        assert!(json.contains("\"nonce\":305419896"));
        assert!(json.contains("\"result_hash_hex\":\"abcdef0123456789\""));

        // Test deserialization
        let deserialized: ClientMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientMessage::Share { v, id, payload } => {
                assert_eq!(v, 1);
                assert_eq!(id, Some("share-789".to_string()));
                assert_eq!(payload.job_id, "job-456");
                assert_eq!(payload.nonce, 0x12345678);
                assert_eq!(payload.result_hash_hex, "abcdef0123456789");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_ping_message_serialization() {
        let ping = ClientMessage::Ping {
            v: 1,
            id: "ping-123".to_string(),
            payload: PingPayload {},
        };

        let json = serde_json::to_string(&ping).unwrap();
        assert!(json.contains("\"type\":\"ping\""));
        assert!(json.contains("\"v\":1"));
        assert!(json.contains("\"id\":\"ping-123\""));

        // Test deserialization
        let deserialized: ClientMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientMessage::Ping { v, id, .. } => {
                assert_eq!(v, 1);
                assert_eq!(id, "ping-123");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_stats_message_serialization() {
        let stats = ServerMessage::Stats {
            v: 1,
            id: Some("stats-123".to_string()),
            payload: StatsPayload {
                session_id: "session-abc".to_string(),
                submits_per_minute: 10,
                messages_per_second: 20,
                policy: PolicyPayload {
                    job_ttl_ms: 30000,
                    max_submits_per_min: 10,
                    max_shares_per_min: 120,
                },
                server_time_ms: 1640000000000,
                tip_height: 2500000,
            },
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"type\":\"stats\""));
        assert!(json.contains("\"v\":1"));
        assert!(json.contains("\"id\":\"stats-123\""));
        assert!(json.contains("\"session_id\":\"session-abc\""));
        assert!(json.contains("\"submits_per_minute\":10"));
        assert!(json.contains("\"messages_per_second\":20"));
        assert!(json.contains("\"policy\""));
        assert!(json.contains("\"job_ttl_ms\":30000"));
        assert!(json.contains("\"max_submits_per_min\":10"));
        assert!(json.contains("\"max_shares_per_min\":120"));
        assert!(json.contains("\"server_time_ms\":1640000000000"));
        assert!(json.contains("\"tip_height\":2500000"));

        // Test deserialization
        let deserialized: ServerMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            ServerMessage::Stats { v, id, payload } => {
                assert_eq!(v, 1);
                assert_eq!(id, Some("stats-123".to_string()));
                assert_eq!(payload.session_id, "session-abc");
                assert_eq!(payload.submits_per_minute, 10);
                assert_eq!(payload.messages_per_second, 20);
                assert_eq!(payload.policy.job_ttl_ms, 30000);
                assert_eq!(payload.policy.max_submits_per_min, 10);
                assert_eq!(payload.policy.max_shares_per_min, 120);
                assert_eq!(payload.server_time_ms, 1640000000000);
                assert_eq!(payload.tip_height, 2500000);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_job_message_serialization() {
        let job = ServerMessage::Job {
            v: 1,
            id: None,
            payload: JobPayload {
                job_id: "job-123".to_string(),
                blob_hex: "0a0a0a".to_string(),
                reserved_offset: 42,
                reserved_value_hex: "deadbeef".to_string(),
                target_hex: "ffffff".to_string(),
                height: 2500000,
                seed_hash: "abc123".to_string(),
                expires_at_ms: 1640030000000,
                share_target_hex: Some("aaaaaa".to_string()),
                algo: "randomx".to_string(),
            },
        };

        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains("\"type\":\"job\""));
        assert!(json.contains("\"v\":1"));
        assert!(json.contains("\"job_id\":\"job-123\""));
        assert!(json.contains("\"blob_hex\":\"0a0a0a\""));
        assert!(json.contains("\"reserved_offset\":42"));
        assert!(json.contains("\"reserved_value_hex\":\"deadbeef\""));
        assert!(json.contains("\"target_hex\":\"ffffff\""));
        assert!(json.contains("\"height\":2500000"));
        assert!(json.contains("\"seed_hash\":\"abc123\""));
        assert!(json.contains("\"expires_at_ms\":1640030000000"));
        assert!(json.contains("\"share_target_hex\":\"aaaaaa\""));
        assert!(json.contains("\"algo\":\"randomx\""));

        // Test deserialization
        let deserialized: ServerMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            ServerMessage::Job { v, id, payload } => {
                assert_eq!(v, 1);
                assert_eq!(id, None);
                assert_eq!(payload.job_id, "job-123");
                assert_eq!(payload.blob_hex, "0a0a0a");
                assert_eq!(payload.reserved_offset, 42);
                assert_eq!(payload.reserved_value_hex, "deadbeef");
                assert_eq!(payload.target_hex, "ffffff");
                assert_eq!(payload.height, 2500000);
                assert_eq!(payload.seed_hash, "abc123");
                assert_eq!(payload.expires_at_ms, 1640030000000);
                assert_eq!(payload.share_target_hex, Some("aaaaaa".to_string()));
                assert_eq!(payload.algo, "randomx");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_submit_result_message_serialization() {
        let submit_result = ServerMessage::SubmitResult {
            v: 1,
            id: "submit-123".to_string(),
            payload: SubmitResultPayload {
                status: SubmitStatus::Accepted,
                message: Some("Block accepted".to_string()),
            },
        };

        let json = serde_json::to_string(&submit_result).unwrap();
        assert!(json.contains("\"type\":\"submit_result\""));
        assert!(json.contains("\"v\":1"));
        assert!(json.contains("\"id\":\"submit-123\""));
        assert!(json.contains("\"status\":\"ACCEPTED\""));
        assert!(json.contains("\"message\":\"Block accepted\""));

        // Test deserialization
        let deserialized: ServerMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            ServerMessage::SubmitResult { v, id, payload } => {
                assert_eq!(v, 1);
                assert_eq!(id, "submit-123");
                match payload.status {
                    SubmitStatus::Accepted => {}
                    _ => panic!("Wrong status"),
                }
                assert_eq!(payload.message, Some("Block accepted".to_string()));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_error_message_serialization() {
        let error = ServerMessage::error(
            Some("req-123".to_string()),
            ErrorCode::BadFormat,
            "Invalid message format",
        );

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("\"type\":\"error\""));
        assert!(json.contains("\"v\":1"));
        assert!(json.contains("\"id\":\"req-123\""));
        assert!(json.contains("\"code\":\"BAD_FORMAT\""));
        assert!(json.contains("\"message\":\"Invalid message format\""));

        // Test deserialization
        let deserialized: ServerMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            ServerMessage::Error { v, id, payload } => {
                assert_eq!(v, 1);
                assert_eq!(id, Some("req-123".to_string()));
                match payload.code {
                    ErrorCode::BadFormat => {}
                    _ => panic!("Wrong error code"),
                }
                assert_eq!(payload.message, "Invalid message format");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_pong_message_serialization() {
        let pong = ServerMessage::Pong {
            v: 1,
            id: "ping-123".to_string(),
            payload: PongPayload {},
        };

        let json = serde_json::to_string(&pong).unwrap();
        assert!(json.contains("\"type\":\"pong\""));
        assert!(json.contains("\"v\":1"));
        assert!(json.contains("\"id\":\"ping-123\""));

        // Test deserialization
        let deserialized: ServerMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            ServerMessage::Pong { v, id, .. } => {
                assert_eq!(v, 1);
                assert_eq!(id, "ping-123");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_all_error_codes() {
        let error_codes = vec![
            ErrorCode::BadFormat,
            ErrorCode::Unauthorized,
            ErrorCode::RateLimit,
            ErrorCode::StaleJob,
            ErrorCode::BadJob,
            ErrorCode::BadReserved,
            ErrorCode::BadPow,
            ErrorCode::Internal,
            ErrorCode::RpcDown,
        ];

        let expected_names = vec![
            "BAD_FORMAT",
            "UNAUTHORIZED",
            "RATE_LIMIT",
            "STALE_JOB",
            "BAD_JOB",
            "BAD_RESERVED",
            "BAD_POW",
            "INTERNAL",
            "RPC_DOWN",
        ];

        for (code, expected) in error_codes.iter().zip(expected_names.iter()) {
            let json = serde_json::to_string(&code).unwrap();
            assert_eq!(json, format!("\"{}\"", expected));
        }
    }

    #[test]
    fn test_all_submit_statuses() {
        let statuses = vec![
            SubmitStatus::Accepted,
            SubmitStatus::Rejected,
            SubmitStatus::Stale,
            SubmitStatus::Error,
        ];

        let expected_names = vec!["ACCEPTED", "REJECTED", "STALE", "ERROR"];

        for (status, expected) in statuses.iter().zip(expected_names.iter()) {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, format!("\"{}\"", expected));
        }
    }
}
