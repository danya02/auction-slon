use std::rc::Rc;

use common::{
    components::{MoneyDisplay, UserAccountTable},
    layout::VerticalStack,
};
use communication::{
    auction::{
        actions::JapaneseAuctionAction,
        state::{JapaneseAuctionBidState, Sponsorship},
    },
    UserClientMessage,
};
use yew::prelude::*;
use yew_hooks::*;

use crate::AppCtx;

#[derive(Properties, PartialEq)]
pub struct JapaneseAuctionBidInputProps {
    pub item_id: i64,
    pub state: JapaneseAuctionBidState,
}

#[function_component]
pub fn JapaneseAuctionBidInput(props: &JapaneseAuctionBidInputProps) -> Html {
    let ctx: Rc<AppCtx> = use_context().expect("no ctx found");
    let my_account = &ctx.my_account;
    let users = &ctx.users;
    let sponsorships = &ctx.sponsorships;
    let send = &ctx.send;

    let window = web_sys::window().expect("should have a window in this context");
    let performance = window
        .performance()
        .expect("performance should be available");

    let repress_delay = 1500.0;

    let available_balance = use_state(|| 0);
    {
        let available_balance = available_balance.clone();
        use_effect_with_deps(
            move |(user_id, users, sponsorships)| {
                available_balance.set(Sponsorship::resolve_available_balance(
                    *user_id,
                    &users,
                    &sponsorships,
                ));
            },
            (my_account.id, users.clone(), sponsorships.clone()),
        );
    }

    //let arena_is_open = matches!(props.state, JapaneseAuctionBidState::EnterArena { .. });
    let me_in_arena = props
        .state
        .get_arena()
        .iter()
        .any(|i| my_account.id == i.id);
    let locked_out_of_arena = match &props.state {
        JapaneseAuctionBidState::EnterArena { current_price, .. } => {
            current_price > &*available_balance
        } // If arena is open, we are locked out iff the current (initial) price is too expensive
        JapaneseAuctionBidState::ClockRunning {
            currently_in_arena, ..
        } => {
            !currently_in_arena // If clock is running, we are locked out when we are not in the arena anymore
                .iter()
                .any(|i| i.id == my_account.id)
        }
    };

    let arena_mode = props.state.get_arena_visibility_mode();

    let pressed = use_state(|| false);
    let changed_recently = use_state(|| false);
    let changed_at = use_state(|| performance.now());

    let trigger = use_force_update();
    // Rerender this once every 10 ms, to animate the progress bar
    use_interval(move || trigger.force_update(), 10);

    // If changed_recently, then the change occurred on the last render:
    // record the time.
    // This is because we need the performance object in order to do this,
    // and it's hard to pass it into each of the callbacks below.
    if *changed_recently {
        changed_recently.set(false);
        changed_at.set(performance.now());
    } else {
        // If not `changed_recently`, then the `changed_at` value is accurate, so now we can run logic for entering and exiting.

        // If the button is released, and it has been released for more than the timeout,
        // and we are still in the arena, then we want to exit the arena.
        // The loop will stop as soon as the server recognizes our exit.
        if (!*pressed) && ((performance.now() - *changed_at) > repress_delay) && me_in_arena {
            send.emit(UserClientMessage::JapaneseAuctionAction {
                item_id: props.item_id,
                action: JapaneseAuctionAction::ExitArena,
            });
        }

        // If the button is pressed, but we are not in the arena, and we could enter the arena,
        // then we want to enter the arena.
        // As before, the loop will stop as soon as the change is acknowledged.
        // (This also helps when the admin kicks us from the arena, but we still want to be there.
        //  If the admin kicks us during the re-press countdown, we do nothing.)
        // Also: if we are locked out because we don't have enough money, we'll not be able to enter here.
        if *pressed && !me_in_arena && !locked_out_of_arena {
            send.emit(UserClientMessage::JapaneseAuctionAction {
                item_id: props.item_id,
                action: JapaneseAuctionAction::EnterArena,
            });
        }
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
        //let changed_recently = changed_recently.clone();

        Callback::from(move |e: TouchEvent| {
            e.prevent_default();
            pressed.set(false);
            changed_recently.set(true);
        })
    };

    let pb_style = if locked_out_of_arena {
        // If we are locked out, show a mild danger color, indicating nothing to do.
        String::from("height: 100%; width: 100%; background-color: var(--bs-danger-bg-subtle); border-radius: inherit;")
    } else if *pressed {
        // If we are still pressing, show a success color.
        String::from("height: 100%; width: 100%; background-color: var(--bs-success); border-radius: inherit;")
    } else if me_in_arena {
        // If we are not pressing, but still in the arena, show a success color, but the box is shrinking relative to how much time is left.
        // The box below this one has a danger color, so it'll be filling up with the danger color.
        let time_until_quit = performance.now() - *changed_at;
        let fraction_remaining = 1.0 - (time_until_quit / repress_delay);
        format!(
            "width: 100%; height: {:.1}%; background-color: var(--bs-success); border-radius: inherit;",
            fraction_remaining * 100.0
        )
    } else {
        // If we are not pressing, and also not in the arena, then show a danger color.
        String::from("height: 100%; width: 100%; background-color: var(--bs-danger); border-radius: inherit;")
    };

    let currently_in_arena = props.state.get_arena();

    let header_line = match &props.state {
        JapaneseAuctionBidState::EnterArena {
            seconds_until_arena_closes,
            current_price,
            ..
        } => {
            let hold = if let Some(s) = seconds_until_arena_closes {
                format!("Hold button to bet: {s:.1} left")
            } else {
                String::from("Hold button to bet")
            };
            html!(
                <>
                    <h1>{hold}</h1>
                    <h2>{"Initial price: "}<MoneyDisplay money={current_price} />{"/ You can use:"}<MoneyDisplay money={*available_balance}/></h2>
                </>
            )
        }
        JapaneseAuctionBidState::ClockRunning { current_price, .. } => {
            if locked_out_of_arena {
                html!(
                    <>
                        <h1>{"Current price: "}<MoneyDisplay money={current_price} /></h1>
                        <h2>{"You are no longer taking part"}</h2>
                    </>
                )
            } else {
                html!(
                    <>
                        <h1>{"Current price: "}<MoneyDisplay money={current_price} /></h1>
                        <h2>{"Hold to keep betting, release to abandon"}{"/ You can use:"}<MoneyDisplay money={*available_balance}/></h2>
                    </>
                )
            }
        }
    };

    let arena_info = match arena_mode {
        communication::auction::state::ArenaVisibilityMode::Full => html!(
            <>
                <h3>{currently_in_arena.len()}{" members taking part"}</h3>
                <UserAccountTable accounts={currently_in_arena.to_vec()} users={users.clone()} sponsorships={sponsorships.clone()}/>
            </>
        ),
        communication::auction::state::ArenaVisibilityMode::OnlyNumber => html!(
            <h3>{currently_in_arena.len()}{" members taking part"}</h3>
        ),
        communication::auction::state::ArenaVisibilityMode::Nothing => {
            if me_in_arena {
                html!(<h3>{"You are taking part"}</h3>)
            } else {
                html!(<h3>{"You are not taking part"}</h3>)
            }
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
                style="width: 50vmin; height: 50vmin; border-radius: 5vmin;" class={classes!("unselectable", "bg-danger", if *pressed {"shadow-sm"} else {"shadow-lg"})}>

                // This is the progress bar div, which shrinks rapidly when the button is released to give a chance to re-press it.
                <div style={pb_style} />
            </div>

            <div class="overflow-scroll" style="height: 20vh; max-height: 20vh;">
                {arena_info}
            </div>
        </VerticalStack>
    }
}
