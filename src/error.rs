use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoordinatorError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("RPC error: {0}")]
    Rpc(String),
    
    #[error("WebSocket error: {0}")]
    WebSocket(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),
    
    #[error("Session error: {0}")]
    Session(String),
}
