use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionStatus {
    Pending,
    Active,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    pub link_id: String,
    pub players: Vec<String>,
    pub created_at: i64,
    pub status: ConnectionStatus,
    pub expires_at: i64,
}

impl Connection {
    pub fn new(player_id: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
            
        Connection {
            id: uuid::Uuid::new_v4().to_string(),
            link_id: uuid::Uuid::new_v4().to_string(),
            players: vec![player_id],
            created_at: now,
            status: ConnectionStatus::Pending,
            expires_at: now + 600, // Expires in 10 minutes
        }
    }
 
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
            
        self.expires_at <= now
    }   
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};


    #[test]
    fn test_connection_timestamps_are_set() {
        let connection = Connection::new("player1".to_string());
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        // Created timestamp should be close to now
        assert!(connection.created_at > now - 1);
        assert!(connection.created_at <= now);

        // Expires timestamp should be 10 minutes after creation
        assert_eq!(connection.expires_at, connection.created_at + 600);
    }

    #[test]
    fn test_connection_expiration_status() {
        let mut connection = Connection::new("player1".to_string());
        
        // New connection should be pending
        assert_eq!(connection.status, ConnectionStatus::Pending);

        // Manually expire the connection
        connection.expires_at = 0;
        assert!(connection.is_expired());
    }

    #[test]
    fn test_connection_ids_are_unique() {
        let connection1 = Connection::new("player1".to_string());
        let connection2 = Connection::new("player2".to_string());
        
        assert_ne!(connection1.id, connection2.id);
        assert_ne!(connection1.link_id, connection2.link_id);
    }

    #[test]
    fn test_connection_ids_are_valid_uuids() {
        let connection = Connection::new("player1".to_string());
        
        // Test that both id and link_id are valid UUIDs
        assert!(uuid::Uuid::parse_str(&connection.id).is_ok());
        assert!(uuid::Uuid::parse_str(&connection.link_id).is_ok());
    }

    #[test]
    fn test_new_connection() {
        let connection = Connection::new("player123".to_string());
        assert_eq!(connection.players[0], "player123");
        assert_eq!(connection.status, ConnectionStatus::Pending);
    }
    
    #[test]
    fn test_create_new_connection() {
        let player_id = "player123".to_string();
        let connection = Connection {
            id: "test_id".to_string(),
            link_id: "test_link".to_string(),
            players: vec![player_id],
            created_at: 0,
            status: ConnectionStatus::Pending,
            expires_at: 0,
        };
        
        assert_eq!(connection.players.len(), 1);
        assert_eq!(connection.status, ConnectionStatus::Pending);
    }
}