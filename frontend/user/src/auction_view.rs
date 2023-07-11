use communication::{auction::state::AuctionState, UserAccountData};
use yew::prelude::*;

use common::{
    components::AuctionReportView,
    layout::{Container, VerticalStack},
    screens::fullscreen_message::FullscreenMsg,
};

use crate::components::{
    bidding_screen::BiddingScreen,
    item_sold::{SoldToSomeoneElse, SoldToYou},
    show_item_before_bid::ShowItemBeforeBid,
};

#[derive(Properties, PartialEq)]
pub struct AuctionViewProps {
    pub state: AuctionState,
    pub members: Vec<UserAccountData>, // TODO: this is inefficient; consider alternative ways of passing this list
    pub account: UserAccountData,
    pub send: Callback<Vec<u8>>,
}

#[function_component]
pub fn AuctionView(props: &AuctionViewProps) -> Html {
    match &props.state {
        AuctionState::WaitingForAuction => {
            html!(<FullscreenMsg message="Waiting for auction to begin..." show_reload_button={true} user_account={props.account.clone()}/>)
        }
        AuctionState::AuctionOver(report) => {
            html!(
            <Container>
                <VerticalStack>
                    <h1>{"Auction has now been concluded"}</h1>
                    <AuctionReportView report={report.clone()} highlight_user_id={Some(props.account.id)}/>
                </VerticalStack>
            </Container>

            )
        }
        AuctionState::WaitingForItem => {
            html!(<FullscreenMsg message="Waiting for item to be presented..." show_reload_button={true} user_account={props.account.clone()}/>)
        }
        AuctionState::ShowingItemBeforeBidding(item) => {
            html!(<ShowItemBeforeBid item={item.clone()} />)
        }
        AuctionState::Bidding(bid_state) => {
            html!(<BiddingScreen bid_state={bid_state.clone()} send={props.send.clone()} my_account={props.account.clone()}/>)
        }
        AuctionState::SoldToYou {
            item,
            sold_for,
            confirmation_code,
        } => {
            html!(<SoldToYou item={item.clone()} sold_for={sold_for} confirmation_code={confirmation_code.clone()} />)
        }
        AuctionState::SoldToSomeoneElse {
            item,
            sold_to,
            sold_for,
        } => {
            html!(<SoldToSomeoneElse item={item.clone()} sold_to={sold_to.clone()} sold_for={sold_for} />)
        }
        _ => {
            html!(<FullscreenMsg message={format!("Current auction state is not implemented: {:?}", &props.state)} show_reload_button={true} user_account={props.account.clone()}/>)
        }
    }
}
