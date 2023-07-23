use serde::{Deserialize, Serialize};

use crate::Money;

/// State info only useful for the admin connection

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct AdminState {
    /// This balance is used to transfer money between accounts when the auction is running.
    pub holding_account_balance: Money,

    /// This is the list of user IDs who currently have an open connection to the server.
    pub connected_users: Vec<i64>,
}
