use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use communication::{
    admin_state::AdminState,
    auction::state::{AuctionItem, AuctionReport, AuctionState},
    ItemState, ItemStateValue, Money, UserAccountData, UserAccountDataWithSecrets,
};
use rand::prelude::*;
use sqlx::{query, SqlitePool};
use tokio::sync::*;
use tracing::{debug, warn};

mod auction_event;
mod english;
mod japanese;
mod sync_handle;
pub use auction_event::*;
pub use english::*;
pub use japanese::*;
pub use sync_handle::*;

use crate::Ignorable;

async fn auction_manager_inner(
    pool: &SqlitePool,
    auction_member_tx: &mut watch::Sender<Vec<UserAccountDataWithSecrets>>,
    get_member_by_key_rx: &mut mpsc::Receiver<(String, oneshot::Sender<Option<UserAccountData>>)>,
    get_member_by_id_rx: &mut mpsc::Receiver<(i64, oneshot::Sender<Option<UserAccountData>>)>,
    auction_state_tx: &mut watch::Sender<AuctionState>,
    auction_event_rx: &mut mpsc::Receiver<AuctionEvent>,
    item_sale_state_tx: &mut watch::Sender<Vec<ItemState>>,
    admin_state_tx: &mut watch::Sender<AdminState>,
) -> anyhow::Result<()> {
    let mut user_data_refresh_interval = tokio::time::interval(Duration::from_secs(1));

    let mut item_data_refresh_interval = tokio::time::interval(Duration::from_secs(5));

    // Initially, the running auction handle is set to a noop task
    // It will be cancelled when needed
    async fn noop() -> anyhow::Result<()> {
        Ok(())
    }
    let mut running_auction_handle = tokio::spawn(noop());

    // These are used to communicate with the two kinds of auction that are running
    // The receiving halves are inside mutexes. Expected to only run one auction task at the same time, so no blocking.
    let (english_tx, english_rx) = mpsc::channel(100);
    let english_rx = Arc::new(Mutex::new(english_rx));
    let (japanese_tx, japanese_rx) = mpsc::channel(100);
    let japanese_rx = Arc::new(Mutex::new(japanese_rx));
    let (state_tx, mut state_rx) = mpsc::channel(100);
    enum AuctionType {
        NoAuction,
        English,
        Japanese,
    }
    use AuctionType::*;
    let mut current_auction = NoAuction;

    // Ensure that the needed KV records are in the database.
    if query!("SELECT * FROM kv_data_int WHERE key='holding_balance'")
        .fetch_optional(pool)
        .await?
        .is_none()
    {
        query!("INSERT INTO kv_data_int (key,value) VALUES ('holding_balance', 0)")
            .execute(pool)
            .await?;
    }

    // Gather the initial admin state and send it.
    let get_admin_state = {
        async move |pool: &SqlitePool| -> anyhow::Result<AdminState> {
            let holding_account_balance =
                query!("SELECT value FROM kv_data_int WHERE key='holding_balance'")
                    .fetch_one(pool)
                    .await?
                    .value;
            let state = AdminState {
                holding_account_balance: holding_account_balance as Money,
                when: SystemTime::now(),
            };
            Ok(state)
        }
    };

    admin_state_tx.send_replace(get_admin_state(&pool).await?);

    let get_item_state = {
        async move |pool: &SqlitePool| -> anyhow::Result<Vec<ItemState>> {
            let item_rows = query!(r#"
                    SELECT
                        auction_item.id, auction_item.name, auction_item.initial_price, auction_item_sale.buyer_id, auction_item_sale.sale_price, auction_user.name AS username, auction_user.balance
                    FROM auction_item
                    LEFT OUTER JOIN auction_item_sale ON auction_item_sale.item_id = auction_item.id
                    LEFT OUTER JOIN auction_user ON auction_item_sale.buyer_id = auction_user.id
                    "#).fetch_all(pool).await?;
            let mut item_data = vec![];
            for row in item_rows {
                let item = AuctionItem {
                    id: row.id,
                    name: row.name,
                    initial_price: row.initial_price as Money,
                };
                let state = match row.buyer_id {
                    None => ItemStateValue::Sellable,
                    Some(id) => ItemStateValue::AlreadySold {
                        buyer: UserAccountData {
                            id,
                            user_name: row.username,
                            balance: row.balance as Money,
                        },
                        sale_price: row.sale_price.unwrap() as Money,
                    },
                };
                item_data.push(ItemState { item, state });
            }
            Ok(item_data)
        }
    };

    let get_user_state =
        async move |pool: &SqlitePool| -> anyhow::Result<Vec<UserAccountDataWithSecrets>> {
            let user_rows = query!("SELECT * FROM auction_user").fetch_all(pool).await?;
            let mut user_data = vec![];
            for row in user_rows {
                user_data.push(UserAccountDataWithSecrets {
                    id: row.id,
                    user_name: row.name,
                    balance: row.balance as u32,
                    login_key: row.login_key,
                });
            }
            Ok(user_data)
        };

    loop {
        tokio::select! {
            // === Periodic updates ===
            _ = item_data_refresh_interval.tick() => {
                item_sale_state_tx.send_replace(get_item_state(pool).await?);
            },

            _ = user_data_refresh_interval.tick() => {
                auction_member_tx.send_replace(get_user_state(pool).await?);
            },



            // === IPC ===
            Some((key, sender)) = get_member_by_key_rx.recv() => {
                let user_row = query!("SELECT * FROM auction_user WHERE login_key=?", key).fetch_optional(pool).await?;
                sender.send(user_row.map(|row| UserAccountData { id: row.id, user_name: row.name, balance: row.balance as Money })).ignore();
            },

            Some((id, sender)) = get_member_by_id_rx.recv() => {
                let user_row = query!("SELECT * FROM auction_user WHERE id=?", id).fetch_optional(pool).await?;
                sender.send(user_row.map(|row| UserAccountData { id: row.id, user_name: row.name, balance: row.balance as Money })).ignore();
            },


            Some(event) = auction_event_rx.recv() => {
                debug!("Received auction event: {event:?}");
                match event {
                    AuctionEvent::StartAuction => {
                        // Switch to auction state = waiting
                        auction_state_tx.send_replace(AuctionState::WaitingForItem);
                        current_auction = NoAuction;
                        running_auction_handle.abort();

                    },
                    AuctionEvent::PrepareAuctioning(item_id) => {
                        // Switch to auction state of showing the item
                        let item = query!("SELECT * FROM auction_item WHERE id=?", item_id).fetch_one(pool).await?;
                        let item = AuctionItem{id: item.id, name: item.name, initial_price: item.initial_price as Money};
                        auction_state_tx.send_replace(AuctionState::ShowingItemBeforeBidding(item));
                        current_auction = NoAuction;
                        running_auction_handle.abort();
                    },
                    AuctionEvent::RunEnglishAuction(item_id) => {
                        running_auction_handle.abort();
                        current_auction = English;
                        running_auction_handle = tokio::spawn(run_english_auction(item_id, pool.clone(), english_rx.clone(), state_tx.clone()));
                    },
                    AuctionEvent::RunJapaneseAuction(item_id) => {
                        running_auction_handle.abort();
                        current_auction = Japanese;
                        running_auction_handle = tokio::spawn(run_japanese_auction(item_id, pool.clone(), japanese_rx.clone(), state_tx.clone()));
                    },

                    AuctionEvent::BidInEnglishAuction{ user_id, item_id, bid_amount } => {
                        // If there is no English auction currently in progress, ignore this
                        if !matches!(current_auction, English) {continue;}
                        english_tx.send(EnglishAuctionEvent::BidPlaced { bidder_id: user_id, bid_amount, item_id }).await?;
                    },

                    AuctionEvent::JapaneseAuctionAction(action) => {
                        // If there is no Japanese auction currently in progress, ignore this
                        if !matches!(current_auction, Japanese) { continue; }
                        japanese_tx.send(action).await?;
                    },

                    AuctionEvent::FinishAuction => {
                        running_auction_handle.abort();
                        current_auction = NoAuction;

                        // Gather auction report
                        // First, collect the latest user data
                        // (which is redundant with the periodic data, but it's more convenient to have it here)
                        let user_data = get_user_state(pool).await?.into_iter().map(|i| i.into()).collect();

                        // Then, collect the sale data
                        let item_data = get_item_state(pool).await?;

                        let report = AuctionReport { items: item_data, members: user_data };
                        auction_state_tx.send_replace(AuctionState::AuctionOver(report));
                    },

                    AuctionEvent::StartAuctionAnew => {
                        running_auction_handle.abort();
                        current_auction = NoAuction;
                        auction_state_tx.send_replace(AuctionState::WaitingForAuction);
                    },

                    AuctionEvent::EditUser {id, name, balance} => {
                        match id {
                            None => {
                                // Creating user
                                if (&name, balance) == (&None, None) {
                                    warn!("Received AuctionEvent::EditUser with all empty fields -- bug?");
                                    continue;
                                } else {
                                    let mut login_key = String::new();
                                    {
                                        let mut rng = rand::thread_rng();
                                        for _ in 0..8 {
                                            let num = rng.gen_range(0..=9);
                                            login_key.extend(num.to_string().chars());
                                        }
                                    }
                                    let name = if let Some(name) = name {
                                        if name.trim().is_empty() {None} else {Some(name.trim().to_string())}
                                    } else {None};
                                    let name = name.or(Some("Unnamed".to_string())).unwrap();
                                    let balance = balance.or(Some(0)).unwrap();
                                    query!(
                                        "INSERT INTO auction_user (name, balance, login_key) VALUES (?,?,?)",
                                        name,
                                        balance,
                                        login_key,
                                    ).execute(pool).await?;
                                };
                                auction_member_tx.send_replace(get_user_state(pool).await?);
                            },
                            Some(id) => {
                                // Changing or deleting user
                                if (&name, balance) == (&None, None) {
                                    query!("DELETE FROM auction_user WHERE id=?", id).execute(pool).await?;
                                } else {
                                    let mut tx = pool.begin().await?;
                                    if let Some(name) = name {
                                        query!("UPDATE auction_user SET name=? WHERE id=?", name, id).execute(&mut tx).await?;
                                    }
                                    if let Some(balance) = balance {
                                        query!("UPDATE auction_user SET balance=? WHERE id=?", balance, id).execute(&mut tx).await?;
                                    }
                                    tx.commit().await?;
                                };
                                auction_member_tx.send_replace(get_user_state(pool).await?);
                            },
                        };
                    },

                    AuctionEvent::ClearSaleStatus {id} => {
                        // Remove the sale row, if it exists.
                        // After, send the item states.
                        query!("DELETE FROM auction_item_sale WHERE item_id=?", id).execute(pool).await?;
                        item_sale_state_tx.send_replace(get_item_state(pool).await?);
                    },

                    AuctionEvent::EditItem {id, name, initial_price} => {
                        match id {
                            Some(id) => {
                                // Editing or deleting item
                                if (&name, initial_price) == (&None, None) {
                                    query!("DELETE FROM auction_item WHERE id=?", id).execute(pool).await?;
                                } else {
                                    let mut tx = pool.begin().await?;

                                    if let Some(name) = name.clone() {
                                        query!("UPDATE auction_item SET name=? WHERE id=?", name, id).execute(&mut tx).await?;
                                    }
                                    if let Some(price) = initial_price {
                                        query!("UPDATE auction_item SET name=? WHERE initial_price=?", name, price).execute(&mut tx).await?;
                                    }
                                    tx.commit().await?;
                                }
                            },
                            None => {
                                // Creating item
                                let name = name.or(Some(String::new())).unwrap().clone();
                                let name = name.trim();
                                let name = if name.is_empty() {
                                    "Unnamed Item".to_string()
                                } else {
                                    name.to_string()
                                };

                                let price = initial_price.or(Some(1)).unwrap();

                                query!("INSERT INTO auction_item (name, initial_price) VALUES (?,?)", name, price).execute(pool).await?;
                            },
                        };

                        // After the action was taken, send the current item states.
                        item_sale_state_tx.send_replace(get_item_state(pool).await?);
                    },
                    AuctionEvent::HoldingAccountTransfer { user_id, new_balance } => {
                        let mut tx = pool.begin().await?;
                        let user_balance = query!("SELECT balance FROM auction_user WHERE id=?", user_id).fetch_optional(&mut tx).await?;
                        let user_balance = match user_balance {
                            Some(t) => t.balance as Money,
                            None => {
                                warn!("Tried to transfer across holding account for user ID {user_id}, which does not exist -- desync?");
                                admin_state_tx.send_replace(get_admin_state(&pool).await?);
                                continue;
                            }
                        };
                        let holding_balance = admin_state_tx.borrow().holding_account_balance;
                        let new_user_balance;
                        let new_holding_balance;
                        if new_balance < user_balance {
                            // Taking money out of user account and putting it into holding
                            // (typically useless check because Money is unsigned)
                            #[allow(unused_comparisons)]
                            let to_withdraw = if new_balance < 0 {
                                user_balance
                            } else {
                                user_balance - new_balance
                            };

                            new_user_balance = user_balance - to_withdraw;
                            new_holding_balance = holding_balance + to_withdraw;
                        } else {
                            // Taking money out of holding and put it into user account
                            let to_deposit = if (new_balance - user_balance)>holding_balance {
                                // This means that we requested to deposit more than holding balance
                                // So deposit all
                                holding_balance
                            } else {
                                new_balance - user_balance
                            };
                            new_holding_balance = holding_balance - to_deposit;
                            new_user_balance = user_balance + to_deposit;
                        }
                        query!("UPDATE auction_user SET balance=? WHERE id=?", new_user_balance, user_id).execute(&mut tx).await?;
                        query!("UPDATE kv_data_int SET value=? WHERE key='holding_balance'", new_holding_balance).execute(&mut tx).await?;
                        tx.commit().await?;

                        admin_state_tx.send_replace(get_admin_state(&pool).await?);
                        auction_member_tx.send_replace(get_user_state(pool).await?);

                    },
                }
            },
            Some(state) = state_rx.recv() => {
                // auction process is publishing an auction state
                // but only if an auction is supposed to be running
                if !matches!(current_auction, NoAuction) {
                    auction_state_tx.send_replace(state);
                }
            }

        }
    }
}
