use std::rc::Rc;

use common::components::{ItemDisplay, UserAccountCard};
use common::layout::{Container, VerticalStack};
use communication::{auction::state::AuctionItem, AdminClientMessage, Money, UserAccountData};
use yew::prelude::*;

use crate::AppCtx;

#[derive(Properties, PartialEq)]
pub struct ItemSoldDisplayProps {
    pub item: AuctionItem,
    pub sold_to: UserAccountData,
    pub sold_for: Money,
    pub confirmation_code: String,
    pub contributions: Vec<(UserAccountData, Money)>,
}

#[function_component]
pub fn ItemSoldDisplay(props: &ItemSoldDisplayProps) -> Html {
    let ctx: Rc<AppCtx> = use_context().expect("no ctx found");
    let send = &ctx.send;

    let return_cb = {
        let send = send.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            send.emit(AdminClientMessage::StartAuction);
        })
    };
    html!(
        <Container>
            <VerticalStack>
                <h1>{"Item sold!"}</h1>
                <div class="row justify-content-evenly align-items-center mb-3">
                    <div class="col-5">
                        <ItemDisplay item={props.item.clone()} />
                    </div>
                    <div class="col-1">
                        <h1>{"→"}</h1>
                    </div>
                    <div class="col-5">
                        <UserAccountCard account={props.sold_to.clone()} />
                    </div>
                </div>

                <h2>{"Код подтверждения:"}</h2>
                <h3 style="font-size: calc(100vw/0.625/6);">{props.confirmation_code.clone()}</h3>
                // Font calc: https://stackoverflow.com/a/31322756/5936187
                <div class="d-grid gap-2">
                    <button onclick={return_cb} class="btn btn-success">{"Вернуться к выбору предмета"}</button>
                </div>
            </VerticalStack>
        </Container>
    )
}
