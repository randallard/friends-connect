// examples/create_connection.rs
use reqwest;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8080/connections")
        .json(&json!({
            "player_id": "player123"
        }))
        .send()
        .await?;

    println!("Status: {}", response.status());
    let body = response.text().await?;
    println!("Response: {}", body);

    Ok(())
}