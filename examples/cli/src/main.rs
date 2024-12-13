use clap::{Parser, Subcommand};
use reqwest;
use friends_connect::models::{Connection, ConnectionStatus};

// In main.rs of friends-connect-cli
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Server URL (default: http://localhost:3000)
    #[arg(long, default_value = "http://localhost:3000")]
    server: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all connections
    List {
        /// Filter by user ID
        #[arg(long)]
        user: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let client = reqwest::Client::new();
    let base_url = cli.server.trim_end_matches('/');

    match cli.command {
        Commands::List { user } => {
            let url = if let Some(user_id) = user {
                format!("{}/api/connections/list?user_id={}", base_url, user_id)
            } else {
                format!("{}/api/connections/all", base_url)
            };

            let connections: Vec<Connection> = client.get(&url)
                .send()
                .await?
                .json()
                .await?;

            print_connection_summary(&connections);
        }
    }

    Ok(())
}

fn print_connection_summary(connections: &[Connection]) {
    let active_count = connections.iter()
        .filter(|c| matches!(c.status, ConnectionStatus::Active))
        .count();
    let pending_count = connections.iter()
        .filter(|c| matches!(c.status, ConnectionStatus::Pending))
        .count();

    println!("\nConnection Summary:");
    println!("Active:  {}", active_count);
    println!("Pending: {}", pending_count);
    println!("Total:   {}\n", connections.len());

    if connections.is_empty() {
        println!("No connections found.");
        return;
    }

    println!("All Connections:");
    println!("{:<36} {:<10} {:<20} {:<20}", "ID", "Status", "From", "To");
    println!("{:-<88}", "");
    
    for conn in connections {
        println!("{:<36} {:<10} {:<20} {:<20}",
            &conn.id[..],
            format!("{:?}", conn.status),
            &conn.initiator_id[..],
            conn.recipient_id.as_deref().unwrap_or("-")
        );
    }
}