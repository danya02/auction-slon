use common::{
    components::{MoneyDisplay, UserAccountCard},
    layout::{Container, VerticalStack},
};
use communication::{auction::state::BiddingState, UserAccountData};
use yew::prelude::*;

use {english::EnglishAuctionBidInput, japanese::JapaneseAuctionBidInput};

mod english;
mod japanese;

#[derive(Properties, PartialEq)]
pub struct BiddingScreenProps {
    pub bid_state: BiddingState,
    pub my_account: UserAccountData,
    pub send: Callback<Vec<u8>>,
}

#[function_component]
pub fn BiddingScreen(props: &BiddingScreenProps) -> Html {
    let item = &props.bid_state.item;
    let bid_ui = match &props.bid_state.active_bid {
        communication::auction::state::ActiveBidState::EnglishAuctionBid {
            current_bid_amount,
            current_bidder,
            minimum_increment,
            seconds_until_commit,
        } => {
            let bid_is_me = current_bidder.id == props.my_account.id;
            html!(
                <Container class={classes!(bid_is_me.then_some("bg-success"))}>
                    <VerticalStack>
                        <h1>
                            {"Bidding on: "}{&item.name}
                        </h1>
                        <p>
                            {"Current top bid: "}<MoneyDisplay money={current_bid_amount} />
                        </p>
                        <UserAccountCard account={current_bidder.clone()} />
                        <EnglishAuctionBidInput item_id={item.id} current_bid={current_bid_amount} increment={minimum_increment} seconds_left={seconds_until_commit} send={props.send.clone()} my_balance={props.my_account.balance} />
                    </VerticalStack>
                </Container>
            )
        }
        communication::auction::state::ActiveBidState::JapaneseAuctionBid(state) => {
            html! {
                <Container>
                    <VerticalStack>
                        <h1>
                            {"Bidding on: "}{&item.name}
                        </h1>
                        <JapaneseAuctionBidInput item_id={item.id} send={props.send.clone()} my_balance={props.my_account.balance} state={state.clone()} my_user_id={props.my_account.id} />
                    </VerticalStack>
                </Container>
            }
        }
    };

    bid_ui
}
