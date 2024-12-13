use axum::{
    routing::{get, post},
    Router, Json, extract::{State, Path, Query},
    response::{Redirect, Html},
    http::StatusCode,
};
use friends_connect::{
    memory::InMemoryConnectionManager,
    connection::ConnectionManager,
    models::{Connection, ConnectionRequest},
};
use std::sync::Arc;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
struct User {
    id: String,
    name: String,
}

#[derive(Clone)]
struct AppState {
    connection_manager: Arc<dyn ConnectionManager + Send + Sync>,
    active_users: Arc<tokio::sync::RwLock<Vec<User>>>,
}

#[derive(Deserialize)]
struct LoginRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct ListConnectionsQuery {
    user_id: String,
}

#[tokio::main]
async fn main() {
    // Initialize the library's connection manager
    let connection_manager = Arc::new(InMemoryConnectionManager::new());
    
    let app_state = AppState {
        connection_manager,
        active_users: Arc::new(tokio::sync::RwLock::new(Vec::new())),
    };

    let cors = CorsLayer::permissive();

    let app = Router::new()
        .route("/", get(serve_home))
        .route("/api/login", post(login))
        .route("/api/connections/create", post(create_connection))
        .route("/api/connections/accept/:id", post(accept_connection))
        .route("/api/connections/list", get(list_connections))
        .route("/api/connections/all", get(list_all_connections))
        .route("/connect/:id", get(handle_connection_link))
        .layer(cors)
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running on http://{}", addr);
    println!("This example demonstrates how to use the friends-connect library to:");
    println!("1. Create and manage user connections");
    println!("2. Generate shareable connection links");
    println!("3. Accept connection requests");
    println!("4. List active connections");

    axum::serve(
        tokio::net::TcpListener::bind(addr)
            .await
            .unwrap(),
        app
    )
    .await
    .unwrap();
}

async fn serve_home() -> Html<&'static str> {
    Html(include_str!("static/index.html"))
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<User>, StatusCode> {
    println!("Login request for user: {}", req.name);
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        name: req.name,
    };
    
    state.active_users.write().await.push(user.clone());
    println!("Created user: {:?}", user);
    Ok(Json(user))
}

async fn create_connection(
    State(state): State<AppState>,
    Json(user_id): Json<String>,
) -> Result<Json<ConnectionRequest>, StatusCode> {
    println!("Creating connection for user: {}", user_id);
    let result = state.connection_manager
        .create_connection(
            user_id.clone(),
            "Friend Request".to_string(),
        )
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    println!("Connection creation result: {:?}", result);
    result
}

async fn accept_connection(
    State(state): State<AppState>,
    Path(connection_id): Path<String>,
    Json(user_id): Json<String>,
) -> Result<Json<Connection>, StatusCode> {
    println!("Accepting connection: {} for user: {}", connection_id, user_id);
    let result = state.connection_manager
        .accept_connection(
            &connection_id,
            user_id.clone(),
            "Friend".to_string(),
        )
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    println!("Connection acceptance result: {:?}", result);
    result
}

async fn list_connections(
    State(state): State<AppState>,
    Query(query): Query<ListConnectionsQuery>,
) -> Result<Json<Vec<Connection>>, StatusCode> {
    println!("Listing connections for user: {}", query.user_id);
    let result = state.connection_manager
        .list_connections(&query.user_id)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    println!("Connection list result: {:?}", result);
    result
}

async fn handle_connection_link(
    Path(connection_id): Path<String>,
) -> Redirect {
    println!("Handling connection link: {}", connection_id);
    Redirect::to(&format!("/?connection={}", connection_id))
}

async fn list_all_connections(
    State(state): State<AppState>,
) -> Result<Json<Vec<Connection>>, StatusCode> {
    state.connection_manager
        .list_all_connections()
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}