use actix_web::{web, App, HttpServer, HttpResponse};
use actix_files as fs;
use std::net::TcpListener;
use std::sync::RwLock;
use std::collections::HashMap;
use serde_json::json;

use crate::connection::{Connection, ConnectionStatus};

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
}

impl Server {
    pub fn new(address: &str) -> Self {
        Server {
            address: address.to_string(),
            connections: web::Data::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn run(&self) -> std::io::Result<()> {
        let address = self.address.clone(); 
        let connections = self.connections.clone();

        HttpServer::new(move || {
            App::new()
                .app_data(connections.clone())
                .route("/connections", web::post().to(create_connection))
                .route("/connections/{id}/join", web::post().to(join_connection))
                .route("/connections/link/{link_id}/join", web::post().to(join_connection_by_link))
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
) -> HttpResponse {
    let player_id = player_id.get("player_id")
        .and_then(|id| id.as_str())
        .unwrap_or("")
        .to_string();
        
    let connection = Connection::new(player_id);
    
    // Store both id and link_id mappings
    let mut conn_map = connections.write().unwrap();
    conn_map.insert(connection.link_id.clone(), connection.clone());
    
    HttpResponse::Ok().json(connection)
}

async fn join_connection_by_link(
    link_id: web::Path<String>,
    join_req: web::Json<JoinRequest>,
    connections: web::Data<RwLock<HashMap<String, Connection>>>,
) -> HttpResponse {
    let mut conn_map = connections.write().unwrap();
    let link_id = link_id.into_inner();
    
    if let Some(connection) = conn_map.get_mut(&link_id) {
        if connection.players.len() != 1 {
            return HttpResponse::BadRequest().json(json!({
                "error": "Connection already has maximum players"
            }));
        }
        
        if connection.players.contains(&join_req.player_id) {
            return HttpResponse::BadRequest().json(json!({
                "error": "Player already in connection"
            }));
        }
        
        connection.players.push(join_req.player_id.clone());
        HttpResponse::Ok().json(connection)
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

    #[test]
    fn test_server_new() {
        let server = Server::new("127.0.0.1:8080");
        assert_eq!(server.address, "127.0.0.1:8080");
    }
}