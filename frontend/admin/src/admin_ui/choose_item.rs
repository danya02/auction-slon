use common::components::MoneyDisplay;
use communication::{admin_state::AdminState, AdminClientMessage, ItemState, WithTimestamp};
use yew::prelude::*;

use super::SendToServer;

#[derive(Properties, PartialEq)]
pub struct ChooseItemProps {
    pub items: WithTimestamp<Vec<ItemState>>,
    pub send: SendToServer,
    pub admin_state: WithTimestamp<AdminState>,
}

#[function_component]
pub fn ChooseItemToSell(props: &ChooseItemProps) -> Html {
    let mut item_rows: Vec<Html> = vec![];
    for item in &*props.items {
        let action = match &item.state {
            communication::ItemStateValue::Sellable => {
                let send = props.send.clone();
                let id = item.item.id;

                // Do not allow selling until the holding account is at zero.
                if props.admin_state.holding_account_balance == 0 {
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

        let item_html = html! {
            <tr>
                <td>{&item.item.name}</td>
                <td><MoneyDisplay money={item.item.initial_price} /></td>
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
