use auction::{
    actions::JapaneseAuctionAction,
    state::{AuctionItem, AuctionState},
};
use serde::{Deserialize, Serialize};

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
    AuctionMembers(Vec<UserAccountData>),
    AuctionState(AuctionState),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AdminServerMessage {
    AuctionMembers(Vec<UserAccountDataWithSecrets>),
    AuctionState(AuctionState),
    ItemStates(Vec<ItemState>),
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

    /// Change a user's name
    ChangeUserName { id: i64, new_name: String },

    /// Change a user's balance. If the balance cannot be parsed as a money value, ignore this.
    ChangeUserBalance { id: i64, new_balance: String },

    /// Create a user by name
    CreateUser { name: String },

    /// Delete a user by ID
    DeleteUser { id: i64 },
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
