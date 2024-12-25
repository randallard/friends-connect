// examples/create_connection.rs
use reqwest;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    // First create a connection with player1
    println!("Creating connection with player1...");
    let create_response = client
        .post("http://localhost:8080/connections")
        .json(&json!({
            "player_id": "player1"
        }))
        .send()
        .await?;

    println!("Create Status: {}", create_response.status());
    let connection = create_response.json::<serde_json::Value>().await?;
    println!("Created Connection: {}", serde_json::to_string_pretty(&connection)?);

    // Get the link_id from the response
    let link_id = connection["link_id"].as_str().unwrap();

    // Join the connection with player2
    println!("\nJoining connection with player2...");
    let join_response = client
        .post(&format!("http://localhost:8080/connections/link/{}/join", link_id))
        .json(&json!({
            "player_id": "player2"
        }))
        .send()
        .await?;

    println!("Join Status: {}", join_response.status());
    let joined = join_response.json::<serde_json::Value>().await?;
    println!("Joined Connection: {}", serde_json::to_string_pretty(&joined)?);

    Ok(())
}