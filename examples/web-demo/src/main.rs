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
    error::ConnectionError,
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
        .route("/api/connections/delete/:id", post(delete_connection))
        .route("/api/connections/create", post(create_connection))
        .route("/api/connections/accept/:id", post(accept_connection))
        .route("/api/connections/list", get(list_connections))
        .route("/api/connections/all", get(list_all_connections))
        .route("/api/connections/check/:id", get(check_connection))
        .route("/connect/:id", get(handle_connection_link))
        .route("/api/connections/recover", post(recover_connection))
        .route("/api/backup-key", get(get_backup_key))
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

async fn check_connection(
    State(state): State<AppState>,
    Path(connection_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    match state.connection_manager.get_request(&connection_id).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(_) => Ok(StatusCode::NOT_FOUND)
    }
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

async fn delete_connection(
    State(state): State<AppState>,
    Path(connection_id): Path<String>,
) -> StatusCode {
    println!("Deleting connection: {}", connection_id);
    match state.connection_manager.delete_connection(&connection_id).await {
        Ok(_) | Err(ConnectionError::NotFound) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

use ring::hmac;
use base64::{Engine as _, engine::general_purpose::STANDARD as b64};

impl AppState {
    fn generate_backup_key(&self, user_id: &str) -> String {
        // Create a deterministic but unique key for each user
        let key = hmac::Key::new(hmac::HMAC_SHA256, b"friends-connect-backup-key");
        let signature = hmac::sign(&key, user_id.as_bytes());
        b64.encode(signature.as_ref())
    }

    fn verify_backup_key(&self, user_id: &str, provided_key: &str) -> bool {
        let expected_key = self.generate_backup_key(user_id);
        ring::constant_time::verify_slices_are_equal(
            expected_key.as_bytes(),
            provided_key.as_bytes()
        ).is_ok()
    }
}

async fn get_backup_key(
    State(state): State<AppState>,
    Query(query): Query<ListConnectionsQuery>,
) -> Result<Json<String>, StatusCode> {
    let key = state.generate_backup_key(&query.user_id);
    Ok(Json(key))
}

async fn recover_connection(
    State(state): State<AppState>,
    Json(connection): Json<Connection>,
) -> StatusCode {
    println!("Recovery attempt for connection {}", connection.id);
    println!("  Status: {:?}", connection.status);
    println!("  Initiator: {} ({})", connection.initiator_id, connection.initiator_label);
    if let Some(ref recipient) = connection.recipient_id {
        println!("  Recipient: {} ({:?})", recipient, connection.recipient_label);
    }
    println!("  Created: {}", connection.created_at);
    if let Some(connected) = connection.connected_at {
        println!("  Connected: {}", connected);
    }

    match state.connection_manager.recover_connection(connection).await {
        Ok(_) => {
            println!("  Recovery successful");
            StatusCode::OK
        }
        Err(ConnectionError::AlreadyExists) => {
            println!("  Connection already exists - no recovery needed");
            StatusCode::OK
        }
        Err(e) => {
            println!("  Recovery failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}