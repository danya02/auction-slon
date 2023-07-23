use std::{sync::Arc, time::Duration};

use axum::extract::ws::{close_code, Message, WebSocket};

use communication::{
    auction::state::AuctionState, decode, encode, forget_user_secrets, ServerMessage,
    UserAccountData, UserAccountDataWithSecrets, UserClientMessage, WithTimestamp,
};
use tokio::time::interval;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::{
    auction::{AuctionEvent, AuctionSyncHandle},
    close_socket,
};

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

    // Once we have a user, we'll lock the mutex,
    // and give away a Weak reference to a value that we own;
    // that way, the auction task will know when this thread dies.
    let my_value = Arc::new(());
    let my_value_weak = Arc::downgrade(&my_value);
    {
        let mut connection_active_handles = sync_handle.connection_active_handles.lock().await;
        connection_active_handles.insert(user.id, my_value_weak);
    }

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
                                        sync_handle.send_event(AuctionEvent::EnglishAuctionAction(crate::auction::EnglishAuctionEvent::BidPlaced { bidder_id: user.id, bid_amount, item_id})).await;
                                    },
                                    UserClientMessage::JapaneseAuctionAction { item_id, action } => {
                                        sync_handle.send_event(AuctionEvent::JapaneseAuctionAction(crate::auction::JapaneseAuctionEvent::UserAction { user_id: user.id, item_id, action })).await;
                                    },
                                    UserClientMessage::SetIsAcceptingSponsorships(state) => {
                                        sync_handle.send_event(AuctionEvent::SetIsAcceptingSponsorships { user_id: user.id, is_accepting_sponsorships: state }).await;
                                    },
                                    UserClientMessage::SetSaleMode(sale_mode) => {
                                        sync_handle.send_event(AuctionEvent::SetSaleMode{ user_id: user.id, sale_mode }).await;
                                    },
                                    UserClientMessage::TryActivateSponsorshipCode(code) => {
                                        sync_handle.send_event(AuctionEvent::TryActivateSponsorshipCode { user_id: user.id, code }).await;
                                    },
                                    UserClientMessage::SetSponsorshipBalance { sponsorship_id, balance } => {
                                        sync_handle.send_event(AuctionEvent::UpdateSponsorship{
                                            actor_id: user.id,
                                            sponsorship_id,
                                            new_amount: Some(balance),
                                            new_status: None,
                                        }).await;
                                    },
                                    UserClientMessage::SetSponsorshipStatus { sponsorship_id, status } => {
                                        sync_handle.send_event(AuctionEvent::UpdateSponsorship{
                                            actor_id: user.id,
                                            sponsorship_id,
                                            new_amount: None,
                                            new_status: Some(status),
                                        }).await;
                                    },

                                    UserClientMessage::RegenerateSponsorshipCode => {
                                        sync_handle.send_event(AuctionEvent::RegenerateSponsorshipCode { user_id: user.id}).await;
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
                    AuctionState::SoldToMember{ item, sold_for, sold_to, confirmation_code, contributions } => {
                        if sold_to.id == user.id {
                            AuctionState::SoldToYou { item, sold_for, confirmation_code, contributions }
                        } else {
                            AuctionState::SoldToSomeoneElse { item, sold_to, sold_for, contributions }
                        }
                    },
                    other => other
                };
                send!(socket, ServerMessage::AuctionState(latest_state.into()));
            },
            _ = sync_handle.auction_members.changed() => {
                let latest_state_with_secrets: Vec<UserAccountDataWithSecrets> = sync_handle.auction_members.borrow().clone();
                // Extract the state of the current user, and send that in addition to the info about everyone.
                user = latest_state_with_secrets.iter().find(|i: &&UserAccountDataWithSecrets| i.id == user.id).expect("Connected user disappeared from auction members?").clone();
                send!(socket, ServerMessage::YourAccount(user.clone()));
                send!(socket, ServerMessage::AuctionMembers(forget_user_secrets(latest_state_with_secrets).into()));

            },
            _ = sync_handle.sponsorship_state.changed() => {
                let latest_state = sync_handle.sponsorship_state.borrow().clone();
                send!(socket, ServerMessage::SponsorshipState(latest_state.into()));
            },


            _ = refresh_interval.tick() => {
                /*
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
                };*/

                send!(socket, ServerMessage::YourAccount(user.clone()));
                let members: Vec<UserAccountData> = forget_user_secrets(sync_handle.auction_members.borrow().clone());
                let members: WithTimestamp<_> = members.into();
                send!(socket, ServerMessage::AuctionMembers(members));

                // Also resend the auction state, just in case it were lost.
                // This is copy-pasted from the case above when the auction state changes
                let latest_state = sync_handle.auction_state.borrow().clone();
                // Map SoldToMember to SoldToYou or SoldToSomeoneElse
                let latest_state = match latest_state {
                    AuctionState::SoldToMember{ item, sold_for, sold_to, confirmation_code, contributions } => {
                        if sold_to.id == user.id {
                            AuctionState::SoldToYou { item, sold_for, confirmation_code, contributions }
                        } else {
                            AuctionState::SoldToSomeoneElse { item, sold_to, sold_for, contributions }
                        }
                    },
                    other => other
                };
                send!(socket, ServerMessage::AuctionState(latest_state.into()));
                let latest_state = sync_handle.sponsorship_state.borrow().clone();
                send!(socket, ServerMessage::SponsorshipState(latest_state.into()));

            },
        }
    }
}
