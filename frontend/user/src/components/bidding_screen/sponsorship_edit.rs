use std::{collections::HashMap, rc::Rc};

use common::components::MoneyDisplay;
use communication::{
    auction::state::{BiddingState, Sponsorship, SponsorshipStatus},
    UserClientMessage,
};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::AppCtx;

#[derive(Properties, PartialEq)]
pub struct SponsorshipEditProps {
    pub bid_state: Option<BiddingState>,
}

#[function_component]
pub fn SponsorshipEdit(props: &SponsorshipEditProps) -> Html {
    let ctx: Rc<AppCtx> = use_context().expect("no ctx found");
    let my_account = &ctx.my_account;
    let users = &ctx.users;
    let sponsorships = &ctx.sponsorships;
    let send = &ctx.send;

    // Get the list of users who need to be highlighted.
    // In an English auction, it is the single user who's currently bidding.
    // In a Japanese auction, it is all the users who are in the arena,
    // or none if the arena users aren't visible.
    let users_to_highlight;
    if let Some(bid_state) = &props.bid_state {
        users_to_highlight = match &bid_state.active_bid {
            communication::auction::state::ActiveBidState::EnglishAuctionBid {
                current_bidder,
                ..
            } => Some(vec![current_bidder.id]),
            communication::auction::state::ActiveBidState::JapaneseAuctionBid(state) => match state
            {
                communication::auction::state::JapaneseAuctionBidState::EnterArena {
                    currently_in_arena,
                    arena_visibility_mode,
                    ..
                } => match arena_visibility_mode {
                    communication::auction::state::ArenaVisibilityMode::Full => {
                        Some(currently_in_arena.iter().map(|u| u.id).collect())
                    }
                    _ => None,
                },
                communication::auction::state::JapaneseAuctionBidState::ClockRunning {
                    currently_in_arena,
                    arena_visibility_mode,
                    ..
                } => match arena_visibility_mode {
                    communication::auction::state::ArenaVisibilityMode::Full => {
                        Some(currently_in_arena.iter().map(|u| u.id).collect())
                    }
                    _ => None,
                },
            },
        }
        .unwrap_or(vec![]);
    } else {
        users_to_highlight = vec![];
    }

    // Get the list of my own sponsorships.
    // Replace each with the newest version, or with the one that's Active.

    let mut my_sponsorships = HashMap::new();
    let my_sponsorships_full_count = sponsorships
        .iter()
        .filter(|s| s.donor_id == my_account.id)
        .count();
    for s in sponsorships {
        if s.donor_id != my_account.id {
            continue;
        }

        let existing_sponsorship = my_sponsorships.get(&s.recepient_id);
        match existing_sponsorship {
            None => {
                my_sponsorships.insert(s.recepient_id, s);
            }
            Some(old_s) => {
                if old_s.status == SponsorshipStatus::Active {
                    continue;
                }
                if old_s.id < s.id {
                    my_sponsorships.insert(s.recepient_id, s);
                }
            }
        };
    }

    // Get the list of members who are available for sponsoring, but we aren't sponsoring.
    let could_sponsor = users
        .iter()
        .filter(|u| u.is_accepting_sponsorships && !my_sponsorships.contains_key(&u.id));

    let could_sponsor = html! {
        <table class="table table-sm">
            <thead>
                <tr>
                    <th scope="col">{"Name"}</th>
                    <th scope="col">{"Own balance"}</th>
                    <th scope="col">{"Total balance with sponsors"}</th>
                </tr>
            </thead>
            <tbody>
                {
                    for
                    could_sponsor.map(|u| {
                        let available_balance = Sponsorship::resolve_available_balance(u.id, users, sponsorships);
                        html!(
                            <tr class={classes!(users_to_highlight.iter().any(|i| *i == u.id).then_some("table-active"))}>
                                <td>{&u.user_name}</td>
                                <td><MoneyDisplay money={u.balance} /></td>
                                <td><MoneyDisplay money={available_balance} /></td>
                            </tr>
                        )
                    })
                }
            </tbody>
        </table>
    };

    let mut my_sponsorships: Vec<_> = my_sponsorships.into_iter().collect();
    my_sponsorships.sort_by(|a, b| a.1.id.cmp(&b.1.id));

    let current_sponsorships = {
        let mut rows = vec![];

        for (rcpt_id, sponsorship) in my_sponsorships.iter() {
            let rcpt = users
                .iter()
                .find(|u| u.id == *rcpt_id)
                .expect("No user receiving sponsorship in list of members?");

            let sponsorship_id = sponsorship.id;
            let sponsorship_status = match sponsorship.status {
                SponsorshipStatus::Rejected => {
                    html!(<span class="text-bg-danger">{"Rejected by recepient"}</span>)
                }
                SponsorshipStatus::Retracted => {
                    html!(<span class="text-bg-warning">{"Retracted by you"}</span>)
                }
                SponsorshipStatus::Active => {
                    let retract_cb = {
                        let send = send.clone();
                        Callback::from(move |e: MouseEvent| {
                            e.prevent_default();
                            send.emit(UserClientMessage::SetSponsorshipStatus {
                                sponsorship_id,
                                status: SponsorshipStatus::Retracted,
                            })
                        })
                    };
                    let can_sub_100 = sponsorship.balance_remaining >= 100;
                    let can_sub_10 = sponsorship.balance_remaining >= 10;
                    let can_sub_1 = sponsorship.balance_remaining >= 1;

                    let set_cb = |what| {
                        let send = send.clone();
                        Callback::from(move |e: MouseEvent| {
                            e.prevent_default();
                            send.emit(UserClientMessage::SetSponsorshipBalance {
                                sponsorship_id,
                                balance: what,
                            })
                        })
                    };
                    let b = sponsorship.balance_remaining;

                    let sub_100_btn = if can_sub_100 {
                        html!(<button class="btn btn-danger" onclick={set_cb(b-100)}>{"-100"}</button>)
                    } else {
                        html!(<button class="btn btn-danger" disabled={true}>{"-100"}</button>)
                    };
                    let sub_10_btn = if can_sub_10 {
                        html!(<button class="btn btn-outline-danger" onclick={set_cb(b-10)}>{"-10"}</button>)
                    } else {
                        html!(<button class="btn btn-outline-danger" disabled={true}>{"-10"}</button>)
                    };
                    let sub_1_btn = if can_sub_1 {
                        html!(<button class="btn btn-outline-danger" onclick={set_cb(b-1)}>{"-1"}</button>)
                    } else {
                        html!(<button class="btn btn-outline-danger" disabled={true}>{"-1"}</button>)
                    };

                    html!(
                        <>
                            <div class="input-group">
                                {sub_100_btn}
                                {sub_10_btn}
                                {sub_1_btn}
                                <span class="input-group-text"><MoneyDisplay money={sponsorship.balance_remaining} /></span>
                                <button class="btn btn-outline-success" onclick={set_cb(b+1)}>{"+1"}</button>
                                <button class="btn btn-outline-success" onclick={set_cb(b+10)}>{"+10"}</button>
                                <button class="btn btn-success" onclick={set_cb(b+100)}>{"+100"}</button>
                                <span class="input-group-text" style="min-width: 5em"></span>
                                <button class="btn btn-warning" onclick={retract_cb}>
                                    {"🚫"}  // U+1F6AB NO ENTRY SIGN
                                </button>
                            </div>
                        </>
                    )
                }
            };

            rows.push(html!(
                <>
                    <tr class={classes!(users_to_highlight.iter().any(|i| i == rcpt_id).then_some("table-active"))}>
                        <td>{&rcpt.user_name}</td>
                        <td><MoneyDisplay money={Sponsorship::resolve_available_balance(*rcpt_id, users, sponsorships)} /></td>
                    </tr>
                    <tr class={classes!(users_to_highlight.iter().any(|i| i == rcpt_id).then_some("table-active"))}>
                        <td colspan="2">{sponsorship_status}</td>
                    </tr>
                </>
            ))
        }

        html!(
            <table class="table">
                <thead>
                    <tr>
                        <th scope="col">{"Name"}</th>
                        <th scope="col">{"Available balance"}</th>
                    </tr>
                </thead>
                <tbody>
                    {for rows}
                </tbody>
            </table>
        )
    };

    // This stores the value inside the input box.
    let sponsor_code_value = use_state(|| String::new());
    {
        // When the number of sponsors changes, it means that the code was entered correctly.
        // Therefore, we clear the input field.
        let sponsor_code_value = sponsor_code_value.clone();
        use_effect_with_deps(
            move |_| sponsor_code_value.set(String::new()),
            my_sponsorships_full_count,
        );
    }

    let sponsor_code_oninput_cb = {
        let sponsor_code_value = sponsor_code_value.clone();
        let send = send.clone();
        Callback::from(move |e: InputEvent| {
            let event: Event = e.dyn_into().unwrap_throw();
            let event_target = event.target().unwrap_throw();
            let target: HtmlInputElement = event_target.dyn_into().unwrap_throw();
            sponsor_code_value.set(target.value());
            send.emit(UserClientMessage::TryActivateSponsorshipCode(
                target.value(),
            ));
        })
    };

    let sponsor_code_input = html!(
        <input class="form-control" value={(*sponsor_code_value).clone()} type="tel" oninput={sponsor_code_oninput_cb} placeholder={"Sponsorship code..."}/>
    );

    html!(
        <>
        <h1>{"Your sponsorships:"}</h1>
        {current_sponsorships}
        <hr />
        <div class="card text-bg-success">
            <div class="card-body">
                <h2>{"Add a sponsorship:"}</h2>
                {sponsor_code_input}
            </div>
        </div>
        <hr />
        <h3>{"Users accepting sponsorships:"}</h3>
        {could_sponsor}
        </>
    )
}
