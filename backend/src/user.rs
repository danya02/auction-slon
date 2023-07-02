use axum::extract::ws::{close_code, Message, WebSocket};

use communication::{decode, encode, ServerMessage, UserClientMessage};
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
    mut sync_handle: AuctionSyncHandle,
) -> anyhow::Result<()> {
    info!("Client connected as user with key {key:?}");
    let user = match sync_handle.get_member_by_key(key).await {
        None => {
            error!("Key does not match set user password");
            close_socket(
                socket,
                close_code::POLICY,
                "Key does not match set user password",
            )
            .await;
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "User password does not match",
            ))?;
        }
        Some(user) => user,
    };

    // Now, give the user the current info on who they are, other members of the auction, and the auction's state.
    send!(socket, ServerMessage::YourAccount(user));
    let members = sync_handle.auction_members.borrow().clone();
    send!(socket, ServerMessage::AuctionMembers(members));

    loop {
        tokio::select! {
            maybe_packet = socket.recv() => {
                match maybe_packet {
                    None => return Ok(()), // connection closed
                    Some(maybe_packet) => match maybe_packet {
                        Ok(packet) => match packet {
                            Message::Text(data) => {
                                error!("User client sent us text data: {data:?}");
                                close_socket(
                                    socket,
                                    close_code::UNSUPPORTED,
                                    "Expected binary data only",
                                ).await;
                                return Ok(());
                            },
                            Message::Binary(data) => {
                                let msg: UserClientMessage = decode(&data)?;
                                match msg {}
                            },
                            _ => {}
                        },
                        Err(why) => return Err(why)?,
                    },
                }
            },
            _ = sync_handle.auction_state.changed() => {
                let latest_state = sync_handle.auction_state.borrow().clone();
                send!(socket, ServerMessage::AuctionState(latest_state));
            },
        }
    }
}
