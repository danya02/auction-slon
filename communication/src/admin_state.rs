use serde::{Deserialize, Serialize};

use crate::Money;

/// State info only useful for the admin connection

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct AdminState {
    /// This balance is used to transfer money between accounts when the auction is running.
    pub holding_account_balance: Money,
}
