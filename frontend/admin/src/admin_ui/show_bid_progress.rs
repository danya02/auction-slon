use std::rc::Rc;

use common::{
    components::{ItemDisplay, MoneyDisplay, UserAccountCard, UserAccountTable},
    layout::Container,
};
use communication::{
    auction::state::{ArenaVisibilityMode, BiddingState, JapaneseAuctionBidState},
    AdminClientMessage, Money, UserAccountData,
};
use yew::prelude::*;

use crate::AppCtx;

#[derive(Properties, PartialEq)]
pub struct ShowBidProgressProps {
    pub bid_state: BiddingState,
}

#[function_component]
pub fn ShowBidProgress(props: &ShowBidProgressProps) -> Html {
    let ctx: Rc<AppCtx> = use_context().expect("no ctx found");
    let users = &ctx.users;
    let send = &ctx.send;
    let sponsorships = &ctx.sponsorships;

    let bidding_on = html! {
        <>
            <h3>{"Продаем:"}</h3>
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
                let send = send.clone();
                let mmuc = *max_millis_until_commit;
                Callback::from(move |e: MouseEvent| {
                    e.prevent_default();
                    send.emit(AdminClientMessage::SetEnglishAuctionCommitPeriod {
                        new_period_ms: mmuc + 5000,
                    })
                })
            };
            let decrease_bet_time_cb = {
                let send = send.clone();
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
                <p>{"Текущая ставка:"}</p>
                <UserAccountCard account={current_bidder.clone()} />
                <p>{"Значение ставки: "}<MoneyDisplay money={current_bid_amount} /></p>
                <p>{"Инкремент ставки: "}<MoneyDisplay money={minimum_increment} /></p>
                <p>{"Остается времени: "}{seconds_until_commit}</p>
                <p>
                    {"Максимальное время: "}{format!("{:.2}", *max_millis_until_commit as f32 / 1000.0)}
                    <button class="btn btn-danger" onclick={decrease_bet_time_cb}>{"-1с"}</button>
                    <button class="btn btn-success" onclick={increase_bet_time_cb}>{"+5с"}</button>
                </p>
                </>
            }
        }
        communication::auction::state::ActiveBidState::JapaneseAuctionBid(state) => {
            let item_id = props.bid_state.item.id;
            // This callback gets a UserAccountData, and returns a button for kicking that member from the arena
            let get_kick_btn_cb = {
                let send = send.clone();
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
                    html!(<button class="btn btn-danger" onclick={kick_press_cb}>{"Удалить из арены"}</button>)
                })
            };

            let current_clock_rate = state.get_price_increase_rate();

            // These two callbacks change the clock rate
            let clock_rate_up_cb = {
                let send = send.clone();
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
                let send = send.clone();
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
                let send = send.clone();
                Callback::from(move |e: MouseEvent| {
                    e.prevent_default();
                    send.emit(AdminClientMessage::SetJapaneseVisibilityMode(
                        ArenaVisibilityMode::Full,
                    ));
                })
            };
            let set_only_number_cb = {
                let send = send.clone();
                Callback::from(move |e: MouseEvent| {
                    e.prevent_default();
                    send.emit(AdminClientMessage::SetJapaneseVisibilityMode(
                        ArenaVisibilityMode::OnlyNumber,
                    ));
                })
            };

            let set_nothing_cb = {
                let send = send.clone();
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
                            <p>{"Арена закрывается через: "}{s}</p>
                        )
                    } else {
                        let start_closing_arena_cb = {
                            let send = send.clone();
                            Callback::from(move |e: MouseEvent| {
                                e.prevent_default();
                                send.emit(AdminClientMessage::StartClosingJapaneseArena);
                            })
                        };
                        html!(
                            <p>
                                <button class="btn btn-warning" onclick={start_closing_arena_cb}>
                                    {"Начать закрывать арену"}
                                </button>
                            </p>
                        )
                    };
                    html! {
                        <>
                            <h1>{"Арена открыта"}</h1>
                            <p>{"Текущая цена: "}<MoneyDisplay money={current_price} /></p>
                            {arena_closes}
                            <p>
                                {"Текущая скорость увеличения: +"}
                                <MoneyDisplay money={current_price_increase_per_100_seconds}/>
                                {"/100 секунд"}
                                <button class="btn btn-danger" onclick={clock_rate_down_cb}>{"-"}</button>
                                <button class="btn btn-success" onclick={clock_rate_up_cb}>{"+"}</button>
                            </p>

                            <p>{"Пользователи видят следующее об арене:"}</p>
                            <div class="btn-group">
                                <button class={classes!("btn", if matches!(arena_visibility_mode, ArenaVisibilityMode::Full){"btn-primary"} else {"btn-outline-primary"})} onclick={set_full_cb}>{"Список"}</button>
                                <button class={classes!("btn", if matches!(arena_visibility_mode, ArenaVisibilityMode::OnlyNumber){"btn-primary"} else {"btn-outline-primary"})} onclick={set_only_number_cb}>{"Только количество"}</button>
                                <button class={classes!("btn", if matches!(arena_visibility_mode, ArenaVisibilityMode::Nothing){"btn-primary"} else {"btn-outline-primary"})} onclick={set_nothing_cb}>{"Ничего"}</button>
                            </div>

                            <div class="overflow-scroll" style="height: 40vh; max-height: 40vh;">
                                <h3>{currently_in_arena.len()}{" пользователей в арене"}</h3>
                                <UserAccountTable accounts={currently_in_arena.clone()} users={users.iter().map(|u| u.into()).collect::<Vec<_>>()} sponsorships={sponsorships.clone()} action_col_cb={get_kick_btn_cb} />
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
                        <h1>{"Арена закрыта"}</h1>
                        <p>{"Текущая цена: "}<MoneyDisplay money={current_price} /></p>

                        <p>
                            {"Текущая скорость увеличения: +"}
                            <MoneyDisplay money={current_price_increase_per_100_seconds}/>
                            {"/100 секунд"}
                            <button class="btn btn-danger" onclick={clock_rate_down_cb}>{"-"}</button>
                            <button class="btn btn-success" onclick={clock_rate_up_cb}>{"+"}</button>
                        </p>

                        <p>{"Пользователи видят следующее об арене:"}</p>
                        <div class="btn-group">
                            <button class={classes!("btn", if matches!(arena_visibility_mode, ArenaVisibilityMode::Full){"btn-primary"} else {"btn-outline-primary"})} onclick={set_full_cb}>{"Список"}</button>
                            <button class={classes!("btn", if matches!(arena_visibility_mode, ArenaVisibilityMode::OnlyNumber){"btn-primary"} else {"btn-outline-primary"})} onclick={set_only_number_cb}>{"Только количество"}</button>
                            <button class={classes!("btn", if matches!(arena_visibility_mode, ArenaVisibilityMode::Nothing){"btn-primary"} else {"btn-outline-primary"})} onclick={set_nothing_cb}>{"Ничего"}</button>
                        </div>

                        <div class="overflow-scroll" style="height: 20vh; max-height: 20vh;">
                            <h3>{currently_in_arena.len()}{" участников в арене"}</h3>
                            <UserAccountTable accounts={currently_in_arena.clone()} users={users.iter().map(|u| u.into()).collect::<Vec<_>>()} sponsorships={sponsorships.clone()} action_col_cb={get_kick_btn_cb} />
                        </div>
                    </>
                },
            }
        }
    };
    let return_cb = {
        let send = send.clone();
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
                <button onclick={return_cb} class="btn btn-danger">{"Вернуться к выбору предмета"}</button>
            </div>

        </Container>
    }
}
