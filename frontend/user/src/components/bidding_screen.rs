use common::{
    components::{MoneyDisplay, UserAccountCard, UserAccountTable},
    layout::{Container, VerticalStack},
};
use communication::{
    auction::{
        actions::JapaneseAuctionAction,
        state::{BiddingState, JapaneseAuctionBidState},
    },
    encode, Money, UserAccountData, UserClientMessage,
};
use yew::prelude::*;
use yew_hooks::*;

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

#[derive(Properties, PartialEq)]
struct EnglishAuctionBidInputProps {
    pub item_id: i64,
    pub current_bid: Money,
    pub increment: Money,
    pub my_balance: Money,
    pub seconds_left: f32,
    pub send: Callback<Vec<u8>>,
}

#[function_component]
fn EnglishAuctionBidInput(props: &EnglishAuctionBidInputProps) -> Html {
    let selected_bid = use_state_eq(|| props.current_bid);

    {
        let selected_bid = selected_bid.clone();
        use_effect_with_deps(
            move |(new_bid, increment)| {
                // When there is a new bid amount,
                // and it is higher than the currently selected bid,
                // update the bid to be a minimum increment higher than that
                if *new_bid >= *selected_bid {
                    selected_bid.set(*new_bid + increment);
                }
            },
            (props.current_bid, props.increment),
        );
    }

    let send_cb = {
        let selected_bid = selected_bid.clone();
        let send = props.send.clone();
        let item_id = props.item_id;
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            send.emit(encode(&UserClientMessage::BidInEnglishAuction {
                item_id,
                bid_amount: *selected_bid,
            }))
        })
    };

    let send_btn = if *selected_bid > props.my_balance {
        html!(<button class="btn btn-info-outline" disabled={true}>{"Cannot afford!"}</button>)
    } else if *selected_bid <= props.current_bid {
        html!(<button class="btn btn-info-outline" disabled={true}>{"Too low!"}</button>)
    } else {
        html!(<button class="btn btn-info" onclick={send_cb}>{"Send bid: "}<MoneyDisplay money={*selected_bid} /></button>)
    };
    let current_val = *selected_bid;

    let value_down = {
        // If the current value is lower than (the current bid)+(minimum increment), cannot decrease bid at all
        if current_val <= (props.current_bid + props.increment) {
            0
        }
        // If the current value is above the minimum increment, can decrease it by any steps.
        else {
            1
        }
    };

    let value_up = {
        // If the current value is lower than (the current bid)+(minimum increment), must increase to at least minimum increment.
        if current_val < (props.current_bid + props.increment) {
            (props.current_bid + props.increment) - current_val
        }
        // If the current value is equal to my balance, cannot increase it.
        else if current_val >= props.my_balance {
            0
        }
        // Otherwise, can increase it by any amount
        else {
            1
        }
    };

    let bid_down = {
        let selected_bid = selected_bid.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            selected_bid.set(*selected_bid - value_down);
        })
    };

    let bid_up = {
        let selected_bid = selected_bid.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            selected_bid.set(*selected_bid + value_up);
        })
    };

    let max_time = 10.0;
    let seconds_left = props.seconds_left;
    let percent_left = (seconds_left / max_time) * 100.0;
    let pb_style = format!("width: {percent_left:.0}%;");
    let pb_text = format!("Time left: {seconds_left:.1}s");

    html! {
        <>
            <div class="input-group mb-3" role="group">
                <button disabled={value_down==0} class={classes!("btn", if value_down==0 {"btn-danger-outline"} else {"btn-danger"})} onclick={bid_down}>{"-"}<MoneyDisplay money={value_down} /></button>
                <span class="input-group-text"><MoneyDisplay money={*selected_bid} /></span>
                <button disabled={value_up==0} class={classes!("btn", if value_up==0 {"btn-success-outline"} else {"btn-success"})} onclick={bid_up}>{"+"}<MoneyDisplay money={value_up} /></button>
            </div>
            <div class="d-grid mb-3">
                {send_btn}
            </div>
            <div class="progress mb-3">
                <div class="progress-bar progress-bar-striped progress-bar-animated" style={pb_style}>{pb_text}</div>
            </div>
        </>
    }
}

#[derive(Properties, PartialEq)]
struct JapaneseAuctionBidInputProps {
    pub item_id: i64,
    pub state: JapaneseAuctionBidState,
    pub my_user_id: i64,
    pub my_balance: Money,
    pub send: Callback<Vec<u8>>,
}

#[function_component]
fn JapaneseAuctionBidInput(props: &JapaneseAuctionBidInputProps) -> Html {
    let window = web_sys::window().expect("should have a window in this context");
    let performance = window
        .performance()
        .expect("performance should be available");

    let repress_delay = 3000.0;

    let arena_is_open = matches!(props.state, JapaneseAuctionBidState::EnterArena { .. });
    let locked_out_of_arena = match &props.state {
        JapaneseAuctionBidState::EnterArena { .. } => false, // If arena is open, we are not locked out
        JapaneseAuctionBidState::ClockRunning {
            currently_in_arena, ..
        } => {
            currently_in_arena // If clock is running, we are locked out when we are not in the arena anymore
                .iter()
                .find(|i| i.id == props.my_user_id)
                .is_none()
        }
    };

    let pressed = use_state(|| false);
    let changed_recently = use_state(|| false);
    let changed_at = use_state(|| performance.now());

    let trigger = use_force_update();
    // Rerender this once every 10 ms, to animate the progress bar
    use_interval(move || trigger.force_update(), 10);

    // If changed_recently, then the change occurred on the last render:
    // record the time, and perform any needed changes.
    if *changed_recently {
        changed_recently.set(false);
        changed_at.set(performance.now());

        // If the arena is currently open, the change can be immediately sent
        if arena_is_open {
            let action = if *pressed {
                JapaneseAuctionAction::EnterArena
            } else {
                JapaneseAuctionAction::ExitArena
            };
            props
                .send
                .emit(encode(&UserClientMessage::JapaneseAuctionAction {
                    item_id: props.item_id,
                    action,
                }));
        } else {
            // If the arena is closed, we can only ever send an ExitArena.
            // Also, we should only do this if we are in the arena now,
            // and after a delay.
        }
    }

    // If the button is released, and it has been released for more than the timeout,
    // and we are still not locked out of the arena, then we want to exit the arena.
    // The loop will stop as soon as the server recognizes our exit.
    if !arena_is_open & !*pressed
        && (performance.now() - *changed_at) > repress_delay
        && !locked_out_of_arena
        && !*changed_recently
    {
        props
            .send
            .emit(encode(&UserClientMessage::JapaneseAuctionAction {
                item_id: props.item_id,
                action: JapaneseAuctionAction::ExitArena,
            }));
    }

    let down = {
        let pressed = pressed.clone();
        let changed_recently = changed_recently.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            pressed.set(true);
            changed_recently.set(true);
        })
    };
    let up = {
        let pressed = pressed.clone();
        let changed_recently = changed_recently.clone();

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            pressed.set(false);
            changed_recently.set(true);
        })
    };

    let downtouch = {
        let pressed = pressed.clone();
        let changed_recently = changed_recently.clone();

        Callback::from(move |e: TouchEvent| {
            e.prevent_default();
            pressed.set(true);
            changed_recently.set(true);
        })
    };

    let uptouch = {
        let pressed = pressed.clone();
        let changed_recently = changed_recently.clone();

        Callback::from(move |e: TouchEvent| {
            e.prevent_default();
            pressed.set(false);
            changed_recently.set(true);
        })
    };

    // If the arena is open, then the button's color directly indicates the press state.
    let pb_style = if arena_is_open {
        format!(
            "height: 100%; width: 100%; background-color: var(--bs-{}); border-radius: inherit;",
            if *pressed { "success" } else { "danger" }
        )
    } else if locked_out_of_arena {
        // If we are locked out, show a mild danger color, indicating nothing to do.
        String::from("height: 100%; width: 100%; background-color: var(--bs-danger-bg-subtle); border-radius: inherit;")
    } else if *pressed {
        // If we are still pressing, show a success color.
        String::from("height: 100%; width: 100%; background-color: var(--bs-success); border-radius: inherit;")
    } else {
        // If we are not pressing, but not yet locked out, show a success color, but the box is shrinking relative to how much time is left.
        // The box below this one has a danger color, so it'll be filling up with the danger color.
        let time_until_quit = performance.now() - *changed_at;
        let fraction_remaining = 1.0 - (time_until_quit / repress_delay);
        format!(
            "width: 100%; height: {:.1}%; background-color: var(--bs-success); border-radius: inherit;",
            fraction_remaining * 100.0
        )
    };

    let currently_in_arena = match &props.state {
        JapaneseAuctionBidState::EnterArena {
            currently_in_arena, ..
        } => currently_in_arena,
        JapaneseAuctionBidState::ClockRunning {
            currently_in_arena, ..
        } => currently_in_arena,
    };

    let header_line = match &props.state {
        JapaneseAuctionBidState::EnterArena {
            seconds_until_arena_closes,
            ..
        } => {
            html!(<h1>{format!("Press and hold to bet: {seconds_until_arena_closes:.1} left")}</h1>)
        }
        JapaneseAuctionBidState::ClockRunning { current_price, .. } => {
            html!(<h1>{"Release if too expensive: "}<MoneyDisplay money={current_price} /></h1>)
        }
    };

    html! {
        <VerticalStack>
            {header_line}

            // This is the click target.
            // It must not trigger select events on mobile,
            // so it must not contain any text or selectable items.
            <div
                onmousedown={down} onmouseup={up} ontouchstart={downtouch} ontouchend={uptouch.clone()} ontouchcancel={uptouch}
                style="width: 50vmin; height: 50vmin; border-radius: 5vmin;" class={classes!("unselectable", "bg-danger", if *pressed {"shadow-lg"} else {"shadow-sm"})}>

                // This is the progress bar div, which shrinks rapidly when the button is released to give a chance to re-press it.
                <div style={pb_style} />
            </div>

            <div class="overflow-scroll" style="height: 20vh; max-height: 20vh;">
                <h3>{currently_in_arena.len()}{" members in arena"}</h3>
                <UserAccountTable accounts={currently_in_arena.clone()} />
            </div>
        </VerticalStack>
    }
}
