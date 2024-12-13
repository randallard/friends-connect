use async_trait::async_trait;
use crate::error::ConnectionError;
use crate::models::{Connection, ConnectionRequest};

#[async_trait]
pub trait ConnectionManager {
    /// Creates a new connection request
    async fn create_connection(&self, 
        initiator_id: String,
        initiator_label: String,
    ) -> Result<ConnectionRequest, ConnectionError>;

    /// Accepts an existing connection request
    async fn accept_connection(&self,
        connection_id: &str,
        recipient_id: String,
        recipient_label: String,
    ) -> Result<Connection, ConnectionError>;

    /// Lists all connections for a given profile
    async fn list_connections(&self, 
        profile_id: &str
    ) -> Result<Vec<Connection>, ConnectionError>;

    /// Gets a specific connection by ID
    async fn get_connection(&self, 
        connection_id: &str
    ) -> Result<Connection, ConnectionError>;

    /// Lists all connections in the system
    async fn list_all_connections(&self) -> Result<Vec<Connection>, ConnectionError>;

    /// Deletes a connection by ID
    async fn delete_connection(&self, 
        connection_id: &str
    ) -> Result<(), ConnectionError>;

    /// Gets a specific connection request by ID
    async fn get_request(&self, 
        connection_id: &str
    ) -> Result<ConnectionRequest, ConnectionError>;
}