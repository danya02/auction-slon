use std::{collections::HashMap, sync::Arc};

use communication::{
    admin_state::AdminState,
    auction::state::{AuctionState, Sponsorship},
    ItemState, UserAccountDataWithSecrets,
};
use sqlx::SqlitePool;
use tokio::sync::*;

use crate::Ignorable;

use super::{auction_manager_inner, AuctionEvent};

/// This struct holds the synchronization items needed to talk to the auction manager.
#[derive(Clone, Debug)]
pub struct AuctionSyncHandle {
    /// Stores info on the current set of auction members.
    pub auction_members: watch::Receiver<Vec<UserAccountDataWithSecrets>>,

    /// Allows fetching member by their login key.
    /// Send in the login key, and a oneshot sender to get back the account data.
    get_member_by_key: mpsc::Sender<(String, oneshot::Sender<Option<UserAccountDataWithSecrets>>)>,

    /// Stores info on the current auction state.
    pub auction_state: watch::Receiver<AuctionState>,

    /// Allows sending into the auction thread events that influence the auction.
    auction_event_sender: mpsc::Sender<AuctionEvent>,

    /// Stores info about the current state of item sales.
    pub item_sale_states: watch::Receiver<Vec<ItemState>>,

    /// Holds handles to ask the connection tasks to quit because somebody else is connecting.
    pub connection_drop_handles: Arc<Mutex<HashMap<i64, oneshot::Sender<()>>>>,

    /// Stores the state that the admin can use
    pub admin_state: watch::Receiver<AdminState>,

    /// Holds the sponsorships currently in the database.
    /// No processing is applied to these. Figure it out yourself.
    pub sponsorship_state: watch::Receiver<Vec<Sponsorship>>,
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
    ) -> Option<(UserAccountDataWithSecrets, oneshot::Receiver<()>)> {
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
        let (adstx, adsrx) = watch::channel(AdminState {
            holding_account_balance: 0,
        });
        let (sptx, sprx) = watch::channel(vec![]);

        let sync_handle = AuctionSyncHandle {
            auction_members: amrx,
            get_member_by_key: gmbktx,
            auction_state: asrx,
            auction_event_sender: aetx,
            item_sale_states: issrx,
            admin_state: adsrx,
            connection_drop_handles: Arc::new(Mutex::new(HashMap::new())),
            sponsorship_state: sprx,
        };

        tokio::spawn(auction_manager(
            pool,
            amtx,
            gmbkrx,
            astx,
            aerx,
            isstx,
            adstx,
            sptx,
            sync_handle.clone(),
        ));
        sync_handle
    }
}

async fn auction_manager(
    pool: SqlitePool,
    mut auction_member_tx: watch::Sender<Vec<UserAccountDataWithSecrets>>,
    mut get_member_by_key_rx: mpsc::Receiver<(
        String,
        oneshot::Sender<Option<UserAccountDataWithSecrets>>,
    )>,
    mut auction_state_tx: watch::Sender<AuctionState>,
    mut auction_event_rx: mpsc::Receiver<AuctionEvent>,
    mut item_sale_state_tx: watch::Sender<Vec<ItemState>>,
    mut admin_state_tx: watch::Sender<AdminState>,
    mut sponsorship_state: watch::Sender<Vec<Sponsorship>>,
    sync_handle: AuctionSyncHandle,
) -> () {
    loop {
        let result = auction_manager_inner(
            &pool,
            &mut auction_member_tx,
            &mut get_member_by_key_rx,
            &mut auction_state_tx,
            &mut auction_event_rx,
            &mut item_sale_state_tx,
            &mut admin_state_tx,
            &mut sponsorship_state,
            sync_handle.clone(),
        )
        .await;
        match result {
            Ok(_) => unreachable!(),
            Err(why) => {
                eprintln!("Auction manager task closed with error: {why} {why:?}");
                // If it's because the pool is closed, then we need to exit
                if pool.is_closed() {
                    return;
                }
            }
        }
    }
}
