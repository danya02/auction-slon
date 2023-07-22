use std::{sync::Arc, time::Duration};

use communication::{
    auction::state::{ActiveBidState, AuctionItem, AuctionState, BiddingState, Sponsorship},
    forget_user_secrets, Money, UserAccountData,
};
use rand::Rng;
use sqlx::{query, SqlitePool};
use tokio::{
    sync::*,
    time::{interval, Instant},
};
use tracing::warn;

use crate::auction::db_actions::{apply_contributions, get_sponsorship_state, get_user_state};

use super::sync_handle;

#[derive(Debug)]
pub enum EnglishAuctionEvent {
    BidPlaced {
        bidder_id: i64,
        bid_amount: Money,
        item_id: i64,
    },

    /// Change the time until the bid is committed
    SetCommitPeriod { new_period: Duration },
}

pub async fn run_english_auction(
    item_id: i64,
    pool: SqlitePool,
    rx: Arc<Mutex<mpsc::Receiver<EnglishAuctionEvent>>>,
    state_tx: mpsc::Sender<AuctionState>,
    mut sync_handle: sync_handle::AuctionSyncHandle,
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

    let mut bidding_duration = Duration::from_secs(15);
    let mut time_when_bidding_over = Instant::now() + Duration::from_secs(u64::MAX / 8); // initial time is basically infinite, but needs to be inside the allowable range.
    let mut check_interval = interval(Duration::from_millis(100));

    let mut current_bid = item.initial_price - 1;
    let mut current_bidder = UserAccountData {
        id: 0,
        user_name: String::from("∅"), // null symbol U+2205
        balance: 0,
        sale_mode: communication::UserSaleMode::Bidding,
        is_accepting_sponsorships: false,
    };
    let mut current_bidder_id = 0;
    let mut bid_history = vec![];
    bid_history.push((current_bidder_id, item.initial_price - 1));

    loop {
        // First check if the bidding has expired
        if time_when_bidding_over < Instant::now() {
            // Bidding over: item is sold

            // Special case: when nobody placed any bids (bidder_id=0),
            // just return to the item selection state
            // The auction admin can then try to re-sell the item.
            if current_bidder_id == 0 {
                state_tx.send(AuctionState::WaitingForItem).await?;
                return Ok(());
            }

            // Fetch the latest states of users and sponsorships: important so that the info is not outdated.
            let users = get_user_state(pool).await?;
            let sponsorships = get_sponsorship_state(pool).await?;

            let current_bidder = users
                .iter()
                .find(|u| u.id == current_bidder_id)
                .expect("Final bidder not in users?");

            let contributions = Sponsorship::calculate_contributions(
                current_bidder_id,
                current_bid,
                &forget_user_secrets(users.clone()),
                &sponsorships,
            );

            let contributions_ids: Vec<_> = contributions.iter().map(|(u, b)| (u.id, *b)).collect();

            apply_contributions(pool, item_id, current_bidder_id, &contributions_ids).await?;

            // Publish the state
            let mut confirmation_code = String::new();
            {
                let mut rng = rand::thread_rng();
                for _ in 0..4 {
                    confirmation_code.push_str(&rng.gen_range(0..9).to_string());
                }
            }

            let state = AuctionState::SoldToMember {
                item,
                sold_for: current_bid,
                sold_to: current_bidder.into(),
                confirmation_code,
                contributions,
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
                        max_millis_until_commit: bidding_duration.as_millis()
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
                        //check_interval.reset();

                        // Retrieve the data for the new bidder
                        let row = query!("SELECT * FROM auction_user WHERE id=?", bidder_id).fetch_optional(pool).await?;
                        match row {
                            None => {
                                warn!("Received English auction bid with user ID={bidder_id} and bid_amount={bid_amount}; no such user: hacking detected?");
                                continue;
                            },
                            Some(row) => {
                                // Get the amount that the user's sponsorship group has access to.
                                let users = forget_user_secrets(sync_handle.auction_members.borrow().clone());
                                let sponsorships = sync_handle.sponsorship_state.borrow();
                                println!("{users:?}");
                                println!("{sponsorships:?}");
                                let accessible_amount = Sponsorship::resolve_available_balance(row.id, &users, &sponsorships);

                                // If the user does not have sufficient funds, ignore the request
                                if accessible_amount < bid_amount {
                                    warn!("Received English auction bid with user ID={bidder_id} and bid_amount={bid_amount}; user only has funds {}: hacking detected?", accessible_amount);
                                    continue;
                                }
                                // TODO: verify that the increment rule is followed
                                // For now, just verify that the new bid is greater than the past one.
                                if bid_amount <= current_bid { continue; }

                                // Now record the bid
                                current_bid = bid_amount;
                                current_bidder_id = row.id;
                                current_bidder = UserAccountData {
                                    id: row.id, user_name: row.name, balance: row.balance as Money,
                                    sale_mode: row.sale_mode.into(),
                                    is_accepting_sponsorships: row.sponsorship_code.is_some(),
                                };
                                bid_history.push((current_bidder_id, current_bid));

                                // and reset the timer
                                time_when_bidding_over = Instant::now() + bidding_duration;
                            }
                        };
                    },
                    EnglishAuctionEvent::SetCommitPeriod{ new_period } => {
                        // If the new duration is longer than the previous one,
                        // shift the timer deadline by that.
                        if new_period > bidding_duration {
                            let diff = new_period - bidding_duration;
                            time_when_bidding_over = time_when_bidding_over + diff;
                        }

                        bidding_duration = new_period;
                    },
                }
            },

            _ = sync_handle.sponsorship_state.changed() => {
                // If sponsorships have changed, then the current bid may have become invalid.
                // Unwind the bids history, checking each along the way.
                // For each bid that turns out to be invalid, discard it,
                // until we come up to the null bid.

                let users = forget_user_secrets(sync_handle.auction_members.borrow().clone());
                let sponsorships = sync_handle.sponsorship_state.borrow().clone();

                loop {
                    // Check if the current bid is still allowable:
                    let accessible_amount = Sponsorship::resolve_available_balance(current_bidder_id, &users, &sponsorships);
                    if current_bid > accessible_amount {
                        // The current bid is not allowed
                        // Remove it from the stack,
                        // then apply the one following it.
                        bid_history.pop();
                        let to_apply =  bid_history.last().unwrap();
                        current_bid = to_apply.1;
                        current_bidder_id = to_apply.0;
                        // Reset the timer
                        time_when_bidding_over = Instant::now() + bidding_duration;

                        // Special case: if the only bid remaining is the null bid, restore the auction to its initial state,
                        // and stop the unwinding.
                        if to_apply.0 == 0 {
                            current_bidder = UserAccountData {
                                id: 0,
                                user_name: String::from("∅"), // null symbol U+2205
                                balance: 0,
                                sale_mode: communication::UserSaleMode::Bidding,
                                is_accepting_sponsorships: false,
                            };
                            time_when_bidding_over = Instant::now() + Duration::from_secs(u64::MAX / 8);
                            break;
                        }

                        // If the bid is by a normal user,
                        // then apply the user's current state (which may have changed).
                        current_bidder = users.iter().find(|u| u.id == current_bidder_id).expect("User disappeared during English auction?").clone();
                    } else {
                        // The current bid is acceptable, so stop the unwinding.
                        // (This case also happens when the very first bid is acceptable,
                        //  and no unwinding actually took place.)
                        break;
                    }
                }
            },
        }
    }
}
