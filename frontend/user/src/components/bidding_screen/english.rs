use std::rc::Rc;

use common::components::MoneyDisplay;
use communication::{auction::state::Sponsorship, Money, UserClientMessage};
use yew::prelude::*;

use crate::AppCtx;

#[derive(Properties, PartialEq)]
pub struct EnglishAuctionBidInputProps {
    pub item_id: i64,
    pub current_bid: Money,
    pub increment: Money,
    pub seconds_left: f32,
    pub max_millis_until_commit: u128,
}

#[function_component]
pub fn EnglishAuctionBidInput(props: &EnglishAuctionBidInputProps) -> Html {
    let ctx: Rc<AppCtx> = use_context().expect("no ctx found");
    let my_account = &ctx.my_account;
    let users = &ctx.users;
    let sponsorships = &ctx.sponsorships;
    let send = &ctx.send;

    let selected_bid = use_state_eq(|| props.current_bid);

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
        let send = send.clone();
        let item_id = props.item_id;
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            send.emit(UserClientMessage::BidInEnglishAuction {
                item_id,
                bid_amount: *selected_bid,
            })
        })
    };

    let send_btn = if *selected_bid > *available_balance {
        html!(<button class="btn btn-lg btn-info-outline" disabled={true}>{"Не хватает денег!"}</button>)
    } else if *selected_bid <= props.current_bid {
        html!(<button class="btn btn-lg btn-info-outline" disabled={true}>{"Слишком низкая ставка!"}</button>)
    } else {
        html!(<button class="btn btn-lg btn-info" onclick={send_cb}>{"Отправить ставку: "}<MoneyDisplay money={*selected_bid} /></button>)
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
        // If the current value is equal to my available balance, cannot increase it.
        else if current_val >= *available_balance {
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

    let max_time = props.max_millis_until_commit as f32 / 1000.0;
    let seconds_left = props.seconds_left;
    let percent_left = (seconds_left / max_time) * 100.0;
    let percent_now = 100.0 - percent_left;
    let (pb_first_style, pb_second_style, pb_text) = if seconds_left < max_time {
        (
            format!("width: {percent_left:.0}%;"),
            format!("width: {percent_now:.0}%;"),
            format!("{seconds_left:.1}s"),
        )
    } else {
        (
            String::from("width: 100%;"),
            String::from("width: 0%;"),
            String::from("Пока нет ставки..."),
        )
    };

    html! {
        <>
            <div class="input-group input-group-lg mb-3" role="group">
                <button disabled={value_down==0} class={classes!("btn", if value_down==0 {"btn-danger-outline"} else {"btn-danger"})} onclick={bid_down}>{"-"}<MoneyDisplay money={value_down} /></button>
                <span class="input-group-text"><MoneyDisplay money={*selected_bid} /></span>
                <button disabled={value_up==0} class={classes!("btn", if value_up==0 {"btn-success-outline"} else {"btn-success"})} onclick={bid_up}>{"+"}<MoneyDisplay money={value_up} /></button>
            </div>
            <div class="d-grid mb-3">
                {send_btn}
            </div>
            <div class="progress-stacked mb-3">
                <div class="progress" style={pb_first_style}>
                    <div class="progress-bar progress-bar-striped progress-bar-animated">
                    </div>
                </div>
                <div class="progress" style={pb_second_style}>
                    <div class="progress-bar progress-bar-striped progress-bar-animated bg-danger">
                        {pb_text}
                    </div>
                </div>
            </div>
        </>
    }
}
