use common::{
    components::{ItemDisplay, MoneyDisplay, UserAccountCard, UserAccountTable},
    layout::Container,
};
use communication::{
    auction::state::{ArenaVisibilityMode, BiddingState, JapaneseAuctionBidState, Sponsorship},
    AdminClientMessage, Money, UserAccountData,
};
use yew::prelude::*;

use super::SendToServer;

#[derive(Properties, PartialEq)]
pub struct ShowBidProgressProps {
    pub bid_state: BiddingState,
    pub users: Vec<UserAccountData>,
    pub sponsorships: Vec<Sponsorship>,
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
            max_millis_until_commit,
        } => {
            let increase_bet_time_cb = {
                let send = props.send.clone();
                let mmuc = *max_millis_until_commit;
                Callback::from(move |e: MouseEvent| {
                    e.prevent_default();
                    send.emit(AdminClientMessage::SetEnglishAuctionCommitPeriod {
                        new_period_ms: mmuc + 5000,
                    })
                })
            };
            let decrease_bet_time_cb = {
                let send = props.send.clone();
                let mmuc = *max_millis_until_commit;
                Callback::from(move |e: MouseEvent| {
                    e.prevent_default();
                    send.emit(AdminClientMessage::SetEnglishAuctionCommitPeriod {
                        new_period_ms: mmuc - 1000,
                    })
                })
            };

            html! {
                <>
                <p>{"Current bidder:"}</p>
                <UserAccountCard account={current_bidder.clone()} />
                <p>{"Current bid amount: "}<MoneyDisplay money={current_bid_amount} /></p>
                <p>{"Minimum bid increment: "}<MoneyDisplay money={minimum_increment} /></p>
                <p>{"Time remaining: "}{seconds_until_commit}</p>
                <p>
                    {"Max bid time: "}{format!("{:.2}", *max_millis_until_commit as f32 / 1000.0)}
                    <button class="btn btn-danger" onclick={decrease_bet_time_cb}>{"Sub 1 second"}</button>
                    <button class="btn btn-success" onclick={increase_bet_time_cb}>{"Add 5 seconds"}</button>
                </p>
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
                    let cr = current_clock_rate as f64;
                    let new_crln = cr.ln() + 0.05;
                    let new_cr = new_crln.exp();
                    let new_clock_rate = new_cr.ceil() as Money;
                    send.emit(AdminClientMessage::SetJapaneseClockRate(new_clock_rate));
                })
            };
            let clock_rate_down_cb = {
                let send = props.send.clone();
                Callback::from(move |e: MouseEvent| {
                    e.prevent_default();
                    let cr = current_clock_rate as f64;
                    let new_crln = cr.ln() - 0.05;
                    let new_cr = new_crln.exp();
                    let new_clock_rate = new_cr.floor() as Money;
                    send.emit(AdminClientMessage::SetJapaneseClockRate(new_clock_rate));
                })
            };

            // These three callbacks set the visibility mode.
            let set_full_cb = {
                let send = props.send.clone();
                Callback::from(move |e: MouseEvent| {
                    e.prevent_default();
                    send.emit(AdminClientMessage::SetJapaneseVisibilityMode(
                        ArenaVisibilityMode::Full,
                    ));
                })
            };
            let set_only_number_cb = {
                let send = props.send.clone();
                Callback::from(move |e: MouseEvent| {
                    e.prevent_default();
                    send.emit(AdminClientMessage::SetJapaneseVisibilityMode(
                        ArenaVisibilityMode::OnlyNumber,
                    ));
                })
            };

            let set_nothing_cb = {
                let send = props.send.clone();
                Callback::from(move |e: MouseEvent| {
                    e.prevent_default();
                    send.emit(AdminClientMessage::SetJapaneseVisibilityMode(
                        ArenaVisibilityMode::Nothing,
                    ));
                })
            };

            match state {
                JapaneseAuctionBidState::EnterArena {
                    currently_in_arena,
                    seconds_until_arena_closes,
                    current_price,
                    current_price_increase_per_100_seconds,
                    arena_visibility_mode,
                } => {
                    let arena_closes = if let Some(s) = seconds_until_arena_closes {
                        html!(
                            <p>{"Arena closes in: "}{s}</p>
                        )
                    } else {
                        let start_closing_arena_cb = {
                            let send = props.send.clone();
                            Callback::from(move |e: MouseEvent| {
                                e.prevent_default();
                                send.emit(AdminClientMessage::StartClosingJapaneseArena);
                            })
                        };
                        html!(
                            <p>
                                <button class="btn btn-warning" onclick={start_closing_arena_cb}>
                                    {"Start closing arena"}
                                </button>
                            </p>
                        )
                    };
                    html! {
                        <>
                            <h1>{"Arena is now open"}</h1>
                            <p>{"Current price: "}<MoneyDisplay money={current_price} /></p>
                            {arena_closes}
                            <p>
                                {"Current price increase rate: +"}
                                <MoneyDisplay money={current_price_increase_per_100_seconds}/>
                                {"/100 seconds"}
                                <button class="btn btn-danger" onclick={clock_rate_down_cb}>{"-"}</button>
                                <button class="btn btn-success" onclick={clock_rate_up_cb}>{"+"}</button>
                            </p>

                            <p>{"Members can see the following info about the arena:"}</p>
                            <div class="btn-group">
                                <button class={classes!("btn", if matches!(arena_visibility_mode, ArenaVisibilityMode::Full){"btn-primary"} else {"btn-outline-primary"})} onclick={set_full_cb}>{"Full info"}</button>
                                <button class={classes!("btn", if matches!(arena_visibility_mode, ArenaVisibilityMode::OnlyNumber){"btn-primary"} else {"btn-outline-primary"})} onclick={set_only_number_cb}>{"Only number of members"}</button>
                                <button class={classes!("btn", if matches!(arena_visibility_mode, ArenaVisibilityMode::Nothing){"btn-primary"} else {"btn-outline-primary"})} onclick={set_nothing_cb}>{"Nothing"}</button>
                            </div>

                            <div class="overflow-scroll" style="height: 40vh; max-height: 40vh;">
                                <h3>{currently_in_arena.len()}{" members in arena"}</h3>
                                <UserAccountTable accounts={currently_in_arena.clone()} users={props.users.clone()} sponsorships={props.sponsorships.clone()} action_col_cb={get_kick_btn_cb} />
                            </div>
                        </>
                    }
                }
                JapaneseAuctionBidState::ClockRunning {
                    currently_in_arena,
                    current_price,
                    current_price_increase_per_100_seconds,
                    arena_visibility_mode,
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

                        <p>{"Members can see the following info about the arena:"}</p>
                        <div class="btn-group">
                            <button class={classes!("btn", if matches!(arena_visibility_mode, ArenaVisibilityMode::Full){"btn-primary"} else {"btn-outline-primary"})} onclick={set_full_cb}>{"Full info"}</button>
                            <button class={classes!("btn", if matches!(arena_visibility_mode, ArenaVisibilityMode::OnlyNumber){"btn-primary"} else {"btn-outline-primary"})} onclick={set_only_number_cb}>{"Only number of members"}</button>
                            <button class={classes!("btn", if matches!(arena_visibility_mode, ArenaVisibilityMode::Nothing){"btn-primary"} else {"btn-outline-primary"})} onclick={set_nothing_cb}>{"Nothing"}</button>
                        </div>

                        <div class="overflow-scroll" style="height: 20vh; max-height: 20vh;">
                            <h3>{currently_in_arena.len()}{" members in arena"}</h3>
                            <UserAccountTable accounts={currently_in_arena.clone()} users={props.users.clone()} sponsorships={props.sponsorships.clone()} action_col_cb={get_kick_btn_cb} />
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
