use common::{
    components::{ItemDisplay, MoneyDisplay},
    layout::Container,
};
use communication::{
    auction::state::{AuctionItem, Sponsorship, SponsorshipStatus},
    encode, UserAccountData, UserAccountDataWithSecrets, UserClientMessage, UserSaleMode,
};
use yew::prelude::*;

use crate::components::bidding_screen::{
    sponsorship_edit::SponsorshipEdit, sponsorship_mode_set::SponsorshipModeSet,
};

#[derive(Properties, PartialEq)]
pub struct ShowItemProps {
    pub item: AuctionItem,
    pub my_account: UserAccountDataWithSecrets,
    pub users: Vec<UserAccountData>,
    pub sponsorships: Vec<Sponsorship>,
    pub send: Callback<Vec<u8>>,
}

#[function_component]
pub fn ShowItemBeforeBid(props: &ShowItemProps) -> Html {
    // These tabs at the top allow you to choose between betting and sponsoring mode.
    let mode_tabs = {
        let mode = props.my_account.sale_mode.clone();
        let set_bidding_cb = {
            let send = props.send.clone();
            Callback::from(move |e: MouseEvent| {
                e.prevent_default();
                send.emit(encode(&UserClientMessage::SetSaleMode(
                    UserSaleMode::Bidding,
                )));
            })
        };
        let set_sponsoring_cb = {
            let send = props.send.clone();
            Callback::from(move |e: MouseEvent| {
                e.prevent_default();
                send.emit(encode(&UserClientMessage::SetSaleMode(
                    UserSaleMode::Sponsoring,
                )));
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

    let i_am_bidding = props.my_account.sale_mode == UserSaleMode::Bidding;
    let screen = if i_am_bidding {
        let maybe_sponsor_balance = match Sponsorship::resolve_available_balance(
            props.my_account.id,
            &props.users,
            &props.sponsorships,
        ) {
            myself if myself == props.my_account.balance => {
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
            let rows = props
                .sponsorships
                .iter()
                .filter(|s| s.recepient_id == props.my_account.id)
                .filter(|s| s.status == SponsorshipStatus::Active)
                .map(|s| (s, props.users.iter().find(|u| u.id == s.donor_id)))
                .filter_map(|(s, u)| u.is_some().then(|| (s, u.unwrap())))
                .map(|(s, u)| {
                    let cancel_cb = {
                        let s_id = s.id;
                        let send = props.send.clone();
                        Callback::from(move |e: MouseEvent| {
                            e.prevent_default();
                            send.emit(encode(&UserClientMessage::SetSponsorshipStatus {
                                sponsorship_id: s_id,
                                status: SponsorshipStatus::Rejected,
                            }));
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
                <p>{"You have: "}<MoneyDisplay money={props.my_account.balance} /></p>
                {maybe_sponsor_balance}
                <SponsorshipModeSet my_account={props.my_account.clone()} send={props.send.clone()} users={props.users.clone()} sponsorships={props.sponsorships.clone()} />
                {sponsor_table}
            </Container>
        )
    } else {
        html!(
            <Container>
                <div class="alert alert-info">{"Item: "}{&props.item.name}{"; initial price: "}<MoneyDisplay money={props.item.initial_price} /></div>
                <SponsorshipEdit my_account={props.my_account.clone()} users={props.users.clone()} sponsorships={props.sponsorships.clone()} send={props.send.clone()} bid_state={None}/>
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
