use std::{collections::HashMap, sync::Arc, time::Duration};

use communication::{
    auction::state::{AuctionItem, AuctionReport, AuctionState},
    ItemState, ItemStateValue, Money, UserAccountData,
};
use sqlx::{query, SqlitePool};
use tokio::sync::*;
use tracing::debug;

mod english;
mod japanese;
pub use english::*;
pub use japanese::*;

/// This struct holds the synchronization items needed to talk to the auction manager.
#[derive(Clone, Debug)]
pub struct AuctionSyncHandle {
    /// Stores info on the current set of auction members.
    pub auction_members: watch::Receiver<Vec<UserAccountData>>,

    /// Allows fetching member by their login key.
    /// Send in the login key, and a oneshot sender to get back the account data.
    get_member_by_key: mpsc::Sender<(String, oneshot::Sender<Option<UserAccountData>>)>,

    /// Allows getting a member by their ID.
    /// Send in the ID, and a oneshot sender to get back the account data.
    get_member_by_id: mpsc::Sender<(i64, oneshot::Sender<Option<UserAccountData>>)>,

    /// Stores info on the current auction state.
    pub auction_state: watch::Receiver<AuctionState>,

    /// Allows sending into the auction thread events that influence the auction.
    auction_event_sender: mpsc::Sender<AuctionEvent>,

    /// Stores info about the current state of item sales.
    pub item_sale_states: watch::Receiver<Vec<ItemState>>,

    /// Holds handles to ask the connection tasks to quit because somebody else is connecting.
    pub connection_drop_handles: Arc<Mutex<HashMap<i64, oneshot::Sender<()>>>>,
}

impl AuctionSyncHandle {
    /// Wrapper for the `get_member_by_key` process.
    ///
    /// If the user exists, it also returns a future
    /// that will resolve if `get_member_by_key` is called again with the same key.
    /// The user connection thread should use that as a signal to terminate the connection,
    /// to avoid race conditions between the two connections.
    pub async fn get_member_by_key(
        &self,
        key: String,
    ) -> Option<(UserAccountData, oneshot::Receiver<()>)> {
        // Make a oneshot channel
        let (tx, rx) = oneshot::channel();
        // Send it to the manager
        self.get_member_by_key
            .send((key, tx))
            .await
            .expect("Manager closed without receiving command to get member");

        let user = rx
            .await
            .expect("Manager closed without giving back member by key");
        match user {
            None => None,
            Some(user_data) => {
                // If we have a stored drop handle, call it.
                let mut handles = self.connection_drop_handles.lock().await;
                if let Some(sender) = handles.remove(&user_data.id) {
                    sender.send(()).ignore();
                }

                // Put a new handle into the drop handles.
                let (drop_tx, drop_rx) = oneshot::channel();
                handles.insert(user_data.id, drop_tx);

                Some((user_data, drop_rx))
            }
        }
    }

    /// Wrapper for the `get_member_by_key` process
    pub async fn get_member_by_id(&self, id: i64) -> Option<UserAccountData> {
        // Make a oneshot channel
        let (tx, rx) = oneshot::channel();
        // Send it to the manager
        self.get_member_by_id
            .send((id, tx))
            .await
            .expect("Manager closed without receiving command to get member");

        rx.await
            .expect("Manager closed without giving back member by ID")
    }

    /// Send an AuctionEvent into the auction process.
    pub async fn send_event(&self, event: AuctionEvent) {
        self.auction_event_sender
            .send(event)
            .await
            .expect("Auction thread is not running while sending AuctionEvent into it?!");
    }

    /// Initialize the auction manager with tokio::spawn, passing in the counterparts of the items in the struct,
    /// and create an instance of this struct.
    ///
    /// You should only call this once per program run.
    pub async fn new(pool: SqlitePool) -> Self {
        let (amtx, amrx) = watch::channel(vec![]);
        let (astx, asrx) = watch::channel(AuctionState::WaitingForAuction);
        let (isstx, issrx) = watch::channel(vec![]);
        let (gmbktx, gmbkrx) = mpsc::channel(100);
        let (gmbitx, gmbirx) = mpsc::channel(100);
        let (aetx, aerx) = mpsc::channel(100);
        tokio::spawn(auction_manager(
            pool, amtx, gmbkrx, gmbirx, astx, aerx, isstx,
        ));
        AuctionSyncHandle {
            auction_members: amrx,
            get_member_by_key: gmbktx,
            get_member_by_id: gmbitx,
            auction_state: asrx,
            auction_event_sender: aetx,
            item_sale_states: issrx,
            connection_drop_handles: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

async fn auction_manager(
    pool: SqlitePool,
    mut auction_member_tx: watch::Sender<Vec<UserAccountData>>,
    mut get_member_by_key_rx: mpsc::Receiver<(String, oneshot::Sender<Option<UserAccountData>>)>,
    mut get_member_by_id_rx: mpsc::Receiver<(i64, oneshot::Sender<Option<UserAccountData>>)>,
    mut auction_state_tx: watch::Sender<AuctionState>,
    mut auction_event_rx: mpsc::Receiver<AuctionEvent>,
    mut item_sale_state_tx: watch::Sender<Vec<ItemState>>,
) -> ! {
    loop {
        let result = auction_manager_inner(
            &pool,
            &mut auction_member_tx,
            &mut get_member_by_key_rx,
            &mut get_member_by_id_rx,
            &mut auction_state_tx,
            &mut auction_event_rx,
            &mut item_sale_state_tx,
        )
        .await;
        match result {
            Ok(_) => unreachable!(),
            Err(why) => eprintln!("Auction manager task closed with error: {why} {why:?}"),
        }
    }
}

trait Ignorable {
    fn ignore(self);
}

impl<T, E> Ignorable for Result<T, E> {
    fn ignore(self) {}
}

/// Represents events that can change the progress of the auction.
#[derive(Debug)]
pub enum AuctionEvent {
    /// An admin has requested that the auction enter the "waiting for item" state.
    StartAuction,

    /// An admin has requested that an item be selected for auctioning.
    PrepareAuctioning(i64),

    /// An admin has requested that an English auction be used to sell the given item.
    RunEnglishAuction(i64),

    /// An admin has requested that a Japanese auction be used to sell the given item.
    RunJapaneseAuction(i64),

    /// A user has placed a bid in an English auction that's in progress.
    BidInEnglishAuction {
        user_id: i64,
        item_id: i64,
        bid_amount: Money,
    },

    /// A user has entered or exited the Japanese auction's arena.
    JapaneseAuctionAction(JapaneseAuctionEvent),

    /// An admin has requested entering the "auction over" state
    FinishAuction,

    /// An admin has requested that the auction be started from the beginning.
    StartAuctionAnew,
}

async fn auction_manager_inner(
    pool: &SqlitePool,
    auction_member_tx: &mut watch::Sender<Vec<UserAccountData>>,
    get_member_by_key_rx: &mut mpsc::Receiver<(String, oneshot::Sender<Option<UserAccountData>>)>,
    get_member_by_id_rx: &mut mpsc::Receiver<(i64, oneshot::Sender<Option<UserAccountData>>)>,
    auction_state_tx: &mut watch::Sender<AuctionState>,
    auction_event_rx: &mut mpsc::Receiver<AuctionEvent>,
    item_sale_state_tx: &mut watch::Sender<Vec<ItemState>>,
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

    loop {
        tokio::select! {
            // === Periodic updates ===
            _ = item_data_refresh_interval.tick() => {
                let item_rows = query!(r#"
                    SELECT
                        auction_item.id, auction_item.name, auction_item.initial_price, auction_item_sale.buyer_id, auction_item_sale.sale_price, auction_user.name AS username, auction_user.balance
                    FROM auction_item
                    LEFT OUTER JOIN auction_item_sale ON auction_item_sale.item_id = auction_item.id
                    LEFT OUTER JOIN auction_user ON auction_item_sale.buyer_id = auction_user.id
                    "#).fetch_all(pool).await?;
                    let mut item_data = vec![];
                    for row in item_rows {
                        let item = AuctionItem { id: row.id, name: row.name, initial_price: row.initial_price as Money };
                        let state = match row.buyer_id {
                            None => ItemStateValue::Sellable,
                            Some(id) => ItemStateValue::AlreadySold { buyer: UserAccountData { id, user_name: row.username, balance: row.balance as Money }, sale_price: row.sale_price.unwrap() as Money },
                        };
                        item_data.push(ItemState {item, state});
                    }
                    item_sale_state_tx.send_replace(item_data);
            },

            _ = user_data_refresh_interval.tick() => {
                // Gather all user info, then apply it to the watcher
                let user_rows = query!("SELECT * FROM auction_user").fetch_all(pool).await?;
                let mut user_data = vec![];
                for row in user_rows {
                    user_data.push(UserAccountData { id: row.id, user_name: row.name, balance: row.balance as u32 });
                }
                auction_member_tx.send_replace(user_data.clone());
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
                        let user_rows = query!("SELECT * FROM auction_user").fetch_all(pool).await?;
                        let user_data = user_rows.into_iter().map(|row| UserAccountData { id: row.id, user_name: row.name, balance: row.balance as u32 }).collect();

                        // Then, collect the sale data
                        let item_rows = query!(r#"
                            SELECT
                                auction_item.id, auction_item.name, auction_item.initial_price, auction_item_sale.buyer_id, auction_item_sale.sale_price, auction_user.name AS username, auction_user.balance
                            FROM auction_item
                            LEFT OUTER JOIN auction_item_sale ON auction_item_sale.item_id = auction_item.id
                            LEFT OUTER JOIN auction_user ON auction_item_sale.buyer_id = auction_user.id
                            "#).fetch_all(pool).await?;
                        let mut item_data = vec![];
                        for row in item_rows {
                            let item = AuctionItem { id: row.id, name: row.name, initial_price: row.initial_price as Money };
                            let state = match row.buyer_id {
                                None => ItemStateValue::Sellable,
                                Some(id) => ItemStateValue::AlreadySold { buyer: UserAccountData { id, user_name: row.username, balance: row.balance as Money }, sale_price: row.sale_price.unwrap() as Money },
                            };
                            item_data.push(ItemState {item, state});
                        }


                        let report = AuctionReport { items: item_data, members: user_data };
                        auction_state_tx.send_replace(AuctionState::AuctionOver(report));
                    },

                    AuctionEvent::StartAuctionAnew => {
                        running_auction_handle.abort();
                        current_auction = NoAuction;
                        auction_state_tx.send_replace(AuctionState::WaitingForAuction);
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
