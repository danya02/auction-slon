use common::{
    components::{MoneyDisplay, UserAccountCard},
    layout::{Container, VerticalStack},
};
use communication::{
    auction::state::{BiddingState, Sponsorship},
    encode, UserAccountData, UserAccountDataWithSecrets, UserClientMessage, UserSaleMode,
};
use yew::prelude::*;

use crate::components::bidding_screen::{
    sponsorship_edit::SponsorshipEdit, sponsorship_mode_set::SponsorshipModeSet,
};

use {english::EnglishAuctionBidInput, japanese::JapaneseAuctionBidInput};

mod english;
mod japanese;
pub mod sponsorship_edit;
pub mod sponsorship_mode_set;

#[derive(Properties, PartialEq)]
pub struct BiddingScreenProps {
    pub bid_state: BiddingState,
    pub my_account: UserAccountDataWithSecrets,
    pub users: Vec<UserAccountData>,
    pub sponsorships: Vec<Sponsorship>,
    pub send: Callback<Vec<u8>>,
}

#[function_component]
pub fn BiddingScreen(props: &BiddingScreenProps) -> Html {
    let item = &props.bid_state.item;
    // These tabs at the top allow you to choose between betting and sponsoring mode.
    let mode_tabs = {
        let mode = props.my_account.sale_mode.clone();
        let set_bidding_cb = {
            let send = props.send.clone();
            Callback::from(move |e: MouseEvent| {
                e.prevent_default();
                send.emit(encode(&UserClientMessage::SetSaleMode(
                    UserSaleMode::Bidding,
                )));
            })
        };
        let set_sponsoring_cb = {
            let send = props.send.clone();
            Callback::from(move |e: MouseEvent| {
                e.prevent_default();
                send.emit(encode(&UserClientMessage::SetSaleMode(
                    UserSaleMode::Sponsoring,
                )));
            })
        };

        html!(
            <ul class="nav nav-tabs">
                <li class="nav-item">
                    <a href="#" onclick={set_bidding_cb} class={classes!("nav-link", (mode == UserSaleMode::Bidding).then_some("active"))}>
                        {"Making bets"}
                    </a>
                </li>
                <li class="nav-item">
                    <a href="#" onclick={set_sponsoring_cb} class={classes!("nav-link", (mode == UserSaleMode::Sponsoring).then_some("active"))}>
                        {"Sponsoring others"}
                    </a>
                </li>
            </ul>
        )
    };

    let i_am_bidding = props.my_account.sale_mode == UserSaleMode::Bidding;

    let bid_ui = match &props.bid_state.active_bid {
        communication::auction::state::ActiveBidState::EnglishAuctionBid {
            current_bid_amount,
            current_bidder,
            minimum_increment,
            seconds_until_commit,
            max_millis_until_commit,
        } => {
            let bid_is_me = current_bidder.id == props.my_account.id;
            let english_screen = if i_am_bidding {
                html!(
                        <Container class={classes!(bid_is_me.then_some("bg-success"))}>
                            <VerticalStack>
                                <h1>
                                    {"Bidding on: "}{&item.name}
                                </h1>
                                <SponsorshipModeSet my_account={props.my_account.clone()} users={props.users.clone()} sponsorships={props.sponsorships.clone()} send={props.send.clone()} />
                                <p>
                                    {"Current top bid: "}<MoneyDisplay money={current_bid_amount} />
                                </p>
                                <UserAccountCard account={current_bidder.clone()} />
                                <EnglishAuctionBidInput item_id={item.id} current_bid={current_bid_amount} increment={minimum_increment} seconds_left={seconds_until_commit} {max_millis_until_commit} send={props.send.clone()} my_account={props.my_account.clone()} users={props.users.clone()} sponsorships={props.sponsorships.clone()} />
                            </VerticalStack>
                        </Container>
                )
            } else {
                html!(
                    <>
                        <div class="alert alert-info">{"current auction info"}</div>
                        <SponsorshipEdit my_account={props.my_account.clone()} users={props.users.clone()} sponsorships={props.sponsorships.clone()} send={props.send.clone()} bid_state={props.bid_state.clone()}/>
                    </>
                )
            };
            html!(
                <Container>
                    {mode_tabs}
                    {english_screen}
                </Container>
            )
        }
        communication::auction::state::ActiveBidState::JapaneseAuctionBid(state) => {
            let japanese_screen = if i_am_bidding {
                html!(
                    <Container>
                        <VerticalStack>
                            <h1>
                                {"Bidding on: "}{&item.name}
                            </h1>
                            <SponsorshipModeSet my_account={props.my_account.clone()} send={props.send.clone()} users={props.users.clone()} sponsorships={props.sponsorships.clone()} />
                            <JapaneseAuctionBidInput item_id={item.id} send={props.send.clone()} state={state.clone()} my_account={props.my_account.clone()} users={props.users.clone()} sponsorships={props.sponsorships.clone()} />
                        </VerticalStack>
                    </Container>
                )
            } else {
                html!(
                    <Container>
                        <div class="alert alert-info">{"current auction info"}</div>
                        <SponsorshipEdit my_account={props.my_account.clone()} users={props.users.clone()} sponsorships={props.sponsorships.clone()} send={props.send.clone()} bid_state={props.bid_state.clone()}/>
                    </Container>
                )
            };
            html! {
                <>
                    {mode_tabs}
                    {japanese_screen}
                </>
            }
        }
    };

    bid_ui
}
