use communication::{auction::state::AuctionState, UserAccountData};
use yew::prelude::*;

use crate::screens::fullscreen_message::FullscreenMsg;

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
        AuctionState::AuctionOver => {
            html!(<FullscreenMsg message="Auction is now concluded" show_reload_button={false} user_account={props.account.clone()}/>)
        }
        AuctionState::WaitingForItem => {
            html!(<FullscreenMsg message="Waiting for item to be presented..." show_reload_button={true} user_account={props.account.clone()}/>)
        }
        _ => {
            html!(<FullscreenMsg message={format!("Current auction state is not implemented: {:?}", &props.state)} show_reload_button={true} user_account={props.account.clone()}/>)
        }
    }
}
