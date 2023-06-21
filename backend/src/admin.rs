use axum::extract::ws::{WebSocket, close_code};

#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::close_socket;


pub async fn handle_socket(mut socket: WebSocket, key: String) {
    info!("Client {socket:?} connected as admin with key {key:?}");
    if key != "admin-pw" {
        error!("Key does not match set admin password");
        return close_socket(socket, close_code::POLICY, "Key does not match set admin password").await;
    }

    

}
