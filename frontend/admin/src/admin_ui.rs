use std::rc::Rc;

use common::{
    components::{AuctionReportView, MoneyDisplay},
    layout::{Container, HorizontalStack, VerticalStack},
};
use communication::{auction::state::AuctionState, AdminClientMessage};
use yew::prelude::*;

use crate::{
    admin_ui::{
        choose_item::ChooseItemToSell, confirm_item::ConfirmItemToSell,
        holding_account_transfer::HoldingAccountTransferTable, item_sold::ItemSoldDisplay,
        show_bid_progress::ShowBidProgress,
    },
    AppCtx,
};

mod choose_item;
mod confirm_item;
mod holding_account_transfer;
mod item_sold;
mod setup;
mod show_bid_progress;

pub type SendToServer = Callback<AdminClientMessage>;

#[function_component]
pub fn AdminUserInterface() -> Html {
    let ctx: Rc<AppCtx> = use_context().expect("no ctx found");
    let send = &ctx.send;
    let start_auction_cb = {
        let send = send.clone();
        Callback::from(move |_: MouseEvent| send.emit(AdminClientMessage::StartAuction))
    };
    let start_auction_anew_cb = {
        let send = send.clone();
        Callback::from(move |_: MouseEvent| send.emit(AdminClientMessage::StartAuctionAnew))
    };

    let content = match &ctx.auction_state {
        AuctionState::WaitingForAuction => html! {
            <VerticalStack>
                <h1>{"Auction is not yet started"}</h1>
                <setup::SetupAuction/>
                <button class="btn btn-success" onclick={start_auction_cb}>{"Begin auction"}</button>
            </VerticalStack>
        },
        AuctionState::AuctionOver(report) => html! {
            <VerticalStack>
                <h1>{"Auction has now been concluded"}</h1>
                <AuctionReportView report={report.clone()} />
                <button class="btn btn-success" onclick={start_auction_anew_cb}>{"Return to start of auction"}</button>
            </VerticalStack>
        },

        AuctionState::WaitingForItem => {
            let conclude_cb = {
                let send = send.clone();
                Callback::from(move |e: MouseEvent| {
                    e.prevent_default();
                    send.emit(AdminClientMessage::FinishAuction);
                })
            };
            html! {
                <HorizontalStack>
                    <VerticalStack>
                        <h1>{"Please choose an item to auction off next"}</h1>
                        <ChooseItemToSell />
                        <button class="btn btn-danger" onclick={conclude_cb}>{"Conclude auction"}</button>
                    </VerticalStack>
                    <VerticalStack>
                        <h1>{"Transfer money manually"}</h1>
                        {
                            if ctx.admin_state.holding_account_balance == 0 {
                                html!(
                                    <div class="alert alert-success">
                                        {"Holding account balance: "}<MoneyDisplay money={0} />
                                    </div>
                                )
                            } else {
                                html!(
                                    <div class="alert alert-warning">
                                        {"Holding account balance: "}<MoneyDisplay money={ctx.admin_state.holding_account_balance} />
                                    </div>
                                )
                            }
                        }
                        <HoldingAccountTransferTable/>
                    </VerticalStack>
                </HorizontalStack>
            }
        }

        AuctionState::ShowingItemBeforeBidding(item) => {
            html!(<ConfirmItemToSell item={item.clone()} />)
        }
        AuctionState::Bidding(bid_state) => {
            html!(<ShowBidProgress bid_state={bid_state.clone()} />)
        }
        AuctionState::SoldToSomeoneElse { .. } => unreachable!(),
        AuctionState::SoldToYou { .. } => unreachable!(),
        AuctionState::SoldToMember {
            item,
            sold_for,
            sold_to,
            confirmation_code,
            contributions,
        } => {
            html!(<ItemSoldDisplay item={item.clone()} sold_to={sold_to.clone()} sold_for={*sold_for} confirmation_code={confirmation_code.clone()} contributions={contributions.clone()} />)
        }
    };

    html! {
        <>
            <AdminUiTabs state={(ctx.auction_state).clone()}/>
            <Container>
                {content}
            </Container>
        </>
    }
}

#[derive(Properties, PartialEq)]
pub struct AdminUiTabsProps {
    pub state: AuctionState,
}

#[function_component]
fn AdminUiTabs(props: &AdminUiTabsProps) -> Html {
    let ctx: Rc<AppCtx> = use_context().expect("no ctx found");
    let admin_state = &ctx.admin_state;
    let users = &ctx.users;

    html! {
        <>
        <nav>
            <ul class="nav nav-pills nav-fill">
                <li class="nav-item">
                    <a class={classes!("nav-link", "disabled", if matches!(props.state, AuctionState::WaitingForAuction) {Some("active")} else {None})}>{"Ждем начала аукциона"}</a>
                </li>

                <li class="nav-item">
                    <a class={classes!("nav-link", "disabled", if matches!(props.state, AuctionState::WaitingForItem) {Some("active")} else {None})}>{"Выбор предмета"}</a>
                </li>

                <li class="nav-item">
                    <a class={classes!("nav-link", "disabled", if matches!(props.state, AuctionState::ShowingItemBeforeBidding(_)) {Some("active")} else {None})}>{"Показываем предмет"}</a>
                </li>

                <li class="nav-item">
                    <a class={classes!("nav-link", "disabled", if matches!(props.state, AuctionState::Bidding(_)) {Some("active")} else {None})}>{"Ставки идут"}</a>
                </li>

                <li class="nav-item">
                    <a class={classes!("nav-link", "disabled", if matches!(props.state, AuctionState::SoldToMember{..}) {Some("active")} else {None})}>{"Продано"}</a>
                </li>

                <li class="nav-item">
                    <a class={classes!("nav-link", "disabled", if matches!(props.state, AuctionState::AuctionOver(_)) {Some("active")} else {None})}>{"Аукцион завершен"}</a>
                </li>
            </ul>
        </nav>
        <div class="alert alert-info">
            {"Подключено: "}
            {admin_state.connected_users.len()}
            {" из "}
            {users.len()}
        </div>
        </>
    }
}
