use std::rc::Rc;

use common::components::ItemDisplay;
use communication::{auction::state::AuctionItem, AdminClientMessage};
use yew::prelude::*;

use crate::AppCtx;

#[derive(Properties, PartialEq)]
pub struct ConfirmItemProps {
    pub item: AuctionItem,
}

#[function_component]
pub fn ConfirmItemToSell(props: &ConfirmItemProps) -> Html {
    let ctx: Rc<AppCtx> = use_context().expect("no ctx found");
    let send = &ctx.send;

    let item = &props.item;
    let item_id = item.id;
    let reset_cb = {
        let send = send.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            send.emit(AdminClientMessage::StartAuction);
        })
    };
    let start_as_english_cb = {
        let send = send.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            send.emit(AdminClientMessage::RunEnglishAuction(item_id));
        })
    };
    let start_as_japanese_cb = {
        let send = send.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            send.emit(AdminClientMessage::RunJapaneseAuction(item_id));
        })
    };

    html! {
        <>
        <h1>{"Готов начать продавать:"}</h1>
        <ItemDisplay item={item.clone()} />
        <div class="d-flex gap-2 justify-content-center mb-3">
            <button class="btn btn-primary" type="button" onclick={start_as_english_cb}>{"Продать английским аукционом"}</button>
            <button class="btn btn-success" type="button" onclick={start_as_japanese_cb}>{"Продать японским аукционом"}</button>
        </div>
        <div class="d-grid gap-2 col-6 mx-auto mb-3">
            <button class="btn btn-danger" type="button" onclick={reset_cb}>{"Не продавать сейчас"}</button>
        </div>
        </>
    }
}
