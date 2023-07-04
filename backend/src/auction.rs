use std::time::Duration;

use communication::{
    auction::state::{AuctionItem, AuctionItemSaleState, AuctionItemSaleStatus, AuctionState},
    Money, UserAccountData,
};
use sqlx::{query, SqlitePool};
use tokio::sync::*;
use tracing::debug;

/// This struct holds the synchronization items needed to talk to the auction manager.
#[derive(Clone, Debug)]
pub struct AuctionSyncHandle {
    /// Stores info on the current set of auction members.
    pub auction_members: watch::Receiver<Vec<UserAccountData>>,

    /// Allows fetching member by their login key.
    /// Send in the login key, and a oneshot sender to get back the account data.
    get_member_by_key: mpsc::Sender<(String, oneshot::Sender<Option<UserAccountData>>)>,

    /// Stores info on the current auction state.
    pub auction_state: watch::Receiver<AuctionState>,

    /// Allows sending into the auction thread events that influence the auction.
    auction_event_sender: mpsc::Sender<AuctionEvent>,

    /// Stores info about the current state of item sales.
    pub item_sale_states: watch::Receiver<Vec<AuctionItemSaleState>>,
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
        let (aetx, aerx) = mpsc::channel(100);
        tokio::spawn(auction_manager(pool, amtx, gmbkrx, astx, aerx, isstx));
        AuctionSyncHandle {
            auction_members: amrx,
            get_member_by_key: gmbktx,
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
    mut auction_state_tx: watch::Sender<AuctionState>,
    mut auction_event_rx: mpsc::Receiver<AuctionEvent>,
    mut item_sale_state_tx: watch::Sender<Vec<AuctionItemSaleState>>,
) -> ! {
    loop {
        let result = auction_manager_inner(
            &pool,
            &mut auction_member_tx,
            &mut get_member_by_key_rx,
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
    fn ignore(self) -> ();
}

impl<T, E> Ignorable for Result<T, E> {
    fn ignore(self) -> () {}
}

/// Represents events that can change the progress of the auction.
#[derive(Debug)]
pub enum AuctionEvent {
    /// An admin has requested that the auction enter the "waiting for item" state.
    StartAuction,
}

async fn auction_manager_inner(
    pool: &SqlitePool,
    auction_member_tx: &mut watch::Sender<Vec<UserAccountData>>,
    get_member_by_key_rx: &mut mpsc::Receiver<(String, oneshot::Sender<Option<UserAccountData>>)>,
    auction_state_tx: &mut watch::Sender<AuctionState>,
    auction_event_rx: &mut mpsc::Receiver<AuctionEvent>,
    item_sale_state_tx: &mut watch::Sender<Vec<AuctionItemSaleState>>,
) -> anyhow::Result<()> {
    let mut user_data_refresh_interval = tokio::time::interval(Duration::from_secs(1));

    let mut item_data_refresh_interval = tokio::time::interval(Duration::from_secs(5));

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
                        let status = match row.buyer_id {
                            None => AuctionItemSaleStatus::Unsold,
                            Some(id) => AuctionItemSaleStatus::SoldToMember { member: UserAccountData { id: id, user_name: row.username, balance: row.balance as Money }, sale_price: row.sale_price.unwrap() as Money },
                        };
                        item_data.push(AuctionItemSaleState {item, status});
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
                sender.send(user_row.map(|row| UserAccountData { id: row.id, user_name: row.name, balance: row.balance as u32 })).ignore();
            },

            Some(event) = auction_event_rx.recv() => {
                debug!("Received auction event: {event:?}");
                match event {
                    AuctionEvent::StartAuction => {
                        // Switch to auction state = waiting
                        auction_state_tx.send_replace(AuctionState::WaitingForItem);
                    },
                }
            }
        }
    }
}
