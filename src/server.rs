use actix_web::{web, App, HttpServer, HttpResponse};
use actix::{Actor, StreamHandler};
use actix_web_actors::ws;
use std::collections::HashMap;
use std::env;
use actix_files as fs;
use std::net::TcpListener;
use std::sync::RwLock;
use serde_json::json;
use actix_cors::Cors;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use std::time::Duration;

use crate::connection::{Connection, ConnectionStatus, Message};
use crate::websocket::{RedpandaConfig, ws_route, setup_notification_consumer};
use std::time::SystemTime;

#[derive(serde::Deserialize)]
struct SendMessageRequest {
    player_id: String,
    content: String,
}

#[derive(serde::Deserialize)]
struct JoinRequest {
    player_id: String,
}

async fn join_connection(
    path: web::Path<String>,
    join_req: web::Json<JoinRequest>,
) -> HttpResponse {
    // For now just return the error since we don't have storage yet
    HttpResponse::BadRequest().json(json!({
        "error": "Player already in connection"
    }))
}

#[derive(Clone)]
pub struct Server {
    pub address: String, 
    connections: web::Data<RwLock<HashMap<String, Connection>>>,
    notifications: web::Data<RwLock<HashMap<String, Vec<String>>>>, 
    redpanda_config: web::Data<RedpandaConfig>,
    producer: Option<web::Data<FutureProducer>>,
}

// Helper function to send messages to Redpanda
fn send_to_redpanda(
    producer: &FutureProducer,
    topic: &str,
    key: &str,
    payload: &str,
) {
    let producer = producer.clone();
    let topic = topic.to_owned();
    let key = key.to_owned();
    let payload = payload.to_owned();

    actix_web::rt::spawn(async move {
        let record = FutureRecord::to(&topic)
            .key(&key)
            .payload(&payload);

        match producer.send(record, Duration::from_secs(1)).await {
            Ok(_) => (),
            Err((err, _)) => eprintln!("Error sending to Redpanda: {:?}", err),
        }
    });
}

impl Server {
    pub fn new(address: &str) -> Self {
        // Load Redpanda configuration from environment
        let bootstrap_servers = env::var("REDPANDA_BOOTSTRAP_SERVERS")
            .unwrap_or_else(|_| "localhost:9092".to_string());
        let username = env::var("REDPANDA_USERNAME")
            .unwrap_or_else(|_| "".to_string());
        let password = env::var("REDPANDA_PASSWORD")
            .unwrap_or_else(|_| "".to_string());
            
        let redpanda_config = RedpandaConfig {
            bootstrap_servers: bootstrap_servers.clone(),
            username: username.clone(),
            password: password.clone(),
        };
        
        // Create Redpanda producer
        let producer = if !bootstrap_servers.is_empty() {
            let mut config = ClientConfig::new();
            config.set("bootstrap.servers", &bootstrap_servers);
            
            if !username.is_empty() && !password.is_empty() {
                config.set("sasl.mechanism", "SCRAM-SHA-256");
                config.set("security.protocol", "SASL_SSL");
                config.set("sasl.username", &username);
                config.set("sasl.password", &password);
            }
            
            match config.create() {
                Ok(producer) => Some(web::Data::new(producer)),
                Err(err) => {
                    eprintln!("Failed to create Redpanda producer: {:?}", err);
                    None
                }
            }
        } else {
            None
        };
        
        Server {
            address: address.to_string(),
            connections: web::Data::new(RwLock::new(HashMap::new())),
            notifications: web::Data::new(RwLock::new(HashMap::new())),
            redpanda_config: web::Data::new(redpanda_config),
            producer,
        }
    }

    pub async fn run(&self) -> std::io::Result<()> {
        let address = self.address.clone(); 
        let connections = self.connections.clone();
        let notifications = self.notifications.clone();
        let redpanda_config = self.redpanda_config.clone();
        let producer = self.producer.clone();
        
        setup_notification_consumer(
            redpanda_config.get_ref().clone(),
            notifications.clone(),
        ).await;

        HttpServer::new(move || {
            let cors = Cors::permissive(); 
            let mut app = App::new()
                .wrap(cors)
                .app_data(connections.clone())
                .app_data(notifications.clone())
                .app_data(redpanda_config.clone());
                
            // Add producer if available
            if let Some(prod) = producer.clone() {
                app = app.app_data(prod.clone());
            }
                
            app.route("/connections", web::post().to(create_connection))
                .route("/connections/{id}/join", web::post().to(join_connection))
                .route(
                    "/connections/link/{link_id}/join", 
                    web::post().to(|link_id, req, connections, notifications, producer| {
                        join_connection_by_link(link_id, req, connections, notifications, producer)
                    })
                )
                .route("/players/{player_id}/notifications", web::get().to(get_player_notifications))        
                .route("/players/{player_id}/notifications/ack", web::post().to(acknowledge_notifications))
                .route("/connections/{id}/messages", web::post().to(send_message))
                .route("/ws", web::get().to(ws_route))
                .service(fs::Files::new("/", "./static")
                .index_file("index.html"))
        })
        .bind(address)?
        .run()
        .await
    }
}

async fn create_connection(
    player_id: web::Json<serde_json::Value>,
    connections: web::Data<RwLock<HashMap<String, Connection>>>,
    producer: Option<web::Data<FutureProducer>>,
) -> HttpResponse {
    let player_id = player_id.get("player_id")
        .and_then(|id| id.as_str())
        .unwrap_or("")
        .to_string();
        
    let connection = Connection::new(player_id.clone());
    
    // Store both id and link_id mappings
    let mut conn_map = connections.write().unwrap();
    conn_map.insert(connection.id.clone(), connection.clone());
    conn_map.insert(connection.link_id.clone(), connection.clone());
    
    // Publish to Redpanda if producer is available
    if let Some(producer) = producer {
        let event = json!({
            "event": "connection_created",
            "connection_id": connection.id,
            "player_id": player_id,
            "timestamp": SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });
        
        send_to_redpanda(
            producer.get_ref(),
            "connection-events",
            &connection.id,
            &event.to_string(),
        );
    }
    
    HttpResponse::Ok().json(connection)
}

async fn join_connection_by_link(
    link_id: web::Path<String>,
    join_req: web::Json<JoinRequest>,
    connections: web::Data<RwLock<HashMap<String, Connection>>>,
    notifications: web::Data<RwLock<HashMap<String, Vec<String>>>>,
    producer: Option<web::Data<FutureProducer>>,
) -> HttpResponse {
    let link_id = link_id.into_inner();
    
    // First get the connection and validate
    let connection = {
        let conn_map = connections.read().unwrap();
        if let Some(conn) = conn_map.get(&link_id) {
            if conn.players.len() != 1 {
                return HttpResponse::BadRequest().json(json!({
                    "error": "Connection already has maximum players"
                }));
            }
            
            if conn.players.contains(&join_req.player_id) {
                return HttpResponse::BadRequest().json(json!({
                    "error": "Player already in connection"
                }));
            }
            conn.clone()
        } else {
            return HttpResponse::NotFound().json(json!({
                "error": "Connection not found"
            }));
        }
    };
    
    // Store notification for first player
    {
        let first_player = &connection.players[0];
        let mut notifications = notifications.write().unwrap();
        notifications
            .entry(first_player.clone())
            .or_insert_with(Vec::new)
            .push(format!("Player {} joined your connection", join_req.player_id));
    }
    
    // Update connection with new player
    let mut updated_connection = connection.clone();
    updated_connection.players.push(join_req.player_id.clone());
    
    // Update both mappings
    {
        let mut conn_map = connections.write().unwrap();
        conn_map.insert(connection.id.clone(), updated_connection.clone());
        conn_map.insert(link_id, updated_connection.clone());
    }
    
    // Publish to Redpanda if producer is available
    if let Some(producer) = producer {
        let event = json!({
            "event": "player_joined",
            "connection_id": connection.id,
            "player_id": join_req.player_id,
            "timestamp": SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });
        
        send_to_redpanda(
            producer.get_ref(),
            "connection-events",
            &connection.id,
            &event.to_string(),
        );
    }
    
    HttpResponse::Ok().json(updated_connection)
}

async fn get_player_notifications(
    player_id: web::Path<String>,
    notifications: web::Data<RwLock<HashMap<String, Vec<String>>>>
) -> HttpResponse {
    let player_id = player_id.into_inner();
    let notifications = notifications.read().unwrap();
    if let Some(player_notifications) = notifications.get(&player_id) {
        HttpResponse::Ok().json(player_notifications)
    } else {
        HttpResponse::Ok().json(Vec::<String>::new())  // Return empty array instead of 404
    }
}

async fn acknowledge_notifications(
    player_id: web::Path<String>,
    notifications: web::Data<RwLock<HashMap<String, Vec<String>>>>,
    producer: Option<web::Data<FutureProducer>>,
) -> HttpResponse {
    let player_id = player_id.into_inner();
    let mut notifications = notifications.write().unwrap();
    
    // Check if there were notifications before removing
    let had_notifications = notifications.get(&player_id).map_or(false, |n| !n.is_empty());
    
    notifications.remove(&player_id);
    
    // Publish to Redpanda if producer is available and there were notifications
    if let Some(producer) = producer {
        if had_notifications {
            let event = json!({
                "event": "notifications_acknowledged",
                "player_id": player_id,
                "timestamp": SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            });
            
            send_to_redpanda(
                producer.get_ref(),
                "connection-events",
                &player_id,
                &event.to_string(),
            );
        }
    }
    
    HttpResponse::Ok().json(json!({"status": "ok"}))
}

async fn send_message(
    connection_id: web::Path<String>,
    message_req: web::Json<SendMessageRequest>,
    connections: web::Data<RwLock<HashMap<String, Connection>>>,
    notifications: web::Data<RwLock<HashMap<String, Vec<String>>>>,
    producer: Option<web::Data<FutureProducer>>,
) -> HttpResponse {
    let conn_map = connections.read().unwrap();
    let connection_id = connection_id.into_inner();
    
    if let Some(connection) = conn_map.get(&connection_id) {
        // Verify sender is in the connection
        if !connection.players.contains(&message_req.player_id) {
            return HttpResponse::BadRequest().json(json!({
                "error": "Player not in this connection"
            }));
        }
        
        // Create the message
        let message = Message {
            id: uuid::Uuid::new_v4().to_string(),
            from: message_req.player_id.clone(),
            content: message_req.content.clone(),
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        };
        
        // Notify other players
        let mut notifications = notifications.write().unwrap();
        for player in &connection.players {
            if player != &message_req.player_id {
                notifications
                    .entry(player.clone())
                    .or_insert_with(Vec::new)
                    .push(format!("Message from {}: {}", message_req.player_id, message_req.content));
            }
        }
        
        // Publish to Redpanda if producer is available
        if let Some(producer) = producer {
            let event = json!({
                "event": "message_sent",
                "connection_id": connection_id,
                "message_id": message.id,
                "player_id": message_req.player_id,
                "content": message_req.content,
                "timestamp": message.timestamp,
            });
            
            send_to_redpanda(
                producer.get_ref(),
                "connection-messages",
                &connection_id,
                &event.to_string(),
            );
        }
        
        HttpResponse::Ok().json(message)
    } else {
        HttpResponse::NotFound().json(json!({
            "error": "Connection not found"
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
        
    fn spawn_app() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let address = format!("127.0.0.1:{}", port);
        
        let server = Server::new(&address);
        let server_address = address.clone();
        
        actix_web::rt::spawn(async move {
            server.run().await.unwrap();
        });
        
        server_address
    }

    #[actix_web::test]
    async fn test_second_player_join_notifies_first_player() {
        // Arrange
        let address = spawn_app();
        let client = reqwest::Client::new();
        
        // Create connection with player1
        let create_resp = client
            .post(&format!("http://{}/connections", address))
            .json(&json!({
                "player_id": "player1"
            }))
            .send()
            .await
            .unwrap();
        
        let connection: Connection = create_resp.json().await.unwrap();
        
        // We need a way to check for notifications
        // Let's have player1 poll an endpoint
        // Check that player1 has no notifications initially
        let initial_notifications: Vec<String> = client
        .get(&format!("http://{}/players/player1/notifications", address))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

        assert!(initial_notifications.is_empty()); // No notifications yet
        
        // Act - Join with player2
        let join_resp = client
            .post(&format!("http://{}/connections/link/{}/join", address, connection.link_id))
            .json(&json!({
                "player_id": "player2"
            }))
            .send()
            .await
            .unwrap();
        
        // Assert - Check that player1 has a notification
        let notifications_resp = client
            .get(&format!("http://{}/players/player1/notifications", address))
            .send()
            .await
            .unwrap();
            
        assert_eq!(notifications_resp.status(), 200);
        let notifications: Vec<String> = notifications_resp.json().await.unwrap();
        assert_eq!(notifications.len(), 1);
        assert!(notifications[0].contains("player2")); // Notification mentions player2
    }

    #[actix_web::test]
    async fn test_join_connection_with_link_id_succeeds() {
        // Arrange
        let address = spawn_app();
        let client = reqwest::Client::new();
        
        // First create a connection
        let create_resp = client
            .post(&format!("http://{}/connections", address))
            .json(&json!({
                "player_id": "player1"
            }))
            .send()
            .await
            .unwrap();
        
        let initial_connection: Connection = create_resp.json().await.unwrap();
        
        // Act - Join with second player using link_id
        let join_resp = client
            .post(&format!("http://{}/connections/link/{}/join", address, initial_connection.link_id))
            .json(&json!({
                "player_id": "player2"
            }))
            .send()
            .await
            .unwrap();
            
        // Assert
        assert_eq!(join_resp.status(), 200);
        let updated_connection: Connection = join_resp.json().await.unwrap();
        assert_eq!(updated_connection.players.len(), 2);
        assert!(updated_connection.players.contains(&"player1".to_string()));
        assert!(updated_connection.players.contains(&"player2".to_string()));
    }

    // In server.rs tests module
    #[actix_web::test]
    async fn test_join_connection_validates_players() {
        // Arrange
        let address = spawn_app();
        let client = reqwest::Client::new();
        
        // First create a connection
        let create_resp = client
            .post(&format!("http://{}/connections", address))
            .json(&json!({
                "player_id": "player1"
            }))
            .send()
            .await
            .unwrap();
        
        let connection: Connection = create_resp.json().await.unwrap();
        
        // Act - Try to join with invalid player
        let join_resp = client
            .post(&format!("http://{}/connections/{}/join", address, connection.id))
            .json(&json!({
                "player_id": "player1" // Same player trying to join
            }))
            .send()
            .await
            .unwrap();
            
        // Assert
        assert_eq!(join_resp.status(), 400);
        let error = join_resp.json::<serde_json::Value>().await.unwrap();
        assert_eq!(error["error"], "Player already in connection");
    }

    #[actix_web::test]
    async fn test_create_connection_returns_success() {
        // Arrange
        let address = spawn_app();
        let client = reqwest::Client::new();
        
        // Act
        let response = client
            .post(&format!("http://{}/connections", address))
            .json(&json!({
                "player_id": "player123"
            }))
            .send()
            .await
            .unwrap();

        // Assert
        assert_eq!(response.status(), 200);
        let connection: Connection = response.json().await.unwrap();
        assert_eq!(connection.players[0], "player123");
    }

    #[actix_web::test]
    async fn test_index_serves_html_file() {
        // Arrange
        let address = spawn_app();
        
        // Give the server a moment to start
        actix_web::rt::time::sleep(Duration::from_millis(100)).await;

        // Act
        let client = reqwest::Client::new();
        let response = client
            .get(&format!("http://{}", address))
            .send()
            .await
            .unwrap();

        // Assert
        assert_eq!(response.status(), 200);
        let body = response.text().await.unwrap();
        assert!(body.contains("Hello World!"));
    }

    #[actix_web::test]
    async fn test_send_message_in_connection() {
        // Arrange
        let address = spawn_app();
        let client = reqwest::Client::new();
        
        // Create connection with player1
        let create_resp = client
            .post(&format!("http://{}/connections", address))
            .json(&json!({
                "player_id": "player1"
            }))
            .send()
            .await
            .unwrap();
        
        let connection: Connection = create_resp.json().await.unwrap();
        
        // Join with player2
        let _join_resp = client
            .post(&format!("http://{}/connections/link/{}/join", address, connection.link_id))
            .json(&json!({
                "player_id": "player2"
            }))
            .send()
            .await
            .unwrap();

        // Act - Send message from player1
        let message_resp = client
            .post(&format!("http://{}/connections/{}/messages", address, connection.id))
            .json(&json!({
                "player_id": "player1",
                "content": "Hello player2!"
            }))
            .send()
            .await
            .unwrap();
            
        // Assert
        assert_eq!(message_resp.status(), 200);
        
        // Check that player2 got the message in their notifications
        let notifications_resp = client
            .get(&format!("http://{}/players/player2/notifications", address))
            .send()
            .await
            .unwrap();
            
        let notifications: Vec<String> = notifications_resp.json().await.unwrap();
        assert!(notifications.iter().any(|n| n.contains("Hello player2!")));
    }

    #[test]
    fn test_server_new() {
        let server = Server::new("127.0.0.1:8080");
        assert_eq!(server.address, "127.0.0.1:8080");
    }

    #[actix_web::test]
    async fn test_notifications_are_cleared_after_acknowledgment() {
        // Arrange
        let address = spawn_app();
        let client = reqwest::Client::new();
        
        // Create connection with player1
        let create_resp = client
            .post(&format!("http://{}/connections", address))
            .json(&json!({
                "player_id": "player1"
            }))
            .send()
            .await
            .unwrap();
        
        let connection: Connection = create_resp.json().await.unwrap();
        
        // Join with player2 and send a message to generate notifications
        client
            .post(&format!("http://{}/connections/link/{}/join", address, connection.link_id))
            .json(&json!({
                "player_id": "player2"
            }))
            .send()
            .await
            .unwrap();
    
        client
            .post(&format!("http://{}/connections/{}/messages", address, connection.id))
            .json(&json!({
                "player_id": "player1",
                "content": "Hello player2!"
            }))
            .send()
            .await
            .unwrap();
            
        // Verify initial notifications exist
        let initial_notifications: Vec<String> = client
            .get(&format!("http://{}/players/player2/notifications", address))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        
        assert!(!initial_notifications.is_empty());
        
        // Acknowledge notifications
        let ack_resp = client
            .post(&format!("http://{}/players/player2/notifications/ack", address))
            .send()
            .await
            .unwrap();
            
        assert_eq!(ack_resp.status(), 200);
        let ack_json: serde_json::Value = ack_resp.json().await.unwrap();
        assert_eq!(ack_json["status"], "ok");
        
        // Verify notifications are cleared
        let final_notifications: Vec<String> = client
            .get(&format!("http://{}/players/player2/notifications", address))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
            
        assert!(final_notifications.is_empty());
    }
}