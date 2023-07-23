use std::{
    error::Error,
    fmt::Debug,
    ops::{Deref, DerefMut},
    time::SystemTime,
};

use admin_state::AdminState;
use auction::{
    actions::JapaneseAuctionAction,
    state::{ArenaVisibilityMode, AuctionItem, AuctionState, Sponsorship, SponsorshipStatus},
};
use serde::{Deserialize, Serialize};

pub mod admin_state;
pub mod auction;

pub fn encode<T>(msg: &T) -> Vec<u8>
where
    T: Serialize,
{
    // JSON
    //let json_data = serde_json::to_vec(msg).expect("Error while serializing to JSON");
    // Postcard
    let pc_data = postcard::to_stdvec(msg).expect("Error while serializing to Postcard");
    pc_data
}

pub fn decode<'de, T>(data: &'de [u8]) -> Result<T, impl Error>
where
    T: Deserialize<'de>,
{
    //serde_json::from_slice::<'de, T>(data)
    postcard::from_bytes(data)
}

#[derive(Serialize, Deserialize, Clone)]
pub enum LoginRequest {
    AsAdmin { key: String },

    AsUser { key: String },
}

pub type Money = u32;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum UserSaleMode {
    /// User is making bids in the auction
    Bidding,

    /// User is sharing money with other bidders
    Sponsoring,
}

impl<T> From<T> for UserSaleMode
where
    T: num::traits::Zero,
{
    fn from(value: T) -> Self {
        if value.is_zero() {
            Self::Bidding
        } else {
            Self::Sponsoring
        }
    }
}

impl From<UserSaleMode> for u8 {
    fn from(val: UserSaleMode) -> Self {
        match val {
            UserSaleMode::Bidding => 0,
            UserSaleMode::Sponsoring => 1,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct UserAccountData {
    pub id: i64,
    pub user_name: String,
    pub balance: Money,
    pub sale_mode: UserSaleMode,
    pub is_accepting_sponsorships: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct UserAccountDataWithSecrets {
    pub id: i64,
    pub user_name: String,
    pub balance: Money,
    pub login_key: String,
    pub sale_mode: UserSaleMode,
    pub sponsorship_code: Option<String>,
}

impl From<&UserAccountDataWithSecrets> for UserAccountData {
    fn from(value: &UserAccountDataWithSecrets) -> Self {
        #[allow(unused_variables)]
        let UserAccountDataWithSecrets {
            id,
            user_name,
            balance,
            login_key,
            sale_mode,
            sponsorship_code,
        } = value.clone();
        UserAccountData {
            id,
            user_name,
            balance,
            sale_mode,
            is_accepting_sponsorships: sponsorship_code.is_some(),
        }
    }
}

pub fn forget_user_secrets(src: Vec<UserAccountDataWithSecrets>) -> Vec<UserAccountData> {
    src.iter().map(|u| u.into()).collect()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerMessage {
    YourAccount(UserAccountDataWithSecrets),
    AuctionMembers(WithTimestamp<Vec<UserAccountData>>),
    AuctionState(WithTimestamp<AuctionState>),
    SponsorshipState(WithTimestamp<Vec<Sponsorship>>),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AdminServerMessage {
    AuctionMembers(WithTimestamp<Vec<UserAccountDataWithSecrets>>),
    AuctionState(WithTimestamp<AuctionState>),
    ItemStates(WithTimestamp<Vec<ItemState>>),
    AdminState(WithTimestamp<AdminState>),
    SponsorshipState(WithTimestamp<Vec<Sponsorship>>),
}

/// A wrapper type that adds a timestamp to the data.
/// This is useful so that the frontend knows to distinguish two identical datas,
/// and can rerender if needed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WithTimestamp<T> {
    pub data: T,
    pub when: SystemTime,
}

impl<T> WithTimestamp<T> {
    pub fn new_with_zero_time(data: T) -> WithTimestamp<T> {
        WithTimestamp {
            data,
            when: SystemTime::UNIX_EPOCH,
        }
    }
}

impl<T> Deref for WithTimestamp<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for WithTimestamp<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T> From<T> for WithTimestamp<T> {
    fn from(value: T) -> Self {
        WithTimestamp {
            data: value,
            when: SystemTime::now(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ItemState {
    pub item: AuctionItem,
    pub state: ItemStateValue,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ItemStateValue {
    /// The item is available to be sold
    Sellable,

    /// The item has been sold, and should not be sold again
    AlreadySold {
        buyer: UserAccountData,
        sale_price: Money,
    },
}

impl ItemStateValue {
    pub fn get_sale_price(&self) -> Option<Money> {
        match self {
            ItemStateValue::Sellable => None,
            ItemStateValue::AlreadySold { sale_price, .. } => Some(*sale_price),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AdminClientMessage {
    /// Reset the auction to the "waiting for item" state.
    StartAuction,

    /// Prepare for auctioning an item by its ID
    PrepareAuctioning(i64),

    /// Start auctioning an item according to the rules of an English auction.
    RunEnglishAuction(i64),

    /// Start auctioning an item according to the rules of a Japanese auction.
    RunJapaneseAuction(i64),

    /// Set the auction to the "auction over" state.
    FinishAuction,

    /// Set the auction to the "preparing" state.
    StartAuctionAnew,

    /// If a Japanese auction is running, remove a user from the arena, if they are there.
    KickFromJapaneseAuction(i64, i64),

    /// If a Japanese auction is running, change its clock rate.
    /// The clock rate is how much money the price increases per 100 seconds.
    SetJapaneseClockRate(Money),

    /// If a Japanese auction is running, change its arena visibility mode.
    SetJapaneseVisibilityMode(ArenaVisibilityMode),

    /// Change a user's name
    ChangeUserName { id: i64, new_name: String },

    /// Change a user's balance. If the balance cannot be parsed as a money value, ignore this.
    ChangeUserBalance { id: i64, new_balance: String },

    /// Create a user by name
    CreateUser { name: String },

    /// Delete a user by ID
    DeleteUser { id: i64 },

    /// Remove the record indicating that the item was sold.
    ClearSaleStatus { id: i64 },

    /// Create an item by name.
    CreateItem { name: String },

    /// Change the name of an item by ID.
    ChangeItemName { id: i64, new_name: String },

    /// Change the initial price of an item by ID.
    ChangeItemInitialPrice { id: i64, new_price: String },

    /// Delete an item by ID.
    DeleteItem { id: i64 },

    /// Transactionally transfer money between the holding account and the given user account,
    /// so that the user account has the given amount of money.
    /// If the holding account does not have enough, zero it out.
    TransferAcrossHolding { user_id: i64, new_balance: Money },

    /// If the current auction is English, change the time before a bid is locked in.
    /// Extend the time remaining in the current bid.
    SetEnglishAuctionCommitPeriod { new_period_ms: u128 },

    /// If the current auction is Japanese, and the arena isn't closing yet,
    /// start closing the arena.
    StartClosingJapaneseArena,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum UserClientMessage {
    BidInEnglishAuction {
        item_id: i64,
        bid_amount: Money,
    },
    JapaneseAuctionAction {
        item_id: i64,
        action: JapaneseAuctionAction,
    },
    SetIsAcceptingSponsorships(bool),
    SetSaleMode(UserSaleMode),
    TryActivateSponsorshipCode(String),
    SetSponsorshipBalance {
        sponsorship_id: i64,
        balance: Money,
    },
    SetSponsorshipStatus {
        sponsorship_id: i64,
        status: SponsorshipStatus,
    },
    RegenerateSponsorshipCode,
}
