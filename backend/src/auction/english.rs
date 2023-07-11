use std::{sync::Arc, time::Duration};

use communication::{
    auction::state::{ActiveBidState, AuctionItem, AuctionState, BiddingState},
    Money, UserAccountData,
};
use rand::Rng;
use sqlx::{query, SqlitePool};
use tokio::{
    sync::*,
    time::{interval, Instant},
};
use tracing::warn;

#[derive(Debug)]
pub enum EnglishAuctionEvent {
    BidPlaced {
        bidder_id: i64,
        bid_amount: Money,
        item_id: i64,
    },
}

pub async fn run_english_auction(
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
