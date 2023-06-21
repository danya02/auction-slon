use serde::{Serialize, Deserialize};

use crate::{UserAccountData, Money};

#[derive(Serialize, Deserialize)]
pub enum AuctionState {
    /// Waiting for an item to be submitted
    WaitingForItem,
    
    /// Showing item before bidding
    ShowingItemBeforeBidding(AuctionItem),

    /// Currently bidding
    Bidding(BiddingState),

    /// Item was sold to someone who isn't you
    SoldToSomeoneElse {
        item: AuctionItem,
        sold_to: UserAccountData,
        sold_for: Money,
    },

    /// Item was sold to you
    SoldToYou {
        item: AuctionItem,
        sold_for: Money,
        confirmation_code: String,  // show this to the auctioneer to retrieve item
    },
}

#[derive(Serialize, Deserialize)]
pub enum AuctionItem {
    /// An item such that there is only one of them
    UniqueItem {
        name: String,
    },

    /// An item sold as one of a sequence of identical items, distinguished by their numbers
    MultipleItem {
        name: String,
        current_count: u32,  // starts at 1
        max_count: u32,
    }
}

#[derive(Serialize, Deserialize)]
pub struct BiddingState {
    item: AuctionItem,
    active_bid: ActiveBidState,
}

#[derive(Serialize, Deserialize)]
pub enum ActiveBidState {

    /// The current auction is an [English Auction](https://en.wikipedia.org/wiki/English_auction)
    EnglishAuctionBid {
        /// Current bid amount and person
        current_bid_amount: Money,
        current_bid: UserAccountData,
        current_bid_is_you: bool,

        /// Currently allowed minimum increment
        minimum_increment: Money,

        /// Amount of time until the current bid is locked in (resets on every bid)
        seconds_until_commit: f32,
    },

    /// The current auction is an [ascending clock auction](https://en.wikipedia.org/wiki/Japanese_auction)
    JapaneseAuctionBid(JapaneseAuctionBidState),
}

#[derive(Serialize, Deserialize)]
pub enum JapaneseAuctionBidState {
    /// The buyers are entering the arena
    EnterArena {
        currently_in_arena: Vec<UserAccountData>,
        you_in_arena: bool,

        seconds_until_arena_closes: f32,
    },

    /// The buyers can now exit the arena; last person standing wins the item
    ClockRunning {
        currently_in_arena: Vec<UserAccountData>,
        you_in_arena: bool,

        current_price: Money,
    },
}
