use auction::state::AuctionState;
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
    pub user_name: String,
    pub balance: Money,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerMessage {
    YourAccount(UserAccountData),
    AuctionMembers(Vec<UserAccountData>),
    AuctionState(AuctionState),
}

impl From<UserAccountData> for ServerMessage {
    fn from(value: UserAccountData) -> Self {
        ServerMessage::YourAccount(value)
    }
}

impl From<Vec<UserAccountData>> for ServerMessage {
    fn from(value: Vec<UserAccountData>) -> Self {
        ServerMessage::AuctionMembers(value)
    }
}

impl From<AuctionState> for ServerMessage {
    fn from(value: AuctionState) -> Self {
        ServerMessage::AuctionState(value)
    }
}
