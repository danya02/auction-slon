use std::{sync::Arc, time::Duration};

use communication::{
    auction::{
        actions::JapaneseAuctionAction,
        state::{
            ActiveBidState, AuctionItem, AuctionReport, AuctionState, BiddingState,
            JapaneseAuctionBidState,
        },
    },
    ItemState, ItemStateValue, Money, UserAccountData,
};
use rand::Rng;
use sqlx::{query, SqlitePool};
use tokio::{
    sync::*,
    time::{interval, Instant},
};
use tracing::{debug, warn};

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
}

impl AuctionSyncHandle {
    /// Wrapper for the `get_member_by_key` process
    pub async fn get_member_by_key(&self, key: String) -> Option<UserAccountData> {
        // Make a oneshot channel
        let (tx, rx) = oneshot::channel();
        // Send it to the manager
        self.get_member_by_key
            .send((key, tx))
            .await
            .expect("Manager closed without receiving command to get member");

        rx.await
            .expect("Manager closed without giving back member by key")
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

#[derive(Debug)]
enum EnglishAuctionEvent {
    BidPlaced {
        bidder_id: i64,
        bid_amount: Money,
        item_id: i64,
    },
}

async fn run_english_auction(
    item_id: i64,
    pool: SqlitePool,
    rx: Arc<Mutex<mpsc::Receiver<EnglishAuctionEvent>>>,
    state_tx: mpsc::Sender<AuctionState>,
) -> anyhow::Result<()> {
    let pool = &pool;
    let mut rx = rx.lock().await;

    let row = query!(
        r#"
    SELECT
        auction_item.id, auction_item.name, auction_item.initial_price
    FROM auction_item
    WHERE id=?
    "#,
        item_id
    )
    .fetch_one(pool)
    .await?;
    let item = AuctionItem {
        id: row.id,
        name: row.name,
        initial_price: row.initial_price as Money,
    };

    let mut time_when_bidding_over = Instant::now() + Duration::from_secs(30); // initial time is higher than regular time
    let mut check_interval = interval(Duration::from_millis(100));

    let mut current_bid = item.initial_price - 1;
    let mut current_bidder = UserAccountData {
        id: 0,
        user_name: String::from("∅"), // null symbol U+2205
        balance: 0,
    };
    let mut current_bidder_id = 0;

    loop {
        // First check if the bidding has expired
        if time_when_bidding_over < Instant::now() {
            // Bidding over: item is sold

            // Special case: when nobody placed any bids (bidder_id=0),
            // just return to the item selection state
            // The auction admin can then try to re-sell the item.
            if current_bidder_id == 0 {
                state_tx.send(AuctionState::WaitingForAuction).await?;
                return Ok(());
            }

            // Update the database
            let mut tx = pool.begin().await?;
            query!(
                "INSERT INTO auction_item_sale(item_id, buyer_id, sale_price) VALUES (?,?,?)",
                item_id,
                current_bidder_id,
                current_bid
            )
            .execute(&mut tx)
            .await?;
            query!(
                "UPDATE auction_user SET balance=balance-? WHERE id=?",
                current_bid,
                current_bidder_id
            )
            .execute(&mut tx)
            .await?;
            tx.commit().await?;

            // Publish the state
            let mut confirmation_code = String::new();
            {
                let mut rng = rand::thread_rng();
                for _ in 0..4 {
                    confirmation_code.push_str(&rng.gen_range(0..9).to_string());
                }
            }

            let current_bidder_row =
                query!("SELECT * FROM auction_user WHERE id=?", current_bidder_id)
                    .fetch_one(pool)
                    .await?;
            let current_bidder = UserAccountData {
                id: current_bidder_row.id,
                user_name: current_bidder_row.name,
                balance: current_bidder_row.balance as Money,
            };
            let state = AuctionState::SoldToMember {
                item,
                sold_for: current_bid,
                sold_to: current_bidder,
                confirmation_code,
            };
            state_tx.send(state).await?;

            return Ok(());
        }

        tokio::select! {
            _ = check_interval.tick() => {
                // Construct an AuctionState and publish it
                let bid_state = BiddingState {
                    item: item.clone(),
                    active_bid: ActiveBidState::EnglishAuctionBid {
                        current_bid_amount: current_bid,
                        current_bidder: current_bidder.clone(),
                        minimum_increment: 1, // TODO: add rule for increasing this
                        seconds_until_commit: time_when_bidding_over.duration_since(Instant::now()).as_secs_f32(),
                    }
                };
                let state = AuctionState::Bidding(bid_state);
                state_tx.send(state).await?;
            }
            Some(event) = rx.recv() => {
                match event {
                    EnglishAuctionEvent::BidPlaced { bidder_id, bid_amount, item_id } => {
                        // If we receive an event about an item that is not the one we're selling, ignore it.
                        if item_id != item.id {continue;}

                        // Reset the check_interval, so that any bid changes are immediately propagated.
                        check_interval.reset();

                        // Retrieve the data for the new bidder
                        let row = query!("SELECT * FROM auction_user WHERE id=?", bidder_id).fetch_optional(pool).await?;
                        match row {
                            None => {
                                warn!("Received English auction bid with user ID={bidder_id} and bid_amount={bid_amount}; no such user: hacking detected?");
                                continue;
                            },
                            Some(row) => {
                                // If the user does not have sufficient funds, ignore the request
                                if (row.balance as Money) < bid_amount {
                                    warn!("Received English auction bid with user ID={bidder_id} and bid_amount={bid_amount}; user only has funds {}: hacking detected?", row.balance);
                                    continue;
                                }
                                // TODO: verify that the increment rule is followed

                                // Now record the bid
                                current_bid = bid_amount;
                                current_bidder_id = row.id;
                                current_bidder = UserAccountData { id: row.id, user_name: row.name, balance: row.balance as Money };

                                // and reset the timer
                                time_when_bidding_over = Instant::now() + Duration::from_secs(10);
                            }
                        };
                    },
                }
            },
        }
    }
}

#[derive(Debug)]
pub enum JapaneseAuctionEvent {
    /// A user has decided to either enter or leave the auction arena.
    UserAction {
        user_id: i64,
        item_id: i64,
        action: JapaneseAuctionAction,
    },
}

async fn run_japanese_auction(
    item_id: i64,
    pool: SqlitePool,
    rx: Arc<Mutex<mpsc::Receiver<JapaneseAuctionEvent>>>,
    state_tx: mpsc::Sender<AuctionState>,
) -> anyhow::Result<()> {
    let pool = &pool;
    let mut rx = rx.lock().await;

    let row = query!(
        r#"
    SELECT
        auction_item.id, auction_item.name, auction_item.initial_price
    FROM auction_item
    WHERE id=?
    "#,
        item_id
    )
    .fetch_one(pool)
    .await?;
    let item = AuctionItem {
        id: row.id,
        name: row.name,
        initial_price: row.initial_price as Money,
    };

    let mut current_price = item.initial_price;

    // TODO: figure out how best to change this interval
    let mut price_increase_interval = tokio::time::interval(Duration::from_secs(1));

    // This interval is used so that the `tokio::select!` does not get stuck for too long,
    // so that the arena closing logic can process,
    // and it also sends redundant copies of the auction state
    let mut update_interval = tokio::time::interval(Duration::from_millis(100));

    let mut arena = vec![];
    let arena_closes_for_entry = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut arena_is_closed = false;

    // This returns an Err when the item is successfully sold.
    // Just call this with ? whenever arena changes.
    async fn run_sold_check(
        arena_is_closed: bool,
        current_price: u32,
        arena: &mut Vec<UserAccountData>,
        state_tx: &mpsc::Sender<AuctionState>,
        pool: &SqlitePool,
        item: &AuctionItem,
    ) -> anyhow::Result<()> {
        let e = Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Error to set Japanese auction to over",
        ));
        // If the arena is closed, and has 0 members, then item cannot be sold. Resetting auction.
        if arena_is_closed {
            if arena.is_empty() {
                state_tx.send(AuctionState::WaitingForItem).await?;
                return e?;
            }
            // If the arena has 1 member, then that is who won the auction.
            //
            // NOTE: when multiple members have the same balance, and the money clock exceeds that balance,
            // the members are removed from the arena one by one, and this function is run at each step.
            // That way, there will be one definite winner.
            // That winner pays the value on the money clock,
            // or their total balance if it is smaller
            // This may undercount the item price by at most 1,
            // and ensures that no balance is negative.

            if arena.len() == 1 {
                let winner: &UserAccountData = arena.first().unwrap();
                let winner_pays = current_price.min(winner.balance);

                // Update the database
                let mut tx = pool.begin().await?;
                query!(
                    "INSERT INTO auction_item_sale(item_id, buyer_id, sale_price) VALUES (?,?,?)",
                    item.id,
                    winner.id,
                    winner_pays
                )
                .execute(&mut tx)
                .await?;
                query!(
                    "UPDATE auction_user SET balance=balance-? WHERE id=?",
                    winner_pays,
                    winner.id,
                )
                .execute(&mut tx)
                .await?;
                tx.commit().await?;

                // Publish the state
                let mut confirmation_code = String::new();
                {
                    let mut rng = rand::thread_rng();
                    for _ in 0..4 {
                        confirmation_code.push_str(&rng.gen_range(0..9).to_string());
                    }
                }

                let current_bidder_row = query!("SELECT * FROM auction_user WHERE id=?", winner.id)
                    .fetch_one(pool)
                    .await?;
                let current_bidder = UserAccountData {
                    id: current_bidder_row.id,
                    user_name: current_bidder_row.name,
                    balance: current_bidder_row.balance as Money,
                };
                let state = AuctionState::SoldToMember {
                    item: item.clone(),
                    sold_for: winner_pays,
                    sold_to: current_bidder,
                    confirmation_code,
                };
                state_tx.send(state).await?;
                return e?;
            }
        }

        Ok(())
    }

    loop {
        // If the arena is currently open, but it's past closing time, close it.
        if !arena_is_closed
            && arena_closes_for_entry
                .duration_since(Instant::now())
                .is_zero()
        {
            arena_is_closed = true;
            // Also, tell the system about this
            let bid_state = JapaneseAuctionBidState::ClockRunning {
                currently_in_arena: arena.clone(),
                current_price,
            };
            state_tx
                .send(AuctionState::Bidding(BiddingState {
                    item: item.clone(),
                    active_bid: ActiveBidState::JapaneseAuctionBid(bid_state),
                }))
                .await?;
        }

        run_sold_check(
            arena_is_closed,
            current_price,
            &mut arena,
            &state_tx,
            pool,
            &item,
        )
        .await?;

        tokio::select! {
            Some(event) = rx.recv() => {
                match event {
                    JapaneseAuctionEvent::UserAction { user_id, item_id, action } => {
                        // If the item ID does not match, ignore this
                        if item_id != item.id { continue; }
                        match action {
                            JapaneseAuctionAction::EnterArena => {
                                // If the arena is closed, ignore this.
                                if arena_is_closed {continue;}
                                let row = query!("SELECT * FROM auction_user WHERE id=?", user_id).fetch_optional(pool).await?;
                                let row = match row {
                                    None => {warn!("User ID {user_id} tried to enter Japanese arena, but does not exist; hacking detected?"); continue;}
                                    Some(row) => row,
                                };
                                let user = UserAccountData { id: row.id, user_name: row.name, balance: row.balance as u32 };
                                arena.push(user);

                                // Publish the current state (price, mode and arena members)
                                let bid_state = if arena_is_closed {
                                    JapaneseAuctionBidState::ClockRunning { currently_in_arena: arena.clone(), current_price }
                                } else {
                                    JapaneseAuctionBidState::EnterArena { currently_in_arena: arena.clone(), seconds_until_arena_closes: arena_closes_for_entry.duration_since(Instant::now()).as_secs_f32() }
                                };
                                state_tx.send(AuctionState::Bidding(BiddingState { item: item.clone(), active_bid: ActiveBidState::JapaneseAuctionBid(bid_state) })).await?;

                            },
                            JapaneseAuctionAction::ExitArena => {
                                // Remove the user from the arena, regardless of whether it's in there or not.
                                arena.retain(|u| u.id != user_id);
                                run_sold_check(arena_is_closed, current_price, &mut arena, &state_tx, pool, &item).await?;

                                // Publish the current state (price, mode and arena members)
                                let bid_state = if arena_is_closed {
                                    JapaneseAuctionBidState::ClockRunning { currently_in_arena: arena.clone(), current_price }
                                } else {
                                    JapaneseAuctionBidState::EnterArena { currently_in_arena: arena.clone(), seconds_until_arena_closes: arena_closes_for_entry.duration_since(Instant::now()).as_secs_f32() }
                                };
                                state_tx.send(AuctionState::Bidding(BiddingState { item: item.clone(), active_bid: ActiveBidState::JapaneseAuctionBid(bid_state) })).await?;

                            },
                        }
                    },
                }
            }

            _ = price_increase_interval.tick() => {
                // The price only increases when the arena is closed.
                if !arena_is_closed {continue;}

                current_price += 1;
                // TODO: apply some kind of transformation to `price_increase_interval`

                // Remove members from the arena who have less than the money clock in their balance,
                // in reverse order of the array, and check for the winner every time.
                // This ensures that the member who entered first is the winner.
                while let Some(member) = arena.iter().rev().find(|i| i.balance < current_price) {
                    let id = member.id;
                    arena.retain(|i| i.id != id);
                    run_sold_check(arena_is_closed, current_price, &mut arena, &state_tx, pool, &item).await?;
                }

                // Publish the current auction state.
                // It is ClockRunning, because we are increasing the price.
                let bid_state = JapaneseAuctionBidState::ClockRunning { currently_in_arena: arena.clone(), current_price };

                state_tx.send(AuctionState::Bidding(BiddingState { item: item.clone(), active_bid: ActiveBidState::JapaneseAuctionBid(bid_state) })).await?;


            }

            _ = update_interval.tick() => {
                // Publish the current state (price, mode and arena members)
                // ONLY IF the arena is currently open -> arena closing timer is counting down
                // (if the arena is closed, this is handled in the price_increase_interval tick, where we send a message on every price change)
                if !arena_is_closed {
                    let bid_state = JapaneseAuctionBidState::EnterArena { currently_in_arena: arena.clone(), seconds_until_arena_closes: arena_closes_for_entry.duration_since(Instant::now()).as_secs_f32() };
                    state_tx.send(AuctionState::Bidding(BiddingState { item: item.clone(), active_bid: ActiveBidState::JapaneseAuctionBid(bid_state) })).await?;
                }

            }
        }
    }
}
