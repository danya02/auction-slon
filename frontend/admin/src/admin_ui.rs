use common::layout::{Container, VerticalStack};
use communication::{auction::state::AuctionState, AdminClientMessage};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct AdminUiProps {
    pub auction_state: AuctionState,
    pub send: Callback<AdminClientMessage>,
}

#[function_component]
pub fn AdminUserInterface(props: &AdminUiProps) -> Html {
    let start_auction_cb = {
        let send = props.send.clone();
        Callback::from(move |_: MouseEvent| send.emit(AdminClientMessage::StartAuction))
    };
    let content = match &props.auction_state {
        AuctionState::WaitingForAuction => html! {
            <VerticalStack>
                <h1>{"Auction is not yet started"}</h1>
                <button class="btn btn-success" onclick={start_auction_cb}>{"Begin auction"}</button>
            </VerticalStack>
        },
        AuctionState::WaitingForItem => html! {
            <VerticalStack>
                <h1>{"Please choose an item to auction off next"}</h1>
            </VerticalStack>
        },
        AuctionState::AuctionOver => html! {
            <VerticalStack>
                <h1>{"Auction has now been concluded"}</h1>
                <button class="btn btn-success" onclick={start_auction_cb}>{"Begin auction"}</button>
            </VerticalStack>
        },
        AuctionState::ShowingItemBeforeBidding(_) => todo!(),
        AuctionState::Bidding(_) => todo!(),
        AuctionState::SoldToSomeoneElse { .. } => unreachable!(),
        AuctionState::SoldToYou { .. } => unreachable!(),
        AuctionState::SoldToMember {
            item,
            sold_for,
            sold_to,
            confirmation_code,
        } => todo!(),
    };

    html! {
        <>
            <AdminUiTabs state={props.auction_state.clone()}/>
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
    html! {
        <nav>
            <ul class="nav nav-pills nav-fill">
                <li class="nav-item">
                    <a href="" class={classes!("nav-link", "disabled", if matches!(props.state, AuctionState::WaitingForAuction) {Some("active")} else {None})}>{"Waiting for auction to begin"}</a>
                </li>

                <li class="nav-item">
                    <a href="" class={classes!("nav-link", "disabled", if matches!(props.state, AuctionState::WaitingForItem) {Some("active")} else {None})}>{"Waiting for item to be selected"}</a>
                </li>

                <li class="nav-item">
                    <a href="" class={classes!("nav-link", "disabled", if matches!(props.state, AuctionState::ShowingItemBeforeBidding(_)) {Some("active")} else {None})}>{"Item is being shown before bidding"}</a>
                </li>

                <li class="nav-item">
                    <a href="" class={classes!("nav-link", "disabled", if matches!(props.state, AuctionState::Bidding(_)) {Some("active")} else {None})}>{"Bidding in progress"}</a>
                </li>

                <li class="nav-item">
                    <a href="" class={classes!("nav-link", "disabled", if matches!(props.state, AuctionState::SoldToMember{..}) {Some("active")} else {None})}>{"Item is sold"}</a>
                </li>

                <li class="nav-item">
                    <a href="" class={classes!("nav-link", "disabled", if matches!(props.state, AuctionState::AuctionOver) {Some("active")} else {None})}>{"Auction is now concluded"}</a>
                </li>
            </ul>
        </nav>
    }
}
