use serde::{Serialize, Deserialize};

use crate::Money;

#[derive(Serialize, Deserialize)]
pub struct EnglishAuctionPlaceBid {
    pub bid_amount: Money,
}

#[derive(Serialize, Deserialize)]
pub enum JapaneseAuctionAction {
    /// Enter the arena, if it is possible to do so
    EnterArena,

    /// Exit the arena, forfeiting the current bid
    ExitArena,
}