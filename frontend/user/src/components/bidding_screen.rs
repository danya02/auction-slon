use common::{
    components::{MoneyDisplay, UserAccountCard},
    layout::{Container, VerticalStack},
};
use communication::{
    auction::state::BiddingState, encode, Money, UserAccountData, UserClientMessage,
};
use yew::prelude::*;

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
        communication::auction::state::ActiveBidState::JapaneseAuctionBid(_) => {
            html!(<h1>{"Japanese auction bidding not implemented yet"}</h1>)
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
