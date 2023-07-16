use communication::Money;

use super::JapaneseAuctionEvent;

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

    /// An admin has requested that a user be changed, created or deleted.
    ///
    /// If id is None, create.
    /// If id is Some, but name and balance is None, delete.
    /// If id is Some, and name or balance is Some, change.
    EditUser {
        id: Option<i64>,
        name: Option<String>,
        balance: Option<Money>,
    },

    /// An admin has forced clearing the sale status of an item
    ClearSaleStatus { id: i64 },

    /// An admin has requested that an item be changed, created or deleted.
    ///
    /// If id is None, create.
    /// If id is Some, but name and balance is None, delete.
    /// If id is Some, and name or balance is Some, change.
    EditItem {
        id: Option<i64>,
        name: Option<String>,
        initial_price: Option<Money>,
    },

    /// Change the value in the holding account in relation to a user account:
    /// either add or subtract the balance there,
    /// so that the user has the given amount of money,
    /// or the holding account has zero.
    HoldingAccountTransfer {
        user_id: i64,
        new_balance: Money,
    },
}
