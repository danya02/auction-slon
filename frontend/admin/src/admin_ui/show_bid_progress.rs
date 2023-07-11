use common::{
    components::{ItemDisplay, MoneyDisplay, UserAccountCard, UserAccountTable},
    layout::Container,
};
use communication::{
    auction::state::{BiddingState, JapaneseAuctionBidState},
    AdminClientMessage, UserAccountData,
};
use yew::prelude::*;

use super::SendToServer;

#[derive(Properties, PartialEq)]
pub struct ShowBidProgressProps {
    pub bid_state: BiddingState,
    pub send: SendToServer,
}

#[function_component]
pub fn ShowBidProgress(props: &ShowBidProgressProps) -> Html {
    let bidding_on = html! {
        <>
            <h3>{"Bidding on this item:"}</h3>
            <ItemDisplay item={props.bid_state.item.clone()} />
        </>
    };
    let bid_state = match &props.bid_state.active_bid {
        communication::auction::state::ActiveBidState::EnglishAuctionBid {
            current_bid_amount,
            current_bidder,
            minimum_increment,
            seconds_until_commit,
        } => {
            html! {
                <>
                <p>{"Current bidder:"}</p>
                <UserAccountCard account={current_bidder.clone()} />
                <p>{"Current bid amount: "}<MoneyDisplay money={current_bid_amount} /></p>
                <p>{"Minimum bid increment: "}<MoneyDisplay money={minimum_increment} /></p>
                <p>{"Time remaining: "}{seconds_until_commit}</p>
                </>
            }
        }
        communication::auction::state::ActiveBidState::JapaneseAuctionBid(state) => {
            let item_id = props.bid_state.item.id;
            // This callback gets a UserAccountData, and returns a button for kicking that member from the arena
            let get_kick_btn_cb = {
                let send = props.send.clone();
                Callback::from(move |user: UserAccountData| {
                    // This is the inner generated callback, which actually performs the kick for this specific user
                    let kick_press_cb = {
                        let send = send.clone();
                        Callback::from(move |e: MouseEvent| {
                            e.prevent_default();
                            send.emit(AdminClientMessage::KickFromJapaneseAuction(
                                item_id, user.id,
                            ));
                        })
                    };

                    // This is the HTML for the button
                    html!(<button class="btn btn-danger" onclick={kick_press_cb}>{"Kick from arena"}</button>)
                })
            };

            let current_clock_rate = state.get_price_increase_rate();

            // These two callbacks change the clock rate
            let clock_rate_up_cb = {
                let send = props.send.clone();
                Callback::from(move |e: MouseEvent| {
                    e.prevent_default();
                    let new_clock_rate = current_clock_rate + 5;
                    send.emit(AdminClientMessage::SetJapaneseClockRate(new_clock_rate));
                })
            };
            let clock_rate_down_cb = {
                let send = props.send.clone();
                Callback::from(move |e: MouseEvent| {
                    e.prevent_default();
                    let new_clock_rate = current_clock_rate - 5;
                    send.emit(AdminClientMessage::SetJapaneseClockRate(new_clock_rate));
                })
            };

            match state {
                JapaneseAuctionBidState::EnterArena {
                    currently_in_arena,
                    seconds_until_arena_closes,
                    current_price,
                    current_price_increase_per_100_seconds,
                } => {
                    html! {
                        <>
                            <h1>{"Arena is now open"}</h1>
                            <p>{"Arena closes in: "}{seconds_until_arena_closes}</p>
                            <p>{"Current price: "}<MoneyDisplay money={current_price} /></p>
                            <p>
                                {"Current price increase rate: +"}
                                <MoneyDisplay money={current_price_increase_per_100_seconds}/>
                                {"/100 seconds"}
                                <button class="btn btn-danger" onclick={clock_rate_down_cb}>{"-"}</button>
                                <button class="btn btn-success" onclick={clock_rate_up_cb}>{"+"}</button>
                            </p>

                            <div class="overflow-scroll" style="height: 40vh; max-height: 40vh;">
                                <h3>{currently_in_arena.len()}{" members in arena"}</h3>
                                <UserAccountTable accounts={currently_in_arena.clone()} action_col_cb={get_kick_btn_cb} />
                            </div>
                        </>
                    }
                }
                JapaneseAuctionBidState::ClockRunning {
                    currently_in_arena,
                    current_price,
                    current_price_increase_per_100_seconds,
                } => html! {
                    <>
                        <h1>{"Arena is now closed"}</h1>
                        <p>{"Current price: "}<MoneyDisplay money={current_price} /></p>

                        <p>
                            {"Current price increase rate: +"}
                            <MoneyDisplay money={current_price_increase_per_100_seconds}/>
                            {"/100 seconds"}
                            <button class="btn btn-danger" onclick={clock_rate_down_cb}>{"-"}</button>
                            <button class="btn btn-success" onclick={clock_rate_up_cb}>{"+"}</button>
                        </p>

                        <div class="overflow-scroll" style="height: 20vh; max-height: 20vh;">
                            <h3>{currently_in_arena.len()}{" members in arena"}</h3>
                            <UserAccountTable accounts={currently_in_arena.clone()} action_col_cb={get_kick_btn_cb} />
                        </div>
                    </>
                },
            }
        }
    };
    let return_cb = {
        let send = props.send.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            send.emit(AdminClientMessage::StartAuction);
        })
    };

    html! {
        <Container>
            <div class="row justify-content-evenly mb-3">
                <div class="col-6">
                    {bidding_on}
                </div>
                <div class="col-6">
                    {bid_state}
                </div>
            </div>
            <div class="d-grid gap-2">
                <button onclick={return_cb} class="btn btn-danger">{"Return to item select"}</button>
            </div>

        </Container>
    }
}
