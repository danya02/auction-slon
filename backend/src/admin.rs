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
                                    AdminClientMessage::StartAuction => sync_handle.send_event(AuctionEvent::StartAuction).await,
                                    AdminClientMessage::PrepareAuctioning(item_id) => sync_handle.send_event(AuctionEvent::PrepareAuctioning(item_id)).await,
                                    AdminClientMessage::RunEnglishAuction(item_id) => sync_handle.send_event(AuctionEvent::RunEnglishAuction(item_id)).await,
                                    AdminClientMessage::RunJapaneseAuction(item_id) => sync_handle.send_event(AuctionEvent::RunJapaneseAuction(item_id)).await,
                                    AdminClientMessage::FinishAuction => sync_handle.send_event(AuctionEvent::FinishAuction).await,
                                    AdminClientMessage::StartAuctionAnew => sync_handle.send_event(AuctionEvent::StartAuctionAnew).await,
                                    AdminClientMessage::KickFromJapaneseAuction(item_id, user_id) => {
                                            info!("Admin requested kick of {user_id} from Japanese auction");
                                            sync_handle.send_event(
                                            AuctionEvent::JapaneseAuctionAction(
                                                // Emit an event that's as though the user left on their own
                                                crate::auction::JapaneseAuctionEvent::UserAction {
                                                    user_id,
                                                    item_id,
                                                    action: communication::auction::actions::JapaneseAuctionAction::ExitArena,
                                                }
                                            )
                                        ).await
                                    },
                                    AdminClientMessage::SetJapaneseClockRate(new_rate) => sync_handle.send_event(
                                        AuctionEvent::JapaneseAuctionAction(
                                            crate::auction::JapaneseAuctionEvent::NewPriceClockInterval { price_increase_per_100_seconds: new_rate }
                                        )
                                    ).await,
                                    AdminClientMessage::ChangeUserName{id, new_name} => {
                                        sync_handle.send_event(
                                            AuctionEvent::EditUser { id: Some(id), name: Some(new_name), balance: None }
                                        ).await;
                                    },
                                    AdminClientMessage::ChangeUserBalance{id, new_balance} => {
                                        // Try parsing the provided value as a number.
                                        // If that fails, ignore it.
                                        let balance = Some(
                                            match new_balance.parse() {
                                                Ok(v) => v,
                                                Err(_) => {
                                                    warn!("Admin inputted invalid number: {new_balance}");
                                                    // Send the latest user state immediately.
                                                    {
                                                        let members = sync_handle.auction_members.borrow().clone();
                                                        send!(socket, AdminServerMessage::AuctionMembers(members));
                                                    }
                                                    continue;
                                                },
                                            }
                                        );
                                        sync_handle.send_event(
                                            AuctionEvent::EditUser { id: Some(id), name: None, balance: balance }
                                        ).await;
                                    },
                                    AdminClientMessage::CreateUser{name} => {
                                        sync_handle.send_event(
                                            AuctionEvent::EditUser { id: None, name: Some(name), balance: None }
                                        ).await;
                                    },
                                    AdminClientMessage::DeleteUser{id} => {
                                        sync_handle.send_event(
                                            AuctionEvent::EditUser { id: Some(id), name: None, balance: None }
                                        ).await;
                                    },
                                    AdminClientMessage::ClearSaleStatus{id} => sync_handle.send_event(AuctionEvent::ClearSaleStatus{id}).await,
                                    AdminClientMessage::ChangeItemName{id, new_name} => sync_handle.send_event(
                                        AuctionEvent::EditItem {id: Some(id), name: Some(new_name), initial_price: None},
                                    ).await,
                                    AdminClientMessage::ChangeItemInitialPrice{id, new_price} => {
                                        // Try parsing the provided value as a number.
                                        // If that fails, ignore it.
                                        let price = Some(
                                            match new_price.parse() {
                                                Ok(v) => v,
                                                Err(_) => {
                                                    warn!("Admin inputted invalid number: {new_price}");
                                                    // Send the latest user state immediately.
                                                    {
                                                        let items = sync_handle.item_sale_states.borrow().clone();
                                                        send!(socket, AdminServerMessage::ItemStates(items));
                                                    }
                                                    continue;
                                                },
                                            }
                                        );
                                        sync_handle.send_event(
                                            AuctionEvent::EditItem {id: Some(id), name: None, initial_price: price},
                                        ).await;
                                    },
                                    AdminClientMessage::DeleteItem{id} => sync_handle.send_event(
                                            AuctionEvent::EditItem {id: Some(id), name: None, initial_price: None},
                                    ).await,
                                    AdminClientMessage::CreateItem{name} => sync_handle.send_event(
                                            AuctionEvent::EditItem {id: None, name: Some(name), initial_price: None},
                                    ).await,
                                }
                            },
                            _ => {},
                        },
                    },
                }
            },
            _ = sync_handle.auction_members.changed() => {
                let latest_state = sync_handle.auction_members.borrow().clone();
                send!(socket, AdminServerMessage::AuctionMembers(latest_state));
            },
            _ = sync_handle.auction_state.changed() => {
                let latest_state = sync_handle.auction_state.borrow().clone();
                send!(socket, AdminServerMessage::AuctionState(latest_state));
            },
            _ = sync_handle.item_sale_states.changed() => {
                let latest_state = sync_handle.item_sale_states.borrow().clone();
                send!(socket, AdminServerMessage::ItemStates(latest_state));
            },
        }
    }
}
