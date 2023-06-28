use communication::UserAccountData;
use tokio::sync::*;

/// This struct holds the synchronization items needed to talk to the auction manager.
#[derive(Clone, Debug)]
pub struct AuctionSyncHandle {
    /// Stores info on the current set of auction members.
    pub auction_members: watch::Receiver<Vec<UserAccountData>>,

    /// Allows fetching member by their login key.
    /// Send in the login key, and a oneshot sender to get back the account data.
    get_member_by_key: mpsc::Sender<(String, oneshot::Sender<Option<UserAccountData>>)>,
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

    /// Initialize the auction manager with tokio::spawn, passing in the counterparts of the items in the struct,
    /// and create an instance of this struct.
    ///
    /// You should only call this once per program run.
    pub async fn new() -> Self {
        let (amtx, amrx) = watch::channel(vec![]);
        let (gmbktx, gmbkrx) = mpsc::channel(100);
        tokio::spawn(auction_manager(amtx, gmbkrx));
        AuctionSyncHandle {
            auction_members: amrx,
            get_member_by_key: gmbktx,
        }
    }
}

async fn auction_manager(
    mut auction_member_tx: watch::Sender<Vec<UserAccountData>>,
    mut get_member_by_key_rx: mpsc::Receiver<(String, oneshot::Sender<Option<UserAccountData>>)>,
) -> ! {
    loop {
        tokio::select! {
            Some((key, sender)) = get_member_by_key_rx.recv() => {
                // TODO: real lookup logic
                if key == "user-pw" {
                    sender.send(Some(UserAccountData { user_name: "Test user".to_string(), balance: 0 })).unwrap();
                } else {
                    sender.send(None).unwrap();
                }
            }
        }
    }
}
