use communication::{auction::state::SponsorshipStatus, Money, UserSaleMode};

use super::{EnglishAuctionEvent, JapaneseAuctionEvent};

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

    /// A user has done an action on the English auction.
    EnglishAuctionAction(EnglishAuctionEvent),

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
    HoldingAccountTransfer { user_id: i64, new_balance: Money },

    /// Change whether a user accepts new sponsorships.
    SetIsAcceptingSponsorships {
        user_id: i64,
        is_accepting_sponsorships: bool,
    },

    SetSaleMode {
        user_id: i64,
        sale_mode: UserSaleMode,
    },

    /// A user is changing the state of a sponsorship
    /// (this is ignored if it does not follow policy)
    UpdateSponsorship {
        actor_id: i64,
        sponsorship_id: i64,
        new_status: Option<SponsorshipStatus>,
        new_amount: Option<Money>,
    },

    /// A user, if has sponsorships turned on, is requesting a new code for joining the sponsorship.
    RegenerateSponsorshipCode { user_id: i64 },

    /// A user is trying to create a sponsorship in which they are the donor, using the given code.
    /// If the code doesn't exist, nothing happens.
    TryActivateSponsorshipCode { user_id: i64, code: String },
}
