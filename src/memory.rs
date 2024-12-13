use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

use crate::connection::ConnectionManager;
use crate::error::ConnectionError;
use crate::models::{Connection, ConnectionRequest, ConnectionStatus};
use crate::validation::{ConnectionValidator, DefaultConnectionValidator};

pub struct InMemoryConnectionManager {
    connections: Arc<RwLock<HashMap<String, Connection>>>,
    requests: Arc<RwLock<HashMap<String, ConnectionRequest>>>,
    validator: Box<dyn ConnectionValidator + Send + Sync>,
}

impl InMemoryConnectionManager {
    pub fn new() -> Self {
        InMemoryConnectionManager {
            connections: Arc::new(RwLock::new(HashMap::new())),
            requests: Arc::new(RwLock::new(HashMap::new())),
            validator: Box::new(DefaultConnectionValidator::new()),
        }
    }
}

#[async_trait::async_trait]
impl ConnectionManager for InMemoryConnectionManager {
    async fn create_connection(
        &self,
        initiator_id: String,
        initiator_label: String,
    ) -> Result<ConnectionRequest, ConnectionError> {
        let connection_id = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + Duration::days(7);

        let request = ConnectionRequest {
            connection_id: connection_id.clone(),
            from_profile_id: initiator_id.clone(),
            to_profile_id: None,
            expires_at,
        };

        // Validate request
        self.validator.validate_request(&request)?;

        let connection = Connection {
            id: connection_id,
            initiator_id,
            recipient_id: None,
            initiator_label,
            recipient_label: None,
            status: ConnectionStatus::Pending,
            created_at: Utc::now(),
            connected_at: None,
        };

        // Validate connection
        self.validator.validate_connection(&connection)?;

        // Store both the request and initial connection
        self.requests.write().await.insert(request.connection_id.clone(), request.clone());
        self.connections.write().await.insert(connection.id.clone(), connection);

        Ok(request)
    }

    async fn accept_connection(
        &self,
        connection_id: &str,
        recipient_id: String,
        recipient_label: String,
    ) -> Result<Connection, ConnectionError> {
        let mut connections = self.connections.write().await;
        
        let connection = connections.get_mut(connection_id)
            .ok_or(ConnectionError::NotFound)?;

        // Validate current state
        self.validator.validate_connection(connection)?;

        if connection.status != ConnectionStatus::Pending {
            return Err(ConnectionError::InvalidRequest("Connection is not pending".to_string()));
        }

        // Update connection
        connection.recipient_id = Some(recipient_id);
        connection.recipient_label = Some(recipient_label);
        connection.status = ConnectionStatus::Active;
        connection.connected_at = Some(Utc::now());

        // Validate updated state
        self.validator.validate_connection(connection)?;

        Ok(connection.clone())
    }

    async fn list_connections(
        &self,
        profile_id: &str
    ) -> Result<Vec<Connection>, ConnectionError> {
        let connections = self.connections.read().await;
        
        let user_connections: Vec<Connection> = connections.values()
            .filter(|conn| {
                conn.initiator_id == profile_id || 
                conn.recipient_id.as_ref().map_or(false, |id| id == profile_id)
            })
            .cloned()
            .collect();

        Ok(user_connections)
    }

    async fn get_connection(
        &self,
        connection_id: &str
    ) -> Result<Connection, ConnectionError> {
        let connections = self.connections.read().await;
        
        connections.get(connection_id)
            .cloned()
            .ok_or(ConnectionError::NotFound)
    }

    async fn list_all_connections(&self) -> Result<Vec<Connection>, ConnectionError> {
        let connections = self.connections.read().await;
        Ok(connections.values().cloned().collect())
    }

    async fn delete_connection(
        &self,
        connection_id: &str
    ) -> Result<(), ConnectionError> {
        let mut connections = self.connections.write().await;
        
        if connections.remove(connection_id).is_some() {
            Ok(())
        } else {
            Err(ConnectionError::NotFound)
        }
    }

    async fn get_request(
        &self,
        connection_id: &str
    ) -> Result<ConnectionRequest, ConnectionError> {
        let requests = self.requests.read().await;
        
        requests.get(connection_id)
            .cloned()
            .ok_or(ConnectionError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_accept_connection() {
        let manager = InMemoryConnectionManager::new();
        
        // Create connection
        let request = manager.create_connection(
            "user1".to_string(),
            "Friend".to_string(),
        ).await.unwrap();

        // Accept connection
        let connection = manager.accept_connection(
            &request.connection_id,
            "user2".to_string(),
            "My Friend".to_string(),
        ).await.unwrap();

        assert_eq!(connection.status, ConnectionStatus::Active);
        assert_eq!(connection.recipient_id, Some("user2".to_string()));
        assert_eq!(connection.recipient_label, Some("My Friend".to_string()));
    }

    #[tokio::test]
    async fn test_list_connections() {
        let manager = InMemoryConnectionManager::new();
        
        // Create two connections
        let request1 = manager.create_connection(
            "user1".to_string(),
            "Friend 1".to_string(),
        ).await.unwrap();

        let request2 = manager.create_connection(
            "user1".to_string(),
            "Friend 2".to_string(),
        ).await.unwrap();

        // Accept both connections
        manager.accept_connection(
            &request1.connection_id,
            "user2".to_string(),
            "My Friend 1".to_string(),
        ).await.unwrap();

        manager.accept_connection(
            &request2.connection_id,
            "user3".to_string(),
            "My Friend 2".to_string(),
        ).await.unwrap();

        // List connections for user1
        let connections = manager.list_connections("user1").await.unwrap();
        assert_eq!(connections.len(), 2);
    }
}