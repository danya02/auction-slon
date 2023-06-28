use axum::extract::ws::{close_code, Message, WebSocket};

use communication::{encode, ServerMessage};
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::{auction::AuctionSyncHandle, close_socket};

macro_rules! send {
    ($s:expr, $v:expr) => {
        $s.send(Message::Binary(encode::<ServerMessage>(&$v.into())))
            .await?;
    };
}


pub async fn handle_socket(
    mut socket: WebSocket,
    key: String,
    sync_handle: AuctionSyncHandle,
) -> anyhow::Result<()> {
    info!("Client {socket:?} connected as admin with key {key:?}");
    if key != "admin-pw" {
        error!("Key does not match set admin password");
        close_socket(
            socket,
            close_code::POLICY,
            "Key does not match set admin password",
        )
        .await;
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Admin password does not match",
        ))?;
    }


    todo!()
}
