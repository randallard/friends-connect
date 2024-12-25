use friends_connect::Server;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let address = format!("0.0.0.0:{}", port);
    let server = Server::new(&address);
    println!("Server running at http://{}", address);
    server.run().await
}