use actix_web::{web, App, HttpServer, HttpResponse};
use actix_files as fs;
use std::net::TcpListener;
use serde_json::json;

pub mod connection;
pub mod server;  

pub use connection::{Connection, ConnectionStatus};
pub use server::Server;
