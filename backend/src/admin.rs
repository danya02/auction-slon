use axum::extract::ws::{close_code, Message, WebSocket};

use communication::{decode, encode, AdminClientMessage, AdminServerMessage};
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::{
    auction::{AuctionEvent, AuctionSyncHandle},
    close_socket,
};

macro_rules! send {
    ($s:expr, $v:expr) => {
        $s.send(Message::Binary(encode::<AdminServerMessage>(&$v.into())))
            .await?;
    };
}

pub async fn handle_socket(
    mut socket: WebSocket,
    key: String,
    mut sync_handle: AuctionSyncHandle,
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

    // Now send the auction info
    let members = sync_handle.auction_members.borrow().clone();
    send!(socket, AdminServerMessage::AuctionMembers(members));
    let state = sync_handle.auction_state.borrow().clone();
    send!(socket, AdminServerMessage::AuctionState(state));
    send!(socket, AdminServerMessage::ItemStates(vec![]));

    loop {
        tokio::select! {
            maybe_msg = socket.recv() => {
                match maybe_msg {
                    None => {info!("Admin client disconnected"); return Ok(());}
                    Some(maybe_msg) => match maybe_msg {
                        Err(why) => {error!("Admin client recv error: {why} {why:?}"); return Err(why)?;}
                        Ok(msg) => match msg {
                            Message::Text(data) => {
                                error!("Admin client sent us text data: {data:?}");
                                close_socket(
                                    socket,
                                    close_code::UNSUPPORTED,
                                    "Expected binary data only",
                                ).await;
                                return Ok(());
                            },
                            Message::Binary(data) => {
                                let msg: AdminClientMessage = decode(&data)?;
                                match msg {
                                    AdminClientMessage::StartAuction => {sync_handle.send_event(AuctionEvent::StartAuction).await},
                                }
                            },
                            _ => {},
                        },
                    },
                }
            },
            _ = sync_handle.auction_state.changed() => {
                let latest_state = sync_handle.auction_state.borrow().clone();
                send!(socket, AdminServerMessage::AuctionState(latest_state));
            },
        }
    }
}