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
    },

    /// Item was sold to you (client only)
    SoldToYou {
        item: AuctionItem,
        sold_for: Money,
        confirmation_code: String, // show this to the auctioneer to retrieve item
    },

    /// Item was sold to an auction member, who will retrieve it (admin only)
    SoldToMember {
        item: AuctionItem,
        sold_for: Money,
        sold_to: UserAccountData,
        confirmation_code: String,
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
    },

    /// The current auction is an [ascending clock auction](https://en.wikipedia.org/wiki/Japanese_auction)
    JapaneseAuctionBid(JapaneseAuctionBidState),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum JapaneseAuctionBidState {
    /// The buyers are entering the arena
    EnterArena {
        currently_in_arena: Vec<UserAccountData>,

        seconds_until_arena_closes: f32,
    },

    /// The buyers can now exit the arena; last person standing wins the item
    ClockRunning {
        currently_in_arena: Vec<UserAccountData>,

        current_price: Money,
    },
}
