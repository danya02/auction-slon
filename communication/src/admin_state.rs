use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::Money;

/// State info only useful for the admin connection

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct AdminState {
    /// The time when this state was created.
    /// This won't be used much, but is needed to signal uniqueness.
    pub when: SystemTime,

    /// This balance is used to transfer money between accounts when the auction is running.
    pub holding_account_balance: Money,
}
