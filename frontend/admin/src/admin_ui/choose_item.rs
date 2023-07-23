use std::rc::Rc;

use common::components::{MoneyDisplay, NumberInput};
use communication::{AdminClientMessage, Money};
use yew::prelude::*;

use crate::AppCtx;


#[function_component]
pub fn ChooseItemToSell() -> Html {
    let ctx: Rc<AppCtx> = use_context().expect("no ctx found");
    let send = &ctx.send;
    let items = &ctx.items;
    let admin_state = &ctx.admin_state;

    let mut item_rows: Vec<Html> = vec![];
    for item in items {
        let action = match &item.state {
            communication::ItemStateValue::Sellable => {
                let send = send.clone();
                let id = item.item.id;

                // Do not allow selling until the holding account is at zero.
                if admin_state.holding_account_balance == 0 {
                    let start_selling_cb = Callback::from(move |e: MouseEvent| {
                        e.prevent_default();
                        send.emit(AdminClientMessage::PrepareAuctioning(id));
                    });

                    html! {
                        <a href="#" class="btn btn-primary" onclick={start_selling_cb}>{"Sell this"}</a>
                    }
                } else {
                    html! {
                        <span class="btn btn-outline-danger disabled">{"Spend holding account first"}</span>
                    }
                }
            }
            communication::ItemStateValue::AlreadySold { buyer, sale_price } => html! {
                <a href="#" class="btn btn-secondary disabled">{"Sold to "}{&buyer.user_name}{" for "}<MoneyDisplay money={sale_price} /></a>
            },
        };

        let item_id = item.item.id;
        let commit_initial_price_cb = {
            let send = send.clone();
            Callback::from(move |s: String| {
                send.emit(AdminClientMessage::ChangeItemInitialPrice {
                    id: item_id,
                    new_price: s,
                });
            })
        };

        let item_html = html! {
            <tr>
                <td>{&item.item.name}</td>
                <td><NumberInput prefill_value={item.item.initial_price.to_string()} onchange={commit_initial_price_cb} min="0" max={Money::MAX.to_string()} step="1" /></td>
                <td>{action}</td>
            </tr>
        };

        item_rows.push(item_html);
    }

    html! {
        <table class="table table-sm">
            <thead>
                <tr>
                    <th scope="col">{"Name"}</th>
                    <th scope="col">{"Initial price"}</th>
                    <th>{"Action"}</th>
                </tr>
            </thead>
            <tbody>
                { for item_rows }
            </tbody>
        </table>

    }
}
