// examples/test_deployed.rs
use reqwest;
use serde_json::json;

const DEPLOY_URL: &str = "https://friends-connect.fly.dev";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    // First create a connection with player1
    println!("Creating connection with player1...");
    let create_response = client
        .post(&format!("{}/connections", DEPLOY_URL))
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
        .post(&format!("{}/connections/link/{}/join", DEPLOY_URL, link_id))
        .json(&json!({
            "player_id": "player2"
        }))
        .send()
        .await?;

    println!("Join Status: {}", join_response.status());
    let joined = join_response.json::<serde_json::Value>().await?;
    println!("Joined Connection: {}", serde_json::to_string_pretty(&joined)?);

    // Player 1 sends a message
    println!("\nPlayer 1 sending message...");
    let p1_message = client
        .post(&format!("{}/connections/{}/messages", DEPLOY_URL, connection["id"].as_str().unwrap()))
        .json(&json!({
            "player_id": "player1",
            "content": "Hi player2! Welcome to the deployed game!"
        }))
        .send()
        .await?;

    println!("Player 1 Message Status: {}", p1_message.status());
    println!("Player 2's notifications:");
    let p2_notifications = client
        .get(&format!("{}/players/player2/notifications", DEPLOY_URL))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    println!("{}", serde_json::to_string_pretty(&p2_notifications)?);

    // Player 2 sends a message back
    println!("\nPlayer 2 sending message...");
    let p2_message = client
        .post(&format!("{}/connections/{}/messages", DEPLOY_URL, connection["id"].as_str().unwrap()))
        .json(&json!({
            "player_id": "player2",
            "content": "Thanks player1! The deployed server works!"
        }))
        .send()
        .await?;

    println!("Player 2 Message Status: {}", p2_message.status());
    println!("Player 1's notifications:");
    let p1_notifications = client
        .get(&format!("{}/players/player1/notifications", DEPLOY_URL))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    println!("{}", serde_json::to_string_pretty(&p1_notifications)?);

    Ok(())
}