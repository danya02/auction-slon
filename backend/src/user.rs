use axum::extract::ws::{WebSocket, close_code};

#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::close_socket;


pub async fn handle_socket(mut socket: WebSocket, key: String) {
    info!("Client {socket:?} connected as user with key {key:?}");
    if key != "user-pw" {
        error!("Key does not match set user password");
        return close_socket(socket, close_code::POLICY, "Key does not match set user password").await;
    }
}
