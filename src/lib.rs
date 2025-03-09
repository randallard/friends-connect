use actix_web::{web, App, HttpServer, HttpResponse};
use actix_files as fs;
use std::net::TcpListener;
use serde_json::json;

pub mod connection;
pub mod server; 
pub mod websocket; 

pub use connection::{Connection, ConnectionStatus};
pub use server::Server;
pub use websocket::setup_ws_routes;