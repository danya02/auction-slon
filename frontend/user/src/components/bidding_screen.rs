use std::rc::Rc;

use common::{
    components::{MoneyDisplay, UserAccountCard},
    layout::{Container, VerticalStack},
};
use communication::{
    auction::state::{BiddingState, JapaneseAuctionBidState},
    UserClientMessage, UserSaleMode,
};
use yew::prelude::*;

use crate::{
    components::bidding_screen::{
        sponsorship_edit::SponsorshipEdit, sponsorship_mode_set::SponsorshipModeSet,
    },
    AppCtx,
};

use {english::EnglishAuctionBidInput, japanese::JapaneseAuctionBidInput};

mod english;
mod japanese;
pub mod sponsorship_edit;
pub mod sponsorship_mode_set;

#[derive(Properties, PartialEq)]
pub struct BiddingScreenProps {
    pub bid_state: BiddingState,
}

#[function_component]
pub fn BiddingScreen(props: &BiddingScreenProps) -> Html {
    let ctx: Rc<AppCtx> = use_context().expect("no ctx found");
    let send = &ctx.send;
    let my_account = &ctx.my_account;

    let item = &props.bid_state.item;
    // These tabs at the top allow you to choose between betting and sponsoring mode.
    let mode_tabs = {
        let mode = my_account.sale_mode.clone();
        let set_bidding_cb = {
            let send = send.clone();
            Callback::from(move |e: MouseEvent| {
                e.prevent_default();
                send.emit(UserClientMessage::SetSaleMode(UserSaleMode::Bidding));
            })
        };
        let set_sponsoring_cb = {
            let send = send.clone();
            Callback::from(move |e: MouseEvent| {
                e.prevent_default();
                send.emit(UserClientMessage::SetSaleMode(UserSaleMode::Sponsoring));
            })
        };

        html!(
            <ul class="nav nav-tabs">
                <li class="nav-item">
                    <a href="#" onclick={set_bidding_cb} class={classes!("nav-link", (mode == UserSaleMode::Bidding).then_some("active"))}>
                        {"Делаю ставки"}
                    </a>
                </li>
                <li class="nav-item">
                    <a href="#" onclick={set_sponsoring_cb} class={classes!("nav-link", (mode == UserSaleMode::Sponsoring).then_some("active"))}>
                        {"Спонсирую других"}
                    </a>
                </li>
            </ul>
        )
    };

    let i_am_bidding = my_account.sale_mode == UserSaleMode::Bidding;

    let bid_ui = match &props.bid_state.active_bid {
        communication::auction::state::ActiveBidState::EnglishAuctionBid {
            current_bid_amount,
            current_bidder,
            minimum_increment,
            seconds_until_commit,
            max_millis_until_commit,
        } => {
            let bid_is_me = current_bidder.id == my_account.id;
            let english_screen = if i_am_bidding {
                html!(
                        <Container class={classes!(bid_is_me.then_some("bg-success"))}>
                            <VerticalStack>
                                <h1>
                                    {"Делаем ставки: "}{&item.name}
                                </h1>
                                <SponsorshipModeSet />
                                <p>
                                    {"Наибольшая ставка: "}<MoneyDisplay money={current_bid_amount} />
                                </p>
                                <UserAccountCard account={current_bidder.clone()} />
                                <EnglishAuctionBidInput item_id={item.id} current_bid={current_bid_amount} increment={minimum_increment} seconds_left={seconds_until_commit} {max_millis_until_commit} />
                            </VerticalStack>
                        </Container>
                )
            } else {
                html!(
                    <>
                        <div class="alert alert-info">
                            {"Продается: "}{&item.name}{"; "}
                            {"текущая ставка: "}
                            <MoneyDisplay money={current_bid_amount} />
                            {" от "}
                            {&current_bidder.user_name}
                        </div>
                        <SponsorshipEdit bid_state={props.bid_state.clone()}/>
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
                                {"Делаем ставки: "}{&item.name}
                            </h1>
                            <SponsorshipModeSet  />
                            <JapaneseAuctionBidInput item_id={item.id} state={state.clone()} />
                        </VerticalStack>
                    </Container>
                )
            } else {
                html!(
                    <Container>
                        <div class="alert alert-info">
                            {"Продается: "}{&item.name}{";"}
                            {
                                match state {
                                    JapaneseAuctionBidState::EnterArena {
                                        currently_in_arena,
                                        current_price,
                                        ..
                                    } => {
                                        html!(
                                            <>
                                            {"начальная цена: "}
                                            <MoneyDisplay money={current_price} />
                                            {"; ставок: "}{currently_in_arena.len()}
                                            </>
                                        )
                                    },
                                    JapaneseAuctionBidState::ClockRunning {
                                        currently_in_arena,
                                        current_price,
                                        ..
                                    } => {
                                        html!(
                                            <>
                                            {"текущая цена: "}
                                            <MoneyDisplay money={current_price} />
                                            {"; остается ставок: "}{currently_in_arena.len()}
                                            </>
                                        )
                                    },
                                }
                            }
                        </div>
                        <SponsorshipEdit bid_state={props.bid_state.clone()}/>
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
