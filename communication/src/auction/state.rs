use serde::{Deserialize, Serialize};

use crate::{ItemState, Money, UserAccountData};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum AuctionState {
    /// Waiting for auction to begin
    WaitingForAuction,

    /// Auction is concluded
    AuctionOver(AuctionReport),

    /// Waiting for an item to be submitted
    WaitingForItem,

    /// Showing item before bidding
    ShowingItemBeforeBidding(AuctionItem),

    /// Currently bidding
    Bidding(BiddingState),

    /// Item was sold to someone who isn't you (client only)
    SoldToSomeoneElse {
        item: AuctionItem,
        sold_to: UserAccountData,
        sold_for: Money,
        contributions: Vec<(UserAccountData, Money)>,
    },

    /// Item was sold to you (client only)
    SoldToYou {
        item: AuctionItem,
        sold_for: Money,
        confirmation_code: String, // show this to the auctioneer to retrieve item
        contributions: Vec<(UserAccountData, Money)>,
    },

    /// Item was sold to an auction member, who will retrieve it (admin only)
    SoldToMember {
        item: AuctionItem,
        sold_for: Money,
        sold_to: UserAccountData,
        confirmation_code: String,
        contributions: Vec<(UserAccountData, Money)>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct AuctionItem {
    pub id: i64,
    pub name: String,
    pub initial_price: Money,
}

/// Structure representing the outcome of the auction, with the members' final balances and sales.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct AuctionReport {
    pub members: Vec<UserAccountData>,
    pub items: Vec<ItemState>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct BiddingState {
    pub item: AuctionItem,
    pub active_bid: ActiveBidState,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ActiveBidState {
    /// The current auction is an [English Auction](https://en.wikipedia.org/wiki/English_auction)
    EnglishAuctionBid {
        /// Current bid amount and person
        current_bid_amount: Money,
        current_bidder: UserAccountData,

        /// Currently allowed minimum increment
        minimum_increment: Money,

        /// Amount of time until the current bid is locked in (resets on every bid)
        seconds_until_commit: f32,

        /// The maximum amount of time that a bid takes to lock in (to which it's reset each bid)
        max_millis_until_commit: u128,
    },

    /// The current auction is an [ascending clock auction](https://en.wikipedia.org/wiki/Japanese_auction)
    JapaneseAuctionBid(JapaneseAuctionBidState),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum JapaneseAuctionBidState {
    /// The buyers are entering the arena
    EnterArena {
        currently_in_arena: Vec<UserAccountData>,
        arena_visibility_mode: ArenaVisibilityMode,
        current_price: Money,
        current_price_increase_per_100_seconds: Money,
        seconds_until_arena_closes: Option<f32>,
    },

    /// The buyers can now exit the arena; last person standing wins the item
    ClockRunning {
        currently_in_arena: Vec<UserAccountData>,
        arena_visibility_mode: ArenaVisibilityMode,
        current_price: Money,
        current_price_increase_per_100_seconds: Money,
    },
}

/// How to show the arena in the user's UI.
///
/// This is not theoretically secure, because the arena needs to be in the message
/// for other technical reasons, but it is effective for mobile users.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub enum ArenaVisibilityMode {
    /// Can see the list of people in the arena, including balance
    Full,

    /// Can see the number of people in the arena, but nothing else.
    OnlyNumber,

    /// Cannot see any info about the arena, apart from whether they are in it or not.
    Nothing,
}

impl JapaneseAuctionBidState {
    pub fn get_arena(&self) -> &[UserAccountData] {
        match self {
            JapaneseAuctionBidState::EnterArena {
                currently_in_arena, ..
            } => currently_in_arena,
            JapaneseAuctionBidState::ClockRunning {
                currently_in_arena, ..
            } => currently_in_arena,
        }
    }

    pub fn get_price_increase_rate(&self) -> Money {
        *match self {
            JapaneseAuctionBidState::EnterArena {
                current_price_increase_per_100_seconds,
                ..
            } => current_price_increase_per_100_seconds,
            JapaneseAuctionBidState::ClockRunning {
                current_price_increase_per_100_seconds,
                ..
            } => current_price_increase_per_100_seconds,
        }
    }

    pub fn get_arena_visibility_mode(&self) -> ArenaVisibilityMode {
        *match self {
            JapaneseAuctionBidState::EnterArena {
                arena_visibility_mode,
                ..
            } => arena_visibility_mode,
            JapaneseAuctionBidState::ClockRunning {
                arena_visibility_mode,
                ..
            } => arena_visibility_mode,
        }
    }
}

/// An active Sponsorship allows one user to spend money that is not in their own account.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct Sponsorship {
    pub id: i64,
    pub donor_id: i64,
    pub recepient_id: i64,
    pub status: SponsorshipStatus,
    pub balance_remaining: Money,
}

impl Sponsorship {
    /// Calculate the effective balance that the user has access to.
    /// First check the balance of the user,
    /// and then the balances of all those who sponsor them.
    /// (Subsponsors are not allowed.)
    pub fn resolve_available_balance(
        user_id: i64,
        users: &[UserAccountData],
        sponsorships: &[Sponsorship],
    ) -> Money {
        // Special case: the null user has infinite money
        if user_id == 0 {
            return Money::MAX;
        }

        // First, get the list of users who directly sponsor me, along with how much they'd spend on me at most.
        let mut my_sponsors = sponsorships
            .iter()
            .filter(|s| s.recepient_id == user_id)
            .filter_map(|s| {
                (s.status == SponsorshipStatus::Active).then_some((s.donor_id, s.balance_remaining))
            })
            .collect::<Vec<_>>();

        // Include myself: I would spend any amount on myself.
        my_sponsors.push((user_id, Money::MAX));

        // But if someone doesn't have the amount they want to spend, then reduce it.
        for (sponsor_id, available_to_spend) in my_sponsors.iter_mut() {
            let user = users
                .iter()
                .find(|u| u.id == *sponsor_id)
                .expect("User is sponsoring me but it does not exist?");
            *available_to_spend = (*available_to_spend).min(user.balance);
        }

        // Finally, calculate the sum. Use saturating arithmetic (just in case the balance is big)
        my_sponsors
            .iter()
            .map(|t| t.1)
            .fold(0, |a, b| a.saturating_add(b))
    }

    /// When a user has won an auction,
    /// use this to calculate how to draw the money from the accounts
    /// of that user and their sponsors.
    #[cfg(feature = "rand")]
    pub fn calculate_contributions(
        user_id: i64,
        purchase_price: Money,
        users: &[UserAccountData],
        sponsorships: &[Sponsorship],
    ) -> Vec<(UserAccountData, Money)> {
        // First, get the list of users who directly sponsor me, along with the maximum value that they will spend on me.
        let mut my_sponsors = sponsorships
            .iter()
            .filter(|s| s.status == SponsorshipStatus::Active)
            .filter_map(|s| {
                (s.recepient_id == user_id).then_some((s.donor_id, s.balance_remaining))
            })
            .map(|(id, balance)| {
                (
                    users
                        .iter()
                        .find(|u| u.id == id)
                        .expect("Exists sponsorship from user who doesn't exist?"),
                    balance,
                )
            })
            .collect::<Vec<_>>();
        // Include myself.
        my_sponsors.push({
            let me = users
                .iter()
                .find(|u| u.id == user_id)
                .expect("Does not exist user who is buying item?");
            (me, me.balance)
        });

        // The weights of each member are the amounts they were willing to spend,
        // ignoring how much they actually have.
        let mut member_weights: Vec<_> = my_sponsors.iter().map(|(_, b)| *b).collect();
        // However, the last member (who is myself)
        // will have a weight half of my balance, plus one.
        // This makes it more likely that other members will get to share some of the spend.
        // TODO: make this more mathematically justified?
        {
            let last_ref = member_weights.last_mut().unwrap();
            *last_ref = (*last_ref) / 2;
            *last_ref += 1;
        }

        // Limit the spendable balance of each member by the true balance that they have
        my_sponsors
            .iter_mut()
            .for_each(|(u, b)| *b = (*b).min(u.balance));

        // Get the collective balance of these.
        let total_balance: Money = my_sponsors.iter().map(|(_, b)| b).sum();
        assert!(
            purchase_price <= total_balance,
            "Sponsorship group cannot afford to buy this"
        );

        let mut contributions: Vec<_> =
            my_sponsors.iter().map(|(u, _)| ((*u).clone(), 0)).collect();

        {
            use rand::distributions::WeightedIndex;
            use rand::prelude::*;
            let mut rng = thread_rng();

            // These are the values of how much a member would have at every step.
            let mut remaining_balances: Vec<_> = my_sponsors.iter().map(|(_, b)| *b).collect();

            // This is the priority with which we'll take money from each member.
            // It starts out equal to their desired money to be shared,
            // but it'll get set to zero when they have no more.
            let mut weights = member_weights;

            // The first zeroizing:
            remaining_balances
                .iter()
                .zip(weights.iter_mut())
                .for_each(|(balance, weight)| *weight = if balance > &&0 { *weight } else { 0 });

            let mut price_left = purchase_price;

            while price_left > 0 {
                // At each step, we'll choose a member based on their balance
                let dist = WeightedIndex::new(&weights)
                    .expect("Weights turned to all zero while distributing money spend?");
                let chosen_member = dist.sample(&mut rng);
                // That member's balance is decreased by one, and the price remaining is too,
                // and their contribution increases.
                remaining_balances[chosen_member] -= 1;
                price_left -= 1;
                contributions[chosen_member].1 += 1;
                // If the member ran out of money, drop them from the sampling.
                if remaining_balances[chosen_member] == 0 {
                    weights[chosen_member] = 0;
                }
            }
        }

        assert_eq!(
            contributions.iter().map(|(_, p)| p).sum::<Money>(),
            purchase_price,
            "Bug in money distribution logic"
        );

        contributions
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub enum SponsorshipStatus {
    /// The sponsorship was created with a valid code, and is now active.
    Active,

    /// The recepient rejected the sponsorship request.
    Rejected,

    /// The donor has decided not to provide the sponsorship anymore.
    Retracted,
}

impl<T> From<T> for SponsorshipStatus
where
    T: TryInto<u8>,
{
    fn from(value: T) -> Self {
        let v: u8 = value.try_into().unwrap_or(0);
        match v {
            1 => Self::Active,
            2 => Self::Rejected,
            3 => Self::Retracted,
            _ => Self::Retracted,
        }
    }
}

impl SponsorshipStatus {
    pub fn to_db_val(&self) -> u8 {
        match self {
            SponsorshipStatus::Active => 1,
            SponsorshipStatus::Rejected => 2,
            SponsorshipStatus::Retracted => 3,
        }
    }
}
