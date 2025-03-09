use actix::{Actor, StreamHandler};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use uuid::Uuid;

// WebSocket message types
#[derive(Serialize, Deserialize)]
struct WsMessage {
    event_type: String,
    payload: serde_json::Value,
}

// WebSocket connection actor
struct WebSocketConnection {
    id: String,
    player_id: String,
    connection_id: Option<String>,
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
            connection_id: None,
            heartbeat: Instant::now(),
            producer,
        }
    }

    // Send a message to Redpanda
    fn send_to_redpanda(&self, topic: &str, key: &str, payload: &str) {
        let producer = self.producer.clone();
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
}

impl Actor for WebSocketConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // Start heartbeat process
        self.heartbeat(ctx);

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

// Handler for WebSocket messages
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
                        "join_connection" => {
                            if let Some(conn_id) = ws_msg.payload.get("connection_id") {
                                if let Some(conn_id_str) = conn_id.as_str() {
                                    self.connection_id = Some(conn_id_str.to_owned());
                                    
                                    // Send join event to Redpanda
                                    let join_event = serde_json::json!({
                                        "event": "join_connection",
                                        "player_id": self.player_id,
                                        "connection_id": conn_id_str,
                                        "timestamp": chrono::Utc::now().timestamp(),
                                    });
                                    
                                    self.send_to_redpanda(
                                        "connection-events", 
                                        conn_id_str,
                                        &join_event.to_string()
                                    );
                                }
                            }
                        }
                        "send_message" => {
                            if let (Some(content), Some(conn_id)) = (
                                ws_msg.payload.get("content").and_then(|c| c.as_str()),
                                self.connection_id.as_ref(),
                            ) {
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
                            }
                        }
                        _ => {
                            eprintln!("Unknown event type: {}", ws_msg.event_type);
                        }
                    }
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
}

// Redpanda configuration
#[derive(Clone)]
struct RedpandaConfig {
    bootstrap_servers: String,
    username: String,
    password: String,
}

// WebSocket connection handler
async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    query: web::Query<HashMap<String, String>>,
    redpanda_config: web::Data<RedpandaConfig>,
) -> Result<HttpResponse, Error> {
    // Extract player_id from query params
    let player_id = query.get("player_id").cloned().unwrap_or_else(|| {
        Uuid::new_v4().to_string() // Generate a temp ID if none provided
    });
    
    // Create the WebSocket connection
    let ws = WebSocketConnection::new(player_id, redpanda_config.get_ref().clone());
    
    // Start the WebSocket connection
    ws::start(ws, &req, stream)
}

// Function to set up Redpanda consumer for notifications
async fn setup_notification_consumer(
    redpanda_config: RedpandaConfig,
    notifications: web::Data<RwLock<HashMap<String, Vec<String>>>>,
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
        .subscribe(&["user-notifications"])
        .expect("Topic subscription failed");
    
    actix_web::rt::spawn(async move {
        loop {
            match consumer.recv().await {
                Ok(msg) => {
                    if let Some(payload) = msg.payload() {
                        if let Ok(payload_str) = std::str::from_utf8(payload) {
                            if let Ok(notification) = serde_json::from_str::<serde_json::Value>(payload_str) {
                                if let (Some(player_id), Some(content)) = (
                                    notification.get("player_id").and_then(|id| id.as_str()),
                                    notification.get("content").and_then(|c| c.as_str()),
                                ) {
                                    let mut notifications_lock = notifications.write().unwrap();
                                    notifications_lock
                                        .entry(player_id.to_owned())
                                        .or_insert_with(Vec::new)
                                        .push(content.to_owned());
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