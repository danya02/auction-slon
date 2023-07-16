use common::components::NumberInput;
use communication::{
    admin_state::AdminState, AdminClientMessage, Money, UserAccountDataWithSecrets,
};
use yew::prelude::*;

use super::SendToServer;

#[derive(Properties, PartialEq)]
pub struct HoldingAccountTransferTableProps {
    pub send: SendToServer,
    pub admin_state: AdminState,
    pub users: Vec<UserAccountDataWithSecrets>,
}

#[function_component]
pub fn HoldingAccountTransferTable(props: &HoldingAccountTransferTableProps) -> Html {
    // Table where first column is user's name, and second column is an input to transfer money to/from holding acct.
    let mut rows = Vec::with_capacity(props.users.len());
    for user in &props.users {
        let onchange = {
            let send = props.send.clone();
            let user_id = user.id;
            Callback::from(move |s: String| {
                // Try parsing the input as a money value.
                // It should always succeed, because the input box is a number
                // with a limit over zero;
                // but, if it fails, just ignore it.
                let m: Money = match s.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return;
                    }
                };

                send.emit(AdminClientMessage::TransferAcrossHolding {
                    user_id,
                    new_balance: m,
                });
            })
        };
        // This maximum is bigger than the real maximum we can put in the account.
        // This is so that scrolling rapidly can work,
        // because it would take until the user's balance really got transferred across the holding account
        // for this maximum to update properly.
        // Instead, if the user scrolls too far out, we'll reset the input at the next user data update.
        let max = user.balance + (props.admin_state.holding_account_balance * 2);
        let max = max.to_string();
        let row = html!(
            <tr>
                <td>{user.user_name.clone()}</td>
                <td>
                    <NumberInput prefill_value={user.balance.to_string()} {onchange} min="0" {max} step="1" />
                </td>
            </tr>
        );

        rows.push(row);
    }

    html! {
        <table class="table table-dark">
            <thead>
                <tr>
                    <th scope="col">{"User name"}</th>
                    <th scope="col">{"Balance"}</th>
                </tr>
            </thead>
            <tbody>
                {for rows}
            </tbody>
        </table>
    }
}
