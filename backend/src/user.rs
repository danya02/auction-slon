use std::time::Duration;

use axum::extract::ws::{close_code, Message, WebSocket};

use communication::{
    auction::state::AuctionState, decode, encode, ServerMessage, UserClientMessage,
};
use tokio::time::interval;
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
    let (mut user, mut disconnect_handle) = match sync_handle.get_member_by_key(key).await {
        None => {
            error!("Key does not match set user password");
            close_socket(
                socket,
                close_code::POLICY,
                "Key does not match set user password",
            )
            .await;
            return Ok(());
        }
        Some(user) => user,
    };

    // Now, we will give the user the current info on who they are, other members of the auction, and the auction's state,
    // when this interval first ticks (which should be immediate).
    let mut refresh_interval = interval(Duration::from_secs(5)); // tune this so that the user does not spend too long with outdated info

    loop {
        tokio::select! {
            Ok(_) = &mut disconnect_handle => {
                close_socket(
                    socket,
                    close_code::POLICY,
                    "Your login key was used to login from a different device. You can only be logged in from one device at a time",
                )
                .await;
                return Ok(());
            },

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
                                match msg {
                                    UserClientMessage::BidInEnglishAuction { item_id, bid_amount } => {
                                        sync_handle.send_event(crate::auction::AuctionEvent::BidInEnglishAuction { user_id: user.id, item_id, bid_amount }).await;
                                    },
                                    UserClientMessage::JapaneseAuctionAction { item_id, action } => {
                                        sync_handle.send_event(crate::auction::AuctionEvent::JapaneseAuctionAction(crate::auction::JapaneseAuctionEvent::UserAction { user_id: user.id, item_id, action })).await;
                                    },
                                }
                            },
                            _ => {}
                        },
                        Err(why) => return Err(why)?,
                    },
                }
            },
            _ = sync_handle.auction_state.changed() => {
                let latest_state = sync_handle.auction_state.borrow().clone();
                // Map SoldToMember to SoldToYou or SoldToSomeoneElse
                let latest_state = match latest_state {
                    AuctionState::SoldToMember{ item, sold_for, sold_to, confirmation_code } => {
                        if sold_to.id == user.id {
                            AuctionState::SoldToYou { item, sold_for, confirmation_code }
                        } else {
                            AuctionState::SoldToSomeoneElse { item, sold_to, sold_for }
                        }
                    },
                    other => other
                };
                send!(socket, ServerMessage::AuctionState(latest_state));
            },

            _ = refresh_interval.tick() => {
                user = match sync_handle.get_member_by_id(user.id).await {
                    None => {
                        error!("User ID disappeared while program was running");
                        close_socket(
                            socket,
                            close_code::ERROR,
                            "User ID disappeared while program was running",
                        )
                        .await;
                        return Ok(());
                    }
                    Some(user) => user,
                };

                send!(socket, ServerMessage::YourAccount(user.clone()));
                let members = sync_handle.auction_members.borrow().clone();
                send!(socket, ServerMessage::AuctionMembers(members));

                // Also resend the auction state, just in case it were lost.
                // This is copy-pasted from the case above when the auction state changes
                let latest_state = sync_handle.auction_state.borrow().clone();
                // Map SoldToMember to SoldToYou or SoldToSomeoneElse
                let latest_state = match latest_state {
                    AuctionState::SoldToMember{ item, sold_for, sold_to, confirmation_code } => {
                        if sold_to.id == user.id {
                            AuctionState::SoldToYou { item, sold_for, confirmation_code }
                        } else {
                            AuctionState::SoldToSomeoneElse { item, sold_to, sold_for }
                        }
                    },
                    other => other
                };
                send!(socket, ServerMessage::AuctionState(latest_state));


            },
        }
    }
}
