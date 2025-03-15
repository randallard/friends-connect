use actix::{Actor, StreamHandler, AsyncContext, ActorContext};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::RwLock;
use crate::connection::{Connection, ConnectionStatus};

// WebSocket message types
#[derive(Serialize, Deserialize)]
struct WsMessage {
    event_type: String,
    payload: serde_json::Value,
}

#[derive(actix::Message)]
#[rtype(result = "()")]
struct WebSocketMessage(String);

// Handler for WebSocketMessage
impl actix::Handler<WebSocketMessage> for WebSocketConnection {
    type Result = ();

    fn handle(&mut self, msg: WebSocketMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

struct WebSocketConnection {
    id: String,
    player_id: String,
    // Replace single connection_id with a set of connection_ids
    connection_ids: std::collections::HashSet<String>,
    heartbeat: Instant,
    producer: FutureProducer,
}

impl WebSocketConnection {
    pub fn new(player_id: String, redpanda_config: RedpandaConfig) -> Self {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &redpanda_config.bootstrap_servers)
            .set("sasl.mechanism", "SCRAM-SHA-256")
            .set("security.protocol", "SASL_SSL")
            .set("sasl.username", &redpanda_config.username)
            .set("sasl.password", &redpanda_config.password)
            .create()
            .expect("Producer creation error");

        Self {
            id: Uuid::new_v4().to_string(),
            player_id,
            connection_ids: std::collections::HashSet::new(),
            heartbeat: Instant::now(),
            producer,
        }
    }

    fn subscribe_to_connection(&mut self, connection_id: &str) -> bool {
        let is_new = self.connection_ids.insert(connection_id.to_string());
        
        if is_new {
            println!("Player {} subscribed to connection {}", self.player_id, connection_id);
            
            // Send subscription event to Redpanda
            let subscription_event = serde_json::json!({
                "event": "player_subscribed",
                "player_id": self.player_id,
                "connection_id": connection_id,
                "timestamp": chrono::Utc::now().timestamp(),
            });
            
            self.send_to_redpanda(
                "connection-events",
                connection_id,
                &subscription_event.to_string(),
            );
        }
        
        is_new
    }

    fn is_subscribed_to(&self, connection_id: &str) -> bool {
        self.connection_ids.contains(connection_id)
    }

    fn send_to_redpanda(&self, topic: &str, key: &str, payload: &str) {
        let producer = self.producer.clone();
        let topic = topic.to_owned();
        let key = key.to_owned();
        let payload = payload.to_owned();

        actix_web::rt::spawn(async move {
            let record = FutureRecord::to(&topic)
                .key(&key)
                .payload(&payload);

            match producer.send(record, Duration::from_secs(10)).await {
                Ok(_) => (),
                Err((err, _)) => eprintln!("Error sending to Redpanda: {:?}", err),
            }
        });
    }
}

impl Actor for WebSocketConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // Start heartbeat process
        self.heartbeat(ctx);

        register_websocket(&self.player_id, ctx.address());

        println!("WebSocket actor started for player: {} with id: {}", self.player_id, self.id);

        // Announce user connection
        let connection_event = serde_json::json!({
            "event": "user_connected",
            "player_id": self.player_id,
            "timestamp": chrono::Utc::now().timestamp(),
        });

        self.send_to_redpanda(
            "connection-events",
            &self.player_id,
            &connection_event.to_string(),
        );
    }

    fn stopped(&mut self, _: &mut Self::Context) {

        unregister_websocket(&self.player_id);

        // Announce user disconnection
        let connection_event = serde_json::json!({
            "event": "user_disconnected",
            "player_id": self.player_id,
            "timestamp": chrono::Utc::now().timestamp(),
        });

        self.send_to_redpanda(
            "connection-events",
            &self.player_id,
            &connection_event.to_string(),
        );
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Pong(_)) => {
                self.heartbeat = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                // Try to parse the message
                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                    match ws_msg.event_type.as_str() {
                        "heartbeat" => {
                            // Simply update the heartbeat time and respond with a pong
                            self.heartbeat = Instant::now();
                            
                            // Optionally respond with a heartbeat acknowledgment
                            let heartbeat_ack = serde_json::json!({
                                "event_type": "heartbeat_ack",
                                "payload": {
                                    "timestamp": chrono::Utc::now().timestamp(),
                                }
                            });
                            
                            ctx.text(heartbeat_ack.to_string());
                        },
                        "join_connection" => {
                            if let Some(conn_id) = ws_msg.payload.get("connection_id").and_then(|id| id.as_str()) {
                                println!("WebSocket: Player {} subscribing to connection {}", self.player_id, conn_id);
                                
                                // Subscribe to this connection
                                let is_new = self.subscribe_to_connection(conn_id);
                                
                                // Send acknowledgment back to client
                                let ack_msg = serde_json::json!({
                                    "event_type": "join_connection_ack",
                                    "payload": {
                                        "connection_id": conn_id,
                                        "status": "subscribed",
                                        "timestamp": chrono::Utc::now().timestamp(),
                                    }
                                });
                                
                                ctx.text(ack_msg.to_string());
                                
                                // If this is a new subscription, send a join event to Redpanda
                                if is_new {
                                    let join_event = serde_json::json!({
                                        "event": "join_connection",
                                        "player_id": self.player_id,
                                        "connection_id": conn_id,
                                        "timestamp": chrono::Utc::now().timestamp(),
                                    });
                                    
                                    self.send_to_redpanda(
                                        "connection-events", 
                                        conn_id,
                                        &join_event.to_string()
                                    );
                                }
                            }
                        },
                        "send_message" => {
                            if let Some(content) = ws_msg.payload.get("content").and_then(|c| c.as_str()) {
                                if let Some(conn_id) = ws_msg.payload.get("connection_id").and_then(|id| id.as_str()) {
                                    // Check if subscribed to this connection
                                    if self.is_subscribed_to(conn_id) {
                                        let message_event = serde_json::json!({
                                            "event": "new_message",
                                            "connection_id": conn_id,
                                            "player_id": self.player_id,
                                            "content": content,
                                            "timestamp": chrono::Utc::now().timestamp(),
                                        });
                                        
                                        self.send_to_redpanda(
                                            "connection-messages",
                                            conn_id,
                                            &message_event.to_string(),
                                        );
                                    } else {
                                        // Not subscribed to this connection
                                        let error_msg = serde_json::json!({
                                            "event_type": "error",
                                            "payload": {
                                                "error": "Not subscribed to this connection",
                                                "connection_id": conn_id,
                                                "timestamp": chrono::Utc::now().timestamp(),
                                            }
                                        });
                                        
                                        ctx.text(error_msg.to_string());
                                    }
                                }
                            }
                        },
                        "update_status" => {
                            if let Some(new_status) = ws_msg.payload.get("status").and_then(|s| s.as_str()) {
                                if let Some(conn_id) = ws_msg.payload.get("connection_id").and_then(|id| id.as_str()) {
                                    // Check if subscribed to this connection
                                    if self.is_subscribed_to(conn_id) {
                                        // Validate the status
                                        let status = match new_status {
                                            "Active" => Some("Active"),
                                            "Expired" => Some("Expired"),
                                            "Pending" => Some("Pending"),
                                            _ => None
                                        };
                                        
                                        if let Some(status) = status {
                                            // Send status update event to Redpanda
                                            let status_event = serde_json::json!({
                                                "event": "status_updated",
                                                "connection_id": conn_id,
                                                "player_id": self.player_id,
                                                "new_status": status,
                                                "timestamp": chrono::Utc::now().timestamp(),
                                            });
                                            
                                            self.send_to_redpanda(
                                                "connection-events", 
                                                conn_id,
                                                &status_event.to_string()
                                            );
                                        }
                                    } else {
                                        // Not subscribed to this connection
                                        let error_msg = serde_json::json!({
                                            "event_type": "error",
                                            "payload": {
                                                "error": "Not subscribed to this connection",
                                                "connection_id": conn_id,
                                                "timestamp": chrono::Utc::now().timestamp(),
                                            }
                                        });
                                        
                                        ctx.text(error_msg.to_string());
                                    }
                                }
                            }
                        },
                        "connection_status_update" => {
                            if let Some(conn_id) = ws_msg.payload.get("connection_id").and_then(|id| id.as_str()) {
                                if let Some(status) = ws_msg.payload.get("status").and_then(|s| s.as_str()) {
                                    // Check if subscribed to this connection
                                    if self.is_subscribed_to(conn_id) {
                                        println!("WebSocket: Status update for connection {}: {} by player {}", 
                                            conn_id, status, self.player_id);
                                        
                                        // Send status update event to Redpanda
                                        let status_event = serde_json::json!({
                                            "event": "status_changed",
                                            "connection_id": conn_id,
                                            "player_id": self.player_id,
                                            "new_status": status,
                                            "timestamp": chrono::Utc::now().timestamp(),
                                        });
                                        
                                        self.send_to_redpanda(
                                            "connection-events", 
                                            conn_id,
                                            &status_event.to_string()
                                        );
                                        
                                        // Also send a direct status update to the client
                                        let client_update = serde_json::json!({
                                            "event_type": "connection_status_updated",
                                            "payload": {
                                                "connection_id": conn_id,
                                                "status": status,
                                                "timestamp": chrono::Utc::now().timestamp(),
                                            }
                                        });
                                        
                                        ctx.text(client_update.to_string());
                                    } else {
                                        // Not subscribed to this connection
                                        let error_msg = serde_json::json!({
                                            "event_type": "error",
                                            "payload": {
                                                "error": "Not subscribed to this connection",
                                                "connection_id": conn_id,
                                                "timestamp": chrono::Utc::now().timestamp(),
                                            }
                                        });
                                        
                                        ctx.text(error_msg.to_string());
                                    }
                                }
                            }
                        },
                        "ping" => {
                            // Also handle ping as an alternative to heartbeat
                            self.heartbeat = Instant::now();
                            
                            // Respond with a pong
                            let pong = serde_json::json!({
                                "event_type": "pong",
                                "payload": {
                                    "timestamp": chrono::Utc::now().timestamp(),
                                }
                            });
                            
                            ctx.text(pong.to_string());
                        },
                        "get_connections" => {
                            // Send back list of connections this player is subscribed to
                            let connections_list = self.connection_ids.iter()
                                .cloned()
                                .collect::<Vec<String>>();
                                
                            let connections_response = serde_json::json!({
                                "event_type": "connections_list",
                                "payload": {
                                    "connections": connections_list,
                                    "timestamp": chrono::Utc::now().timestamp(),
                                }
                            });
                            
                            ctx.text(connections_response.to_string());
                        },
                        _ => {
                            eprintln!("Unknown event type: {}", ws_msg.event_type);
                            
                            // Send error response
                            let error_msg = serde_json::json!({
                                "event_type": "error",
                                "payload": {
                                    "error": format!("Unknown event type: {}", ws_msg.event_type),
                                    "timestamp": chrono::Utc::now().timestamp(),
                                }
                            });
                            
                            ctx.text(error_msg.to_string());
                        }
                    }
                } else {
                    // Failed to parse message
                    let error_msg = serde_json::json!({
                        "event_type": "error",
                        "payload": {
                            "error": "Invalid message format",
                            "timestamp": chrono::Utc::now().timestamp(),
                        }
                    });
                    
                    ctx.text(error_msg.to_string());
                }
            }
            Ok(ws::Message::Binary(_)) => println!("Binary message received"),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

impl WebSocketConnection {
    fn heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(Duration::from_secs(30), |act, ctx| {
            if Instant::now().duration_since(act.heartbeat) > Duration::from_secs(60) {
                println!("WebSocket heartbeat failed, disconnecting!");
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }
    
    fn send_status_update(&self, ctx: &mut ws::WebsocketContext<Self>, status: &str) {
        if let Some(conn_id) = &self.connection_id {
            let status_msg = serde_json::json!({
                "event_type": "status_update",
                "payload": {
                    "connection_id": conn_id,
                    "status": status,
                    "timestamp": chrono::Utc::now().timestamp(),
                }
            });
            
            ctx.text(status_msg.to_string());
        }
    }
}

#[derive(Clone)]
pub struct RedpandaConfig {
    pub bootstrap_servers: String,
    pub username: String,
    pub password: String,
}

lazy_static::lazy_static! {
    static ref WS_CONNECTIONS: std::sync::RwLock<std::collections::HashMap<String, actix::Addr<WebSocketConnection>>> = 
        std::sync::RwLock::new(std::collections::HashMap::new());
}

fn register_websocket(player_id: &str, addr: actix::Addr<WebSocketConnection>) {
    let mut connections = WS_CONNECTIONS.write().unwrap();
    connections.insert(player_id.to_string(), addr);
    println!("Registered WebSocket for player: {}", player_id);
    println!("Total active WebSockets: {}", connections.len());
}

fn unregister_websocket(player_id: &str) {
    let mut connections = WS_CONNECTIONS.write().unwrap();
    connections.remove(player_id);
    println!("Unregistered WebSocket for player: {}", player_id);
    println!("Total active WebSockets: {}", connections.len());
}

fn get_websocket_for_player(player_id: &str) -> Option<actix::Addr<WebSocketConnection>> {
    let connections = WS_CONNECTIONS.read().unwrap();
    connections.get(player_id).cloned()
}

// In websocket.rs - modify ws_route function to auto-join connections

pub async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    query: web::Query<HashMap<String, String>>,
    redpanda_config: web::Data<RedpandaConfig>,
    connections: web::Data<RwLock<HashMap<String, Connection>>>,
) -> Result<HttpResponse, Error> {
    // Extract player_id from query params
    let player_id = query.get("player_id").cloned().unwrap_or_else(|| {
        Uuid::new_v4().to_string() // Generate a temp ID if none provided
    });
    
    println!("WebSocket connection established for player: {}", player_id);
    
    // Create the WebSocket connection
    let mut ws = WebSocketConnection::new(player_id.clone(), redpanda_config.get_ref().clone());
    
    // Check if player is in any connections and auto-subscribe
    let conn_map = connections.read().unwrap();
    let mut player_connections = Vec::new();
    
    for (conn_id, conn) in conn_map.iter() {
        if conn.players.contains(&player_id) {
            // This player is in this connection, add to list to auto-subscribe
            player_connections.push(conn_id.clone());
        }
    }
    
    // Drop the lock before we start the actor to avoid deadlock
    drop(conn_map);
    
    // Start the actor
    let ws_actor = ws::start(ws, &req, stream)?;
    
    // Auto-subscribe to connections after a small delay 
    // to ensure WebSocket is fully established
    let player_id_clone = player_id.clone();
    actix_web::rt::spawn(async move {
        // Wait briefly to ensure the WebSocket connection is established
        actix_web::rt::time::sleep(Duration::from_millis(500)).await;
        
        if let Some(actor_addr) = get_websocket_for_player(&player_id_clone) {
            for conn_id in player_connections {
                // Create a join message
                let join_msg = serde_json::json!({
                    "event_type": "join_connection",
                    "payload": {
                        "connection_id": conn_id,
                        "auto": true
                    }
                });
                
                // Send message to actor
                actor_addr.do_send(WebSocketMessage(join_msg.to_string()));
                println!("Auto-subscribed player {} to connection {}", player_id_clone, conn_id);
            }
            
            // Send a welcome message
            let welcome_msg = serde_json::json!({
                "event_type": "welcome",
                "payload": {
                    "message": "WebSocket connection established",
                    "player_id": player_id_clone,
                    "timestamp": chrono::Utc::now().timestamp(),
                }
            });
            
            actor_addr.do_send(WebSocketMessage(welcome_msg.to_string()));
        }
    });
    
    Ok(ws_actor)
}

pub async fn setup_notification_consumer(
    redpanda_config: RedpandaConfig,
    notifications: web::Data<RwLock<HashMap<String, Vec<String>>>>,    
    connections: web::Data<RwLock<HashMap<String, Connection>>>, 
) {
    use rdkafka::config::ClientConfig;
    use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
    use rdkafka::message::Message;
    
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &redpanda_config.bootstrap_servers)
        .set("sasl.mechanism", "SCRAM-SHA-256")
        .set("security.protocol", "SASL_SSL")
        .set("sasl.username", &redpanda_config.username)
        .set("sasl.password", &redpanda_config.password)
        .set("group.id", "friends-connect-server")
        .set("auto.offset.reset", "earliest")
        .create()
        .expect("Consumer creation failed");
    
    consumer
        .subscribe(&["user-notifications","connection-events", "connection-messages"])
        .expect("Topic subscription failed");
    
    actix_web::rt::spawn(async move {
        loop {
            match consumer.recv().await {
                Ok(msg) => {
                    if let Some(payload) = msg.payload() {
                        if let Ok(payload_str) = std::str::from_utf8(payload) {
                            if let Ok(event) = serde_json::from_str::<serde_json::Value>(payload_str) {
                                // Check the topic
                                let topic_str = msg.topic();
                                match topic_str {
                                    "user-notifications" => {
                                        // Handle user notifications - forward to WebSocket if connected
                                        println!("Processing user notification: {:?}", event);
                                        
                                        if let Some(event_type) = event.get("event").and_then(|e| e.as_str()) {
                                            match event_type {
                                                "notification" => {
                                                    // Handle new notification
                                                    if let (Some(player_id), Some(message)) = (
                                                        event.get("player_id").and_then(|id| id.as_str()),
                                                        event.get("message").and_then(|m| m.as_str()),
                                                    ) {
                                                        // Add to in-memory notifications
                                                        let mut notif_lock = notifications.write().unwrap();
                                                        notif_lock
                                                            .entry(player_id.to_string())
                                                            .or_insert_with(Vec::new)
                                                            .push(message.to_string());
                                                            
                                                        println!("Added notification for player {}: {}", player_id, message);
                                                        
                                                        // Also forward to WebSocket if connected
                                                        if let Some(ws) = get_websocket_for_player(player_id) {
                                                            let ws_msg = serde_json::json!({
                                                                "event_type": "notification",
                                                                "payload": {
                                                                    "message": message,
                                                                    "timestamp": chrono::Utc::now().timestamp(),
                                                                }
                                                            });
                                                            
                                                            ws.do_send(WebSocketMessage(ws_msg.to_string()));
                                                            println!("Forwarded notification to player {} via WebSocket", player_id);
                                                        }
                                                    }
                                                },
                                                "notifications_cleared" => {
                                                    // Handle notification clearance
                                                    if let Some(player_id) = event.get("player_id").and_then(|id| id.as_str()) {
                                                        // Clear notifications for this player
                                                        let mut notif_lock = notifications.write().unwrap();
                                                        notif_lock.remove(player_id);
                                                        println!("Cleared notifications for player {}", player_id);
                                                    }
                                                },
                                                _ => {
                                                    println!("Unknown user-notifications event type: {}", event_type);
                                                }
                                            }
                                        }
                                    },
                                    "connection-events" => {
                                        // Process connection events and forward to relevant WebSockets
                                        if let Some(event_type) = event.get("event").and_then(|e| e.as_str()) {
                                            match event_type {
                                                "status_changed" => {
                                                    // Process status change event
                                                    println!("Processing status_changed event: {:?}", event);
                                                    
                                                    if let (Some(conn_id), Some(new_status)) = (
                                                        event.get("connection_id").and_then(|id| id.as_str()),
                                                        event.get("new_status").and_then(|s| s.as_str()),
                                                    ) {
                                                        // Update connection in our local state
                                                        let mut conn_map = connections.write().unwrap();
                                                        
                                                        // First get the current connection to get player IDs
                                                        let player_ids = if let Some(conn) = conn_map.get(conn_id) {
                                                            conn.players.clone()
                                                        } else {
                                                            Vec::new()
                                                        };
                                                        
                                                        // Update the connection status
                                                        if let Some(conn) = conn_map.get_mut(conn_id) {
                                                            let old_status = conn.status.clone();
                                                            
                                                            // Update the status
                                                            conn.status = match new_status {
                                                                "Active" => ConnectionStatus::Active,
                                                                "Expired" => ConnectionStatus::Expired,
                                                                "Pending" => ConnectionStatus::Pending,
                                                                _ => conn.status.clone(),
                                                            };
                                                            
                                                            println!("Updated connection {} status from {:?} to {:?}", 
                                                                conn_id, old_status, conn.status);
                                                        }
                                                        
                                                        // Forward status change to all players via WebSocket
                                                        for player_id in player_ids {
                                                            if let Some(ws) = get_websocket_for_player(&player_id) {
                                                                let ws_msg = serde_json::json!({
                                                                    "event_type": "connection_status_update",
                                                                    "payload": {
                                                                        "connection_id": conn_id,
                                                                        "status": new_status,
                                                                        "timestamp": chrono::Utc::now().timestamp(),
                                                                    }
                                                                });
                                                                
                                                                ws.do_send(WebSocketMessage(ws_msg.to_string()));
                                                                println!("Sent status update to player {} via WebSocket", player_id);
                                                            }
                                                            
                                                            // Also add notification
                                                            let mut notif_lock = notifications.write().unwrap();
                                                            notif_lock
                                                                .entry(player_id)
                                                                .or_insert_with(Vec::new)
                                                                .push(format!("Connection status changed to {}", new_status));
                                                        }
                                                    }
                                                },
                                                "player_joined" => {
                                                    // Handle player joining event
                                                    println!("Processing player_joined event: {:?}", event);
                                                    
                                                    if let (Some(conn_id), Some(joined_player_id)) = (
                                                        event.get("connection_id").and_then(|id| id.as_str()),
                                                        event.get("player_id").and_then(|id| id.as_str()),
                                                    ) {
                                                        // Get connection from our local state
                                                        let conn_map = connections.read().unwrap();
                                                        
                                                        if let Some(conn) = conn_map.get(conn_id) {
                                                            // Notify all players in the connection
                                                            for player_id in &conn.players {
                                                                if player_id != joined_player_id {
                                                                    // Skip the player who joined
                                                                    // Add notification
                                                                    let message = format!("Player {} joined your connection", joined_player_id);
                                                                    
                                                                    let mut notif_lock = notifications.write().unwrap();
                                                                    notif_lock
                                                                        .entry(player_id.clone())
                                                                        .or_insert_with(Vec::new)
                                                                        .push(message.clone());
                                                                        
                                                                    // Forward to WebSocket if connected
                                                                    if let Some(ws) = get_websocket_for_player(player_id) {
                                                                        let ws_msg = serde_json::json!({
                                                                            "event_type": "notification",
                                                                            "payload": {
                                                                                "message": message,
                                                                                "connection_id": conn_id,
                                                                                "timestamp": chrono::Utc::now().timestamp(),
                                                                            }
                                                                        });
                                                                        
                                                                        ws.do_send(WebSocketMessage(ws_msg.to_string()));
                                                                        println!("Sent join notification to player {} via WebSocket", player_id);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                },
                                                _ => {
                                                    // Handle other connection events
                                                }
                                            }
                                        }
                                    },
                                    "connection-messages" => {
                                        // Process new messages
                                        if let Some(event_type) = event.get("event").and_then(|e| e.as_str()) {
                                            if event_type == "message_sent" || event_type == "new_message" {
                                                if let (Some(conn_id), Some(sender_id), Some(content)) = (
                                                    event.get("connection_id").and_then(|id| id.as_str()),
                                                    event.get("player_id").and_then(|id| id.as_str()),
                                                    event.get("content").and_then(|c| c.as_str()),
                                                ) {
                                                    // Get connection details
                                                    let conn_map = connections.read().unwrap();
                                                    
                                                    if let Some(conn) = conn_map.get(conn_id) {
                                                        // Forward message to all players except sender
                                                        for player_id in &conn.players {
                                                            if player_id != sender_id {
                                                                // Forward message to WebSocket if connected
                                                                if let Some(ws) = get_websocket_for_player(player_id) {
                                                                    let ws_msg = serde_json::json!({
                                                                        "event_type": "new_message",
                                                                        "payload": {
                                                                            "connection_id": conn_id,
                                                                            "sender_id": sender_id,
                                                                            "content": content,
                                                                            "timestamp": event.get("timestamp").and_then(|t| t.as_i64())
                                                                                .unwrap_or_else(|| chrono::Utc::now().timestamp()),
                                                                        }
                                                                    });
                                                                    
                                                                    ws.do_send(WebSocketMessage(ws_msg.to_string()));
                                                                    println!("Forwarded message to player {} via WebSocket", player_id);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    },
                                    _ => {
                                        // Ignore other topics
                                    }
                                }
                            }
                        }
                    }
                    consumer.commit_message(&msg, CommitMode::Async).unwrap();
                }
                Err(e) => {
                    eprintln!("Error while receiving from Redpanda: {:?}", e);
                    actix_web::rt::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });
}