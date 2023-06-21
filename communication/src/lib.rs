use serde::{Deserialize, Serialize};

pub mod auction;

pub fn encode(msg: &impl Serialize) -> Vec<u8> {
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

#[derive(Serialize, Deserialize)]
pub enum LoginRequest {
    AsAdmin { key: String },

    AsUser { key: String },
}

pub type Money = u32;

#[derive(Serialize, Deserialize)]
pub struct UserAccountData {
    pub user_name: String,
    pub balance: Money,
}

#[derive(Serialize, Deserialize)]
pub struct YourAccount(UserAccountData);

#[derive(Serialize, Deserialize)]
pub struct AuctionMembers(Vec<UserAccountData>);


