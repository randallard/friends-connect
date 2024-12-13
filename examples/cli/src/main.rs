use clap::{Parser, Subcommand};
use friends_connect::{
    memory::InMemoryConnectionManager,
    connection::ConnectionManager,
};
use tokio;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all connections
    List,
    /// Show details for a specific connection
    Show {
        #[arg(help = "Connection ID to show")]
        id: String,
    },
    /// List pending connection requests
    ListPending {
        #[arg(help = "User ID to filter by (optional)")]
        user_id: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let connection_manager = InMemoryConnectionManager::new();

    match cli.command {
        Commands::List => {
            // For demo purposes, we'll list all connections for a test user
            let connections = connection_manager.list_connections("test-user").await?;
            println!("\nAll Connections:");
            println!("{:<36} {:<15} {:<15} {}", "ID", "Status", "Initiator", "Recipient");
            println!("{:-<80}", "");
            
            for conn in connections {
                println!("{:<36} {:<15} {:<15} {}",
                    conn.id,
                    format!("{:?}", conn.status),
                    conn.initiator_id,
                    conn.recipient_id.unwrap_or_default()
                );
            }
        },
        Commands::Show { id } => {
            match connection_manager.get_connection(&id).await {
                Ok(conn) => {
                    println!("\nConnection Details:");
                    println!("ID:              {}", conn.id);
                    println!("Status:          {:?}", conn.status);
                    println!("Initiator ID:    {}", conn.initiator_id);
                    println!("Initiator Label: {}", conn.initiator_label);
                    println!("Recipient ID:    {}", conn.recipient_id.unwrap_or_default());
                    println!("Recipient Label: {}", conn.recipient_label.unwrap_or_default());
                    println!("Created At:      {}", conn.created_at);
                    println!("Connected At:    {}", conn.connected_at.map_or("Not connected".to_string(), |t| t.to_string()));
                }
                Err(e) => println!("Error: Connection not found - {}", e),
            }
        },
        Commands::ListPending { user_id } => {
            let connections = connection_manager.list_connections(user_id.as_deref().unwrap_or("test-user")).await?;
            let pending = connections.iter().filter(|c| matches!(c.status, friends_connect::models::ConnectionStatus::Pending));
            
            println!("\nPending Connections:");
            println!("{:<36} {:<15} {}", "ID", "Initiator", "Created At");
            println!("{:-<80}", "");
            
            for conn in pending {
                println!("{:<36} {:<15} {}",
                    conn.id,
                    conn.initiator_id,
                    conn.created_at
                );
            }
        }
    }

    Ok(())
}