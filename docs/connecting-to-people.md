# Player Connection System

A flexible connection system that enables players to link profiles through invitation links or direct requests. Supports both immediate play for new users and account linking for existing players, maintaining the project's progressive engagement philosophy.

[Flowchart describing player connection](connecting-people-flow.mermaid)

## Overview

The connection system allows players to:
- Generate shareable invitation links
- Connect with existing players directly
- Maintain persistent connections across sessions
- Share game achievements and challenges
- Send and receive encouragement messages

## Connection Methods

### Invitation Links
Players can generate unique invitation links that:
- Include an embedded connection ID
- Work for both new and existing players
- Maintain connection persistence through browser changes
- Allow custom naming of connections on both sides

### Direct Connections
Existing players can also connect through the platform:
- Search by player name or ID
- Send connection requests
- Accept or decline incoming requests
- Manage connection lists

## Technical Implementation

### Data Structures

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    id: String,
    initiator_id: String,
    recipient_id: Option<String>,
    initiator_label: String,
    recipient_label: Option<String>,
    status: ConnectionStatus,
    created_at: DateTime<Utc>,
    connected_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Pending,
    Active,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionRequest {
    connection_id: String,
    from_profile_id: String,
    to_profile_id: Option<String>,
    expires_at: DateTime<Utc>,
}
```

### Connection Flow

1. **Creating Connections**
   - Generate unique connection ID
   - Create pending connection record
   - Generate shareable link
   - Store connection request

2. **Accepting Connections**
   - Validate connection ID
   - Create or load recipient profile
   - Update connection status
   - Store bidirectional connection

3. **Managing Connections**
   - List active connections
   - Update connection labels
   - Remove connections
   - Block unwanted connections

## Storage Considerations

### Local Storage
- Connection lists cached in browser
- Pending invitations stored locally
- Connection status synced when online

### Cloud Storage (Premium)
- Real-time connection status updates
- Cross-device synchronization
- Connection history preservation
- Enhanced privacy controls

## Security Measures

1. **Invitation Links**
   - Time-limited validity
   - Single-use connection IDs
   - Rate limiting on generation
   - Validation before activation

2. **Connection Privacy**
   - Mutual consent required
   - Blocking capabilities
   - Private connection labels
   - Controlled visibility options

## Integration Points

### Profile System
```rust
impl PlayerProfile {
    pub async fn create_connection(&self, label: String) -> Result<Connection, ConnectionError>;
    pub async fn accept_connection(&self, connection_id: &str, label: String) -> Result<Connection, ConnectionError>;
    pub async fn list_connections(&self) -> Result<Vec<Connection>, ConnectionError>;
}
```

### Game Integration
```rust
impl GameProgress {
    pub async fn share_achievement(&self, connection_id: &str) -> Result<(), ShareError>;
    pub async fn send_challenge(&self, connection_id: &str, challenge: Challenge) -> Result<(), ChallengeError>;
}
```

## Future Enhancements

1. **Social Features**
   - Achievement sharing
   - Challenge tracking
   - Leaderboards
   - Group connections

2. **Premium Features**
   - Enhanced privacy controls
   - Extended connection history
   - Group challenges
   - Custom achievement sharing

3. **Administration**
   - Connection analytics
   - Abuse prevention
   - System health monitoring
   - Privacy compliance tools

## Error Handling

```rust
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
    Storage(#[from] StorageError),
}
```

## Testing Strategy

1. **Unit Tests**
   - Connection creation
   - Status transitions
   - Error handling
   - Data validation

2. **Integration Tests**
   - Full connection flow
   - Storage integration
   - Profile system interaction
   - Game progress sharing

3. **Security Tests**
   - Link validation
   - Privacy controls
   - Rate limiting
   - Abuse prevention