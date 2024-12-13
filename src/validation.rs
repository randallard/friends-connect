use chrono::Utc;
use crate::error::ConnectionError;
use crate::models::{Connection, ConnectionRequest, ConnectionStatus};

pub trait ConnectionValidator {
    fn validate_request(&self, request: &ConnectionRequest) -> Result<(), ConnectionError>;
    fn validate_connection(&self, connection: &Connection) -> Result<(), ConnectionError>;
}

pub struct DefaultConnectionValidator;

impl DefaultConnectionValidator {
    pub fn new() -> Self {
        DefaultConnectionValidator
    }
}

impl ConnectionValidator for DefaultConnectionValidator {
    fn validate_request(&self, request: &ConnectionRequest) -> Result<(), ConnectionError> {
        // Check expiration
        if request.expires_at < Utc::now() {
            return Err(ConnectionError::Expired);
        }

        // Ensure from_profile_id is not empty
        if request.from_profile_id.is_empty() {
            return Err(ConnectionError::InvalidRequest(
                "Initiator profile ID cannot be empty".to_string()
            ));
        }

        // If to_profile_id is set, ensure it's different from from_profile_id
        if let Some(to_id) = &request.to_profile_id {
            if to_id == &request.from_profile_id {
                return Err(ConnectionError::InvalidRequest(
                    "Cannot create connection with self".to_string()
                ));
            }
        }

        Ok(())
    }

    fn validate_connection(&self, connection: &Connection) -> Result<(), ConnectionError> {
        // Basic validation
        if connection.id.is_empty() {
            return Err(ConnectionError::InvalidRequest(
                "Connection ID cannot be empty".to_string()
            ));
        }

        if connection.initiator_label.is_empty() {
            return Err(ConnectionError::InvalidRequest(
                "Initiator label cannot be empty".to_string()
            ));
        }

        // Status-specific validation
        match connection.status {
            ConnectionStatus::Active => {
                // Active connections must have recipient info
                if connection.recipient_id.is_none() || connection.recipient_label.is_none() {
                    return Err(ConnectionError::InvalidRequest(
                        "Active connection must have recipient information".to_string()
                    ));
                }
            }
            ConnectionStatus::Rejected => {
                // Rejected connections should have a recipient ID
                if connection.recipient_id.is_none() {
                    return Err(ConnectionError::InvalidRequest(
                        "Rejected connection must have recipient ID".to_string()
                    ));
                }
            }
            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn create_test_request() -> ConnectionRequest {
        ConnectionRequest {
            connection_id: "test-id".to_string(),
            from_profile_id: "initiator-id".to_string(),
            to_profile_id: None,
            expires_at: Utc::now() + Duration::days(7),
        }
    }

    fn create_test_connection() -> Connection {
        Connection {
            id: "test-id".to_string(),
            initiator_id: "initiator-id".to_string(),
            recipient_id: None,
            initiator_label: "Test Label".to_string(),
            recipient_label: None,
            status: ConnectionStatus::Pending,
            created_at: Utc::now(),
            connected_at: None,
        }
    }

    #[test]
    fn test_validate_request() {
        let validator = DefaultConnectionValidator::new();
        
        // Test valid request
        let request = create_test_request();
        assert!(validator.validate_request(&request).is_ok());

        // Test expired request
        let mut expired_request = request.clone();
        expired_request.expires_at = Utc::now() - Duration::hours(1);
        assert!(matches!(
            validator.validate_request(&expired_request),
            Err(ConnectionError::Expired)
        ));
    }

    #[test]
    fn test_validate_connection() {
        let validator = DefaultConnectionValidator::new();
        
        // Test valid pending connection
        let connection = create_test_connection();
        assert!(validator.validate_connection(&connection).is_ok());

        // Test invalid active connection
        let mut active_connection = connection.clone();
        active_connection.status = ConnectionStatus::Active;
        assert!(matches!(
            validator.validate_connection(&active_connection),
            Err(ConnectionError::InvalidRequest(_))
        ));
    }
}