use std::{sync::Arc, time::Duration};

use communication::{
    admin_state::AdminState,
    auction::state::{AuctionItem, AuctionReport, AuctionState, Sponsorship, SponsorshipStatus},
    forget_user_secrets, ItemState, Money, UserAccountDataWithSecrets,
};
use rand::prelude::*;
use sqlx::{query, SqlitePool};
use tokio::{sync::*, time::interval};
use tracing::{debug, warn};

mod auction_event;
mod db_actions;
mod english;
mod japanese;
mod sync_handle;
pub use auction_event::*;
pub use english::*;
pub use japanese::*;
pub use sync_handle::*;

use crate::{
    auction::db_actions::{get_item_state, get_sponsorship_state, get_user_state},
    Ignorable,
};

async fn gen_sponsorship_code(pool: &SqlitePool) -> anyhow::Result<String> {
    // Because ThreadRng is not Send, it cannot be alive by the time that an `await` point is crossed;
    // however, we may need to generate multiple codes.
    // So, we seed our own StdRng, which is allowed to survive an `await`.
    let seed = rand::random();
    let mut rng = StdRng::from_seed(seed);

    let nums = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
    let mut code = String::with_capacity(4);
    loop {
        // Try a new code
        for _ in 0..4 {
            code.push(*nums.choose(&mut rng).unwrap());
        }
        // If that code doesn't exist in the database, then we're done.
        // If it does, loop again.
        if let None = query!("SELECT id FROM auction_user WHERE sponsorship_code=?", code)
            .fetch_optional(pool)
            .await?
        {
            break;
        }
    }
    Ok(code)
}

async fn auction_manager_inner(
    pool: &SqlitePool,
    auction_member_tx: &mut watch::Sender<Vec<UserAccountDataWithSecrets>>,
    get_member_by_key_rx: &mut mpsc::Receiver<(
        String,
        oneshot::Sender<Option<UserAccountDataWithSecrets>>,
    )>,
    auction_state_tx: &mut watch::Sender<AuctionState>,
    auction_event_rx: &mut mpsc::Receiver<AuctionEvent>,
    item_sale_state_tx: &mut watch::Sender<Vec<ItemState>>,
    admin_state_tx: &mut watch::Sender<AdminState>,
    sponsorship_state_tx: &mut watch::Sender<Vec<Sponsorship>>,
    sync_handle: AuctionSyncHandle,
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
        async move |pool: &SqlitePool,
                    sync_handle: &AuctionSyncHandle|
                    -> anyhow::Result<AdminState> {
            let holding_account_balance =
                query!("SELECT value FROM kv_data_int WHERE key='holding_balance'")
                    .fetch_one(pool)
                    .await?
                    .value;

            // To get the list of connected users, we'll go through the HashMap of connected handles,
            // and check whether each of those is still alive
            // (and if not, delete the entry)
            let connection_active_handles_ref = sync_handle.connection_active_handles.clone();
            let connected_users = {
                let mut connection_active_handles = connection_active_handles_ref.lock().await;
                connection_active_handles.retain(|_, weakref| weakref.upgrade().is_some());
                connection_active_handles.iter().map(|(k, _v)| *k).collect()
            };

            let state = AdminState {
                holding_account_balance: holding_account_balance as Money,
                connected_users,
            };
            Ok(state)
        }
    };

    admin_state_tx.send_replace(get_admin_state(&pool, &sync_handle).await?);

    sponsorship_state_tx.send_replace(get_sponsorship_state(pool).await?);

    let mut admin_data_refresh_interval = interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            // === Periodic updates ===
            // TODO!!!: check if this is really necessary
            _ = item_data_refresh_interval.tick() => {
                item_sale_state_tx.send_replace(get_item_state(pool).await?);
            },

            _ = user_data_refresh_interval.tick() => {
                auction_member_tx.send_replace(get_user_state(pool).await?);
            },

            // This one is definitely necessary: the admin state can change by external means (user connects/disconnects)
            _ = admin_data_refresh_interval.tick() => {
                    admin_state_tx.send_replace(get_admin_state(&pool, &sync_handle).await?);
            },



            // === IPC ===
            Some((key, sender)) = get_member_by_key_rx.recv() => {
                let user_row = query!("SELECT * FROM auction_user WHERE login_key=?", key).fetch_optional(pool).await?;
                sender.send(user_row.map(
                 |row| UserAccountDataWithSecrets {
                    id: row.id,
                    user_name: row.name,
                    balance: row.balance as Money,
                    sale_mode: row.sale_mode.into(),
                    login_key: row.login_key,
                    sponsorship_code: row.sponsorship_code,
                 }
             )).ignore();
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
                        running_auction_handle = tokio::spawn(run_english_auction(item_id, pool.clone(), english_rx.clone(), state_tx.clone(), sync_handle.clone()));
                    },
                    AuctionEvent::RunJapaneseAuction(item_id) => {
                        running_auction_handle.abort();
                        current_auction = Japanese;
                        running_auction_handle = tokio::spawn(run_japanese_auction(item_id, pool.clone(), japanese_rx.clone(), state_tx.clone(), sync_handle.clone()));
                    },

                    AuctionEvent::EnglishAuctionAction(action) => {
                        // If there is no English auction currently in progress, ignore this
                        if !matches!(current_auction, English) {continue;}
                        english_tx.send(action).await?;
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
                        let user_data = forget_user_secrets(get_user_state(pool).await?);

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
                                        query!("UPDATE auction_item SET initial_price=? WHERE id=?", price, id).execute(&mut tx).await?;
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
                                admin_state_tx.send_replace(get_admin_state(&pool, &sync_handle).await?);
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

                        admin_state_tx.send_replace(get_admin_state(&pool, &sync_handle).await?);
                        auction_member_tx.send_replace(get_user_state(pool).await?);

                    },

                    AuctionEvent::SetIsAcceptingSponsorships {user_id, is_accepting_sponsorships} => {
                        let maybe_sponsorship_code = is_accepting_sponsorships.then_some(gen_sponsorship_code(pool).await?);
                        query!("UPDATE auction_user SET sponsorship_code=? WHERE id=?", maybe_sponsorship_code, user_id).execute(pool).await?;
                        auction_member_tx.send_replace(get_user_state(pool).await?);
                    },
                    AuctionEvent::TryActivateSponsorshipCode { user_id, code } => {
                        // Try to find a user who has the given sponsorship code.
                        let maybe_user_row = query!("SELECT * FROM auction_user WHERE sponsorship_code=?", code).fetch_optional(pool).await?;
                        // If such a user does not exist, ignore.
                        let row = match maybe_user_row {
                            None => {continue;},
                            Some(r) => r,
                        };

                        // If it would be the same user as the donor, do not accept this.
                        if row.id == user_id { continue; }

                        // Get the donor's balance, which will be the initial sponsorship amount.
                        let balance = {
                            let users = auction_member_tx.borrow();
                            users.iter().find(|u| u.id == user_id).expect("User creating sponsorship does not exist?").balance
                        };

                        let mut tx = pool.begin().await?;

                        // Any other sponsorships from the donor to the recepient that are Active need to be changed to Retracted.
                        let retracted = SponsorshipStatus::Retracted.to_db_val();
                        let active = SponsorshipStatus::Active.to_db_val();
                        query!("UPDATE sponsorship SET status=? WHERE status=? AND donor_id=? AND recepient_id=?",
                            retracted, active,
                            user_id, row.id,
                        ).execute(&mut tx).await?;

                        // Create the sponsorship row
                        query!(
                            "INSERT INTO sponsorship (donor_id, recepient_id, status, remaining_balance) VALUES (?,?,?,?)",
                            user_id, row.id, 1 /*status=active*/, balance
                            ).execute(&mut tx).await?;

                        tx.commit().await?;

                        // Fetch current sponsorships
                        sponsorship_state_tx.send_replace(get_sponsorship_state(pool).await?);

                    },

                    AuctionEvent::UpdateSponsorship { actor_id, sponsorship_id, new_status, new_amount } => {
                        // Fetch the sponsorship. If it doesn't exist, ignore.
                        let mut sponsorship = {
                            let sponsorships = sponsorship_state_tx.borrow();
                            let mut s_iter = sponsorships.iter();
                            let s = s_iter.find(|s| s.id == sponsorship_id);
                            match s {
                                None => {continue;},
                                Some(s) => s.clone(),
                            }
                        };
                        let mut did_change = false;

                        // If I'm the recepient, I can change the status to Rejected.
                        if actor_id == sponsorship.recepient_id && new_status == Some(SponsorshipStatus::Rejected) {
                            sponsorship.status = SponsorshipStatus::Rejected;
                            did_change = true;
                        }

                        // If I'm the donor, I can change the balance to anything.
                        if let Some(b) = new_amount { if actor_id == sponsorship.donor_id {
                            sponsorship.balance_remaining = b;
                            did_change = true;
                        }}

                        // If I'm the donor, I can change the status to Retracted.
                        if actor_id == sponsorship.donor_id && new_status == Some(SponsorshipStatus::Retracted) {
                            sponsorship.status = SponsorshipStatus::Retracted;
                            did_change = true;
                        }


                        // If any of the changes were applied, persist them,
                        // then update the receivers.
                        if did_change {
                            let status = sponsorship.status.to_db_val();
                            query!("UPDATE sponsorship SET status=?, remaining_balance=? WHERE id=?",
                                status, sponsorship.balance_remaining, sponsorship.id)
                                .execute(pool).await?;
                            sponsorship_state_tx.send_replace(get_sponsorship_state(pool).await?);

                        }
                    },
                    AuctionEvent::SetSaleMode { user_id, sale_mode } => {
                        let sale_mode = u8::from(sale_mode);
                        query!("UPDATE auction_user SET sale_mode=? WHERE id=?", sale_mode, user_id).execute(pool).await?;
                        auction_member_tx.send_replace(get_user_state(pool).await?);
                    },
                    AuctionEvent::RegenerateSponsorshipCode { user_id } => {
                        // Get the user with the given ID.
                        let user = {
                            let users = auction_member_tx.borrow();
                            let mut u_iter = users.iter();
                            let u = u_iter.find(|u| u.id == user_id);
                            match u {
                                None => {continue;},
                                Some(u) => u.clone()
                            }
                        };

                        // If it doesn't have a sponsorship code, ignore it.
                        if user.sponsorship_code.is_none() {continue;}
                        // But if it does, make a new one
                        let new_code = gen_sponsorship_code(pool).await?;

                        query!("UPDATE auction_user SET sponsorship_code=? WHERE id=?",
                            new_code, user.id
                        ).execute(pool).await?;
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
