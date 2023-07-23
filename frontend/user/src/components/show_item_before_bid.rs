use std::rc::Rc;

use common::{
    components::{ItemDisplay, MoneyDisplay},
    layout::Container,
};
use communication::{
    auction::state::{AuctionItem, Sponsorship, SponsorshipStatus},
    UserClientMessage, UserSaleMode,
};
use yew::prelude::*;

use crate::{
    components::bidding_screen::{
        sponsorship_edit::SponsorshipEdit, sponsorship_mode_set::SponsorshipModeSet,
    },
    AppCtx,
};

#[derive(Properties, PartialEq)]
pub struct ShowItemProps {
    pub item: AuctionItem,
}

#[function_component]
pub fn ShowItemBeforeBid(props: &ShowItemProps) -> Html {
    let ctx: Rc<AppCtx> = use_context().expect("no ctx found");
    let send = &ctx.send;
    let users = &ctx.users;
    let sponsorships = &ctx.sponsorships;
    let my_account = &ctx.my_account;

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
                        {"Making bets"}
                    </a>
                </li>
                <li class="nav-item">
                    <a href="#" onclick={set_sponsoring_cb} class={classes!("nav-link", (mode == UserSaleMode::Sponsoring).then_some("active"))}>
                        {"Sponsoring others"}
                    </a>
                </li>
            </ul>
        )
    };

    let i_am_bidding = my_account.sale_mode == UserSaleMode::Bidding;
    let screen = if i_am_bidding {
        let maybe_sponsor_balance = match Sponsorship::resolve_available_balance(
            my_account.id,
            users,
            sponsorships,
        ) {
            myself if myself == my_account.balance => {
                // If nobody is sponsoring me, do not show any balance here
                html!()
            }
            other => {
                html!(
                    <p>{"Available balance (including sponsors): "}<MoneyDisplay money={other} /></p>
                )
            }
        };

        let sponsor_table = 'sponsortable: {
            let rows = sponsorships
                .iter()
                .filter(|s| s.recepient_id == my_account.id)
                .filter(|s| s.status == SponsorshipStatus::Active)
                .map(|s| (s, users.iter().find(|u| u.id == s.donor_id)))
                .filter_map(|(s, u)| u.is_some().then(|| (s, u.unwrap())))
                .map(|(s, u)| {
                    let cancel_cb = {
                        let s_id = s.id;
                        let send = send.clone();
                        Callback::from(move |e: MouseEvent| {
                            e.prevent_default();
                            send.emit(UserClientMessage::SetSponsorshipStatus {
                                sponsorship_id: s_id,
                                status: SponsorshipStatus::Rejected,
                            });
                        })
                    };
                    html!(
                        <tr>
                            <td>{&u.user_name}</td>
                            <td><MoneyDisplay money={
                                s.balance_remaining.min(u.balance)
                            } /></td>
                            <td>
                                <button class="btn btn-warning" onclick={cancel_cb}>
                                    {"🚫"}  // U+1F6AB NO ENTRY SIGN
                                </button>
                            </td>
                        </tr>
                    )
                })
                .collect::<Vec<_>>();

            // If there are no sponsors present, don't show the table header either.
            if rows.is_empty() {
                break 'sponsortable html!();
            }

            html!(
                <table class="table">
                    <thead>
                        <tr>
                            <th scope="col">{"Name"}</th>
                            <th scope="col">{"Added balance"}</th>
                            <th scope="col">{"Cancel"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for rows}
                    </tbody>
                </table>
            )
        };

        html!(
            <Container>
                <h1>{"Prepare to bid on item:"}</h1>
                <ItemDisplay item={props.item.clone()} />
                <p>{"You have: "}<MoneyDisplay money={my_account.balance} /></p>
                {maybe_sponsor_balance}
                <SponsorshipModeSet />
                {sponsor_table}
            </Container>
        )
    } else {
        html!(
            <Container>
                <div class="alert alert-info">{"Item: "}{&props.item.name}{"; initial price: "}<MoneyDisplay money={props.item.initial_price} /></div>
                <SponsorshipEdit bid_state={None}/>
            </Container>
        )
    };

    html! {
        <>
            {mode_tabs}
            {screen}
        </>
    }
}
