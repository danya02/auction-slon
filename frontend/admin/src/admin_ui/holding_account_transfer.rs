use std::rc::Rc;

use common::components::NumberInput;
use communication::{AdminClientMessage, Money};
use yew::prelude::*;

use crate::AppCtx;

#[function_component]
pub fn HoldingAccountTransferTable() -> Html {
    let ctx: Rc<AppCtx> = use_context().expect("no ctx found");
    let users = &ctx.users;
    let send = &ctx.send;
    let admin_state = &ctx.admin_state;

    // Table where first column is user's name, and second column is an input to transfer money to/from holding acct.
    let mut rows = Vec::with_capacity(users.len());
    for user in users {
        let onchange = {
            let send = send.clone();
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
        let max = user.balance + (admin_state.holding_account_balance * 2);
        let max = max.to_string();
        let row = html!(
            <tr>
                <td>
                    <span class={classes!(if admin_state.connected_users.iter().any(|u| *u == user.id) {"text-success"} else {"text-danger"})}>
                        {user.user_name.clone()}
                    </span>
                </td>
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
                    <th scope="col">{"Имя пользователя"}</th>
                    <th scope="col">{"Баланс"}</th>
                </tr>
            </thead>
            <tbody>
                {for rows}
            </tbody>
        </table>
    }
}
