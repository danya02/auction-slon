use common::components::MoneyDisplay;
use communication::{AdminClientMessage, ItemState};
use yew::prelude::*;

use super::SendToServer;

#[derive(Properties, PartialEq)]
pub struct ChooseItemProps {
    pub items: Vec<ItemState>,
    pub send: SendToServer,
}

#[function_component]
pub fn ChooseItemToSell(props: &ChooseItemProps) -> Html {
    let mut item_rows: Vec<Html> = vec![];
    for item in &props.items {
        let action = match &item.state {
            communication::ItemStateValue::Sellable => {
                let send = props.send.clone();
                let id = item.item.id;
                let start_selling_cb = Callback::from(move |e: MouseEvent| {
                    e.prevent_default();
                    send.emit(AdminClientMessage::PrepareAuctioning(id));
                });

                html! {
                    <a href="#" class="btn btn-primary stretched-link" onclick={start_selling_cb}>{"Sell this"}</a>
                }
            }
            communication::ItemStateValue::BeingSold => html! {
                <a href="#" class="btn btn-secondary disabled">{"Already selling this"}</a>
            },
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
