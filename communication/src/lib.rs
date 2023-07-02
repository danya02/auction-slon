use auction::state::{AuctionItem, AuctionState};
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerMessage {
    YourAccount(UserAccountData),
    AuctionMembers(Vec<UserAccountData>),
    AuctionState(AuctionState),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AdminServerMessage {
    AuctionMembers(Vec<UserAccountData>),
    AuctionState(AuctionState),
    ItemStates(Vec<ItemState>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ItemState {
    pub item: AuctionItem,
    pub state: ItemStateValue,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ItemStateValue {
    /// The item is available to be sold
    Sellable,

    /// The item is the subject of the current sale
    BeingSold,

    /// The item has been sold, and should not be sold again
    AlreadySold,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AdminClientMessage {
    /// Reset the auction to the "waiting for item" state.
    StartAuction,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum UserClientMessage {}
