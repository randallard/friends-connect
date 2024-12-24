use actix_web::{web, App, HttpServer, HttpResponse};
use actix_files as fs;
use std::net::TcpListener;
use serde_json::json;

use crate::connection::{Connection, ConnectionStatus};

#[derive(Clone)]
pub struct Server {
    pub address: String, // Remove Arc since it's not needed
}

impl Server {
    pub fn new(address: &str) -> Self {
        Server {
            address: address.to_string(),
        }
    }

    pub async fn run(&self) -> std::io::Result<()> {
        let address = self.address.clone(); // Clone here for the closure
        HttpServer::new(move || {
            App::new()
                .route("/connections", web::post().to(create_connection))
                .service(fs::Files::new("/", "./static")
                .index_file("index.html"))
        })
        .bind(address)?
        .run()
        .await
    }
}

async fn create_connection(player_id: web::Json<serde_json::Value>) -> HttpResponse {
    let player_id = player_id.get("player_id")
        .and_then(|id| id.as_str())
        .unwrap_or("")
        .to_string();
        
    let connection = Connection::new(player_id);
    HttpResponse::Ok().json(connection)
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