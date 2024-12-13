use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("Connection not found")]
    NotFound,
    
    #[error("Connection already exists")]
    AlreadyExists,
    
    #[error("Connection expired")]
    Expired,
    
    #[error("Connection rejected")]
    Rejected,
    
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Invalid connection request: {0}")]
    InvalidRequest(String),
}