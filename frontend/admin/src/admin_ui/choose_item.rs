use common::components::MoneyDisplay;
use communication::{admin_state::AdminState, AdminClientMessage, ItemState};
use yew::prelude::*;

use super::SendToServer;

#[derive(Properties, PartialEq)]
pub struct ChooseItemProps {
    pub items: Vec<ItemState>,
    pub send: SendToServer,
    pub admin_state: AdminState,
}

#[function_component]
pub fn ChooseItemToSell(props: &ChooseItemProps) -> Html {
    let mut item_rows: Vec<Html> = vec![];
    for item in &props.items {
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
                        <a href="#" class="btn btn-primary stretched-link" onclick={start_selling_cb}>{"Sell this"}</a>
                    }
                } else {
                    html! {
                        <span class="btn btn-outline-danger disabled">{"Spend holding account first"}</span>
                    }
                }
            }
            communication::ItemStateValue::AlreadySold { buyer, sale_price } => html! {
                <a href="#" class="btn btn-secondary disabled">{format!("Sold this to {} for ", buyer.user_name)}<MoneyDisplay money={sale_price} /></a>
            },
        };

        let item_html = html! {
            <div class="card mb-3">
                <div class="card-body">
                    <h5 class="card-title">{&item.item.name}</h5>
                    <h6 class="card-subtitle">{"Initial price:"}<MoneyDisplay money={item.item.initial_price} /></h6>
                    {action}
                </div>
            </div>
        };

        item_rows.push(item_html);
    }
    let h: Html = item_rows.iter().cloned().collect();
    // TODO: why is the clone() necessary???
    h
}
