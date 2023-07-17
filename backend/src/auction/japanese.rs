use std::{sync::Arc, time::Duration};

use communication::{
    auction::{
        actions::JapaneseAuctionAction,
        state::{
            ActiveBidState, ArenaVisibilityMode, AuctionItem, AuctionState, BiddingState,
            JapaneseAuctionBidState,
        },
    },
    Money, UserAccountData,
};
use rand::Rng;
use sqlx::{query, SqlitePool};
use tokio::{
    sync::*,
    time::{interval_at, Instant},
};
use tracing::warn;

#[derive(Debug)]
pub enum JapaneseAuctionEvent {
    /// A user has decided to either enter or leave the auction arena.
    UserAction {
        user_id: i64,
        item_id: i64,
        action: JapaneseAuctionAction,
    },

    /// An admin has requested that the price increase interval be changed.
    NewPriceClockInterval {
        price_increase_per_100_seconds: Money,
    },

    /// An admin has changed the arena visibility mode.
    NewArenaVisibilityMode(ArenaVisibilityMode),
}

pub async fn run_japanese_auction(
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

    let mut current_price_increase_per_100_seconds = 100;
    let mut price_increase_interval = tokio::time::interval(Duration::from_secs_f32(
        100.0 / current_price_increase_per_100_seconds as f32,
    ));

    // This interval is used so that the `tokio::select!` does not get stuck for too long,
    // so that the arena closing logic can process,
    // and it also sends redundant copies of the auction state
    // when the arena is open, thus updating the timer.
    let mut update_interval = tokio::time::interval(Duration::from_millis(100));

    let mut arena = vec![];
    let arena_closes_for_entry = tokio::time::Instant::now() + Duration::from_secs(15);
    let mut arena_is_closed = false;

    let mut arena_visibility_mode = ArenaVisibilityMode::Full;

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
            "This error value is used to exit the Japanese auction loop, and is expected",
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
                current_price_increase_per_100_seconds,
                arena_visibility_mode,
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
                                // If the user with this ID is already in the arena, ignore this.
                                if arena.iter().any(|i| i.id == user_id) {continue;}
                                let row = query!("SELECT * FROM auction_user WHERE id=?", user_id).fetch_optional(pool).await?;
                                let row = match row {
                                    None => {warn!("User ID {user_id} tried to enter Japanese arena, but does not exist; hacking detected?"); continue;}
                                    Some(row) => row,
                                };
                                let user = UserAccountData { id: row.id, user_name: row.name, balance: row.balance as u32 };
                                arena.push(user);

                                // Publish the current state (price, mode and arena members)
                                let bid_state = if arena_is_closed {
                                    JapaneseAuctionBidState::ClockRunning { currently_in_arena: arena.clone(), current_price, current_price_increase_per_100_seconds, arena_visibility_mode }
                                } else {
                                    JapaneseAuctionBidState::EnterArena { currently_in_arena: arena.clone(), seconds_until_arena_closes: arena_closes_for_entry.duration_since(Instant::now()).as_secs_f32(), current_price, current_price_increase_per_100_seconds, arena_visibility_mode }
                                };
                                state_tx.send(AuctionState::Bidding(BiddingState { item: item.clone(), active_bid: ActiveBidState::JapaneseAuctionBid(bid_state) })).await?;

                            },
                            JapaneseAuctionAction::ExitArena => {
                                // Remove the user from the arena, regardless of whether it's in there or not.
                                arena.retain(|u| u.id != user_id);
                                run_sold_check(arena_is_closed, current_price, &mut arena, &state_tx, pool, &item).await?;

                                // Publish the current state (price, mode and arena members)
                                let bid_state = if arena_is_closed {
                                    JapaneseAuctionBidState::ClockRunning { currently_in_arena: arena.clone(), current_price, current_price_increase_per_100_seconds, arena_visibility_mode }
                                } else {
                                    JapaneseAuctionBidState::EnterArena { currently_in_arena: arena.clone(), seconds_until_arena_closes: arena_closes_for_entry.duration_since(Instant::now()).as_secs_f32(), current_price, current_price_increase_per_100_seconds, arena_visibility_mode }
                                };
                                state_tx.send(AuctionState::Bidding(BiddingState { item: item.clone(), active_bid: ActiveBidState::JapaneseAuctionBid(bid_state) })).await?;

                            },
                        }
                    },
                    JapaneseAuctionEvent::NewPriceClockInterval { price_increase_per_100_seconds } => {
                        // The new interval is set immediately, but its first tick will happen half of the previous interval into the future.
                        // This ensures that a new tick will happen quickly, but also that a rapid sequence of updates doesn't cause many quick ticks.
                        let new_period = Duration::from_secs_f32(100.0 / price_increase_per_100_seconds as f32);
                        price_increase_interval = interval_at(Instant::now() + (price_increase_interval.period()/2), new_period);
                        current_price_increase_per_100_seconds = price_increase_per_100_seconds;

                        // Also, we need to send an update of the state now, so that the button receives the new change
                        let bid_state = if arena_is_closed {
                            JapaneseAuctionBidState::ClockRunning { currently_in_arena: arena.clone(), current_price, current_price_increase_per_100_seconds, arena_visibility_mode }
                        } else {
                            JapaneseAuctionBidState::EnterArena { currently_in_arena: arena.clone(), seconds_until_arena_closes: arena_closes_for_entry.duration_since(Instant::now()).as_secs_f32(), current_price, current_price_increase_per_100_seconds, arena_visibility_mode }
                        };
                        state_tx.send(AuctionState::Bidding(BiddingState { item: item.clone(), active_bid: ActiveBidState::JapaneseAuctionBid(bid_state) })).await?;

                    },
                    JapaneseAuctionEvent::NewArenaVisibilityMode(mode) => {
                        arena_visibility_mode = mode;
                        let bid_state = if arena_is_closed {
                            JapaneseAuctionBidState::ClockRunning { currently_in_arena: arena.clone(), current_price, current_price_increase_per_100_seconds, arena_visibility_mode }
                        } else {
                            JapaneseAuctionBidState::EnterArena { currently_in_arena: arena.clone(), seconds_until_arena_closes: arena_closes_for_entry.duration_since(Instant::now()).as_secs_f32(), current_price, current_price_increase_per_100_seconds, arena_visibility_mode }
                        };
                        state_tx.send(AuctionState::Bidding(BiddingState { item: item.clone(), active_bid: ActiveBidState::JapaneseAuctionBid(bid_state) })).await?;
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
                let bid_state = JapaneseAuctionBidState::ClockRunning { currently_in_arena: arena.clone(), current_price, current_price_increase_per_100_seconds, arena_visibility_mode };

                state_tx.send(AuctionState::Bidding(BiddingState { item: item.clone(), active_bid: ActiveBidState::JapaneseAuctionBid(bid_state) })).await?;


            }

            _ = update_interval.tick() => {
                // Publish the current state (price, mode and arena members)
                // ONLY IF the arena is currently open -> arena closing timer is counting down
                // (if the arena is closed, this is handled in the price_increase_interval tick, where we send a message on every price change)
                if !arena_is_closed {
                    let bid_state = JapaneseAuctionBidState::EnterArena { currently_in_arena: arena.clone(), seconds_until_arena_closes: arena_closes_for_entry.duration_since(Instant::now()).as_secs_f32(), current_price, current_price_increase_per_100_seconds, arena_visibility_mode };
                    state_tx.send(AuctionState::Bidding(BiddingState { item: item.clone(), active_bid: ActiveBidState::JapaneseAuctionBid(bid_state) })).await?;
                }

            }
        }
    }
}
