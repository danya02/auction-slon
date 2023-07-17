use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    time::SystemTime,
};

use admin_state::AdminState;
use auction::{
    actions::JapaneseAuctionAction,
    state::{ArenaVisibilityMode, AuctionItem, AuctionState},
};
use serde::{Deserialize, Serialize};

pub mod admin_state;
pub mod auction;

pub fn encode<T>(msg: &T) -> Vec<u8>
where
    T: Serialize,
{
    // JSON
    serde_json::to_vec(msg).expect("Error while serializing to JSON")
}

pub type DecodeError = serde_json::Error;

pub fn decode<'de, T>(data: &'de [u8]) -> Result<T, DecodeError>
where
    T: Deserialize<'de>,
{
    serde_json::from_slice::<'de, T>(data)
}

#[derive(Serialize, Deserialize, Clone)]
pub enum LoginRequest {
    AsAdmin { key: String },

    AsUser { key: String },
}

pub type Money = u32;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct UserAccountData {
    pub id: i64,
    pub user_name: String,
    pub balance: Money,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct UserAccountDataWithSecrets {
    pub id: i64,
    pub user_name: String,
    pub balance: Money,
    pub login_key: String,
}

impl From<UserAccountDataWithSecrets> for UserAccountData {
    fn from(value: UserAccountDataWithSecrets) -> Self {
        #[allow(unused_variables)]
        let UserAccountDataWithSecrets {
            id,
            user_name,
            balance,
            login_key,
        } = value;
        UserAccountData {
            id,
            user_name,
            balance,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerMessage {
    YourAccount(UserAccountData),
    AuctionMembers(WithTimestamp<Vec<UserAccountData>>),
    AuctionState(WithTimestamp<AuctionState>),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AdminServerMessage {
    AuctionMembers(WithTimestamp<Vec<UserAccountDataWithSecrets>>),
    AuctionState(WithTimestamp<AuctionState>),
    ItemStates(WithTimestamp<Vec<ItemState>>),
    AdminState(WithTimestamp<AdminState>),
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
}
