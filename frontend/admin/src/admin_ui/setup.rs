use common::components::{MoneyDisplay, NumberInput, TextInput};
use communication::{AdminClientMessage, ItemState, Money, UserAccountDataWithSecrets};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::HtmlInputElement;
use yew::prelude::*;

use super::SendToServer;

#[derive(Properties, PartialEq)]
pub struct SetupAuctionProps {
    pub send: SendToServer,
    pub users: Vec<UserAccountDataWithSecrets>,
    pub items: Vec<ItemState>,
}

#[function_component]
pub fn SetupAuction(props: &SetupAuctionProps) -> Html {
    html! {
        <div class="row">
            <div class="col-6">
                <h2>{"Edit users"}</h2>
                <UserSetup users={props.users.clone()} send={props.send.clone()} />
            </div>
            <div class="col-6">
                <h2>{"Edit items"}</h2>
                <ItemSetup items={props.items.clone()} send={props.send.clone()} />
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct UserSetupProps {
    pub send: SendToServer,
    pub users: Vec<UserAccountDataWithSecrets>,
}

#[function_component]
fn UserSetup(props: &UserSetupProps) -> Html {
    let mut rows = Vec::with_capacity(props.users.len());

    for user in &props.users {
        let commit_name_cb = {
            let send = props.send.clone();
            let user_id = user.id;

            Callback::from(move |s: String| {
                send.emit(AdminClientMessage::ChangeUserName {
                    id: user_id,
                    new_name: s,
                });
            })
        };

        let commit_balance_cb = {
            let send = props.send.clone();
            let user_id = user.id;

            Callback::from(move |s: String| {
                send.emit(AdminClientMessage::ChangeUserBalance {
                    id: user_id,
                    new_balance: s,
                });
            })
        };

        let delete_user_cb = {
            let send = props.send.clone();
            let user_id = user.id;
            Callback::from(move |e: MouseEvent| {
                e.prevent_default();
                send.emit(AdminClientMessage::DeleteUser { id: user_id });
            })
        };

        let row = html! {
            <div class="card mb-3">
                <div class="card-body">
                    <TextInput prefill_value={user.user_name.clone()} onchange={commit_name_cb} />
                    <div class="input-group mb-2">
                        <span class="input-group-text">{"Balance: "}</span>
                        <NumberInput prefill_value={user.balance.to_string()} onchange={commit_balance_cb} min="0" max={Money::MAX.to_string()} step="1" />
                    </div>
                    <p>{"Login key: "}<code>{user.login_key.clone()}</code></p>
                    <button class="btn btn-danger" onclick={delete_user_cb}>{"Delete user"}</button>
                </div>
            </div>
        };
        rows.push(row);
    }

    let new_user_name = use_state(|| String::new());

    let new_user_name_edit_cb = {
        let new_user_name = new_user_name.clone();
        Callback::from(move |e: InputEvent| {
            let event: Event = e.dyn_into().unwrap_throw();
            let event_target = event.target().unwrap_throw();
            let target: HtmlInputElement = event_target.dyn_into().unwrap_throw();
            new_user_name.set(target.value());
        })
    };

    let add_user_cb = {
        let new_user_name = new_user_name.clone();
        let send = props.send.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            let name = (*new_user_name).clone();
            send.emit(AdminClientMessage::CreateUser { name });
            new_user_name.set(String::new());
        })
    };
    rows.push(html!(
        <div class="card mb-3">
            <div class="card-body">
                <input class="form-control mb-2" type="text" value={(*new_user_name).clone()} oninput={new_user_name_edit_cb} placeholder="New user name..." />
                <button class="btn btn-success" onclick={add_user_cb}>{"Add user"}</button>
            </div>
        </div>
    ));

    html! { for rows }
}

#[derive(Properties, PartialEq)]
struct ItemSetupProps {
    pub send: SendToServer,
    pub items: Vec<ItemState>,
}

#[function_component]
fn ItemSetup(props: &ItemSetupProps) -> Html {
    let mut rows = Vec::with_capacity(props.items.len());

    for item in &props.items {
        let item_id = item.item.id;

        let item_state_component = match &item.state {
            communication::ItemStateValue::Sellable => html!(<p>{"Item can be sold"}</p>),
            communication::ItemStateValue::AlreadySold { buyer, sale_price } => {
                let reset_sale_status_cb = {
                    let send = props.send.clone();
                    Callback::from(move |e: MouseEvent| {
                        e.prevent_default();
                        send.emit(AdminClientMessage::ClearSaleStatus { id: item_id });
                    })
                };
                html! {
                    <>
                        <p>{"Item was already sold to "}{buyer.user_name.clone()}{" for "}<MoneyDisplay money={sale_price} /></p>
                        <button class="btn btn-warning" onclick={reset_sale_status_cb}>{"Clear sale status"}</button>
                    </>
                }
            }
        };

        let commit_name_cb = {
            let send = props.send.clone();
            Callback::from(move |s: String| {
                send.emit(AdminClientMessage::ChangeItemName {
                    id: item_id,
                    new_name: s,
                });
            })
        };

        let commit_initial_price_cb = {
            let send = props.send.clone();
            Callback::from(move |s: String| {
                send.emit(AdminClientMessage::ChangeItemInitialPrice {
                    id: item_id,
                    new_price: s,
                });
            })
        };

        let delete_item_cb = {
            let send = props.send.clone();
            Callback::from(move |e: MouseEvent| {
                e.prevent_default();
                send.emit(AdminClientMessage::DeleteItem { id: item_id });
            })
        };

        let row = html! {
            <div class="card mb-3">
                <div class="card-body">
                    <TextInput prefill_value={item.item.name.clone()} onchange={commit_name_cb} />
                    <div class="input-group mb-2">
                        <span class="input-group-text">{"Initial price: "}</span>
                        <NumberInput prefill_value={item.item.initial_price.to_string()} onchange={commit_initial_price_cb} min="0" max={Money::MAX.to_string()} step="1" />
                    </div>
                    {item_state_component}
                    <button class="btn btn-danger" onclick={delete_item_cb}>{"Delete item"}</button>
                </div>
            </div>
        };

        rows.push(row);
    }

    let new_item_name = use_state(|| String::new());

    let new_item_name_edit_cb = {
        let new_item_name = new_item_name.clone();
        Callback::from(move |e: InputEvent| {
            let event: Event = e.dyn_into().unwrap_throw();
            let event_target = event.target().unwrap_throw();
            let target: HtmlInputElement = event_target.dyn_into().unwrap_throw();
            new_item_name.set(target.value());
        })
    };

    let add_item_cb = {
        let new_item_name = new_item_name.clone();
        let send = props.send.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            let name = (*new_item_name).clone();
            send.emit(AdminClientMessage::CreateItem { name });
            new_item_name.set(String::new());
        })
    };
    rows.push(html!(
        <div class="card mb-3">
            <div class="card-body">
                <input class="form-control mb-2" type="text" value={(*new_item_name).clone()} oninput={new_item_name_edit_cb} placeholder="New item name..." />
                <button class="btn btn-success" onclick={add_item_cb}>{"Add item"}</button>
            </div>
        </div>
    ));

    html! { for rows }
}
