use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionStatus {
    Pending,
    Active,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    pub initiator_id: String,
    pub recipient_id: Option<String>,
    pub initiator_label: String,
    pub recipient_label: Option<String>,
    pub status: ConnectionStatus,
    pub created_at: DateTime<Utc>,
    pub connected_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionRequest {
    pub connection_id: String,
    pub from_profile_id: String,
    pub to_profile_id: Option<String>,
    pub expires_at: DateTime<Utc>,
}