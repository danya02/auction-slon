use std::rc::Rc;

use common::components::MoneyDisplay;
use communication::{
    auction::state::{Sponsorship, SponsorshipStatus},
    UserClientMessage,
};
use yew::prelude::*;

use crate::AppCtx;

#[function_component]
pub fn SponsorshipModeSet() -> Html {
    let ctx: Rc<AppCtx> = use_context().expect("no ctx found");
    let my_account = &ctx.my_account;
    let users = &ctx.users;
    let sponsorships = &ctx.sponsorships;
    let send = &ctx.send;

    let my_sponsors_count = sponsorships
        .iter()
        .filter(|s| s.status == SponsorshipStatus::Active && s.recepient_id == my_account.id)
        .count();

    let available_balance =
        Sponsorship::resolve_available_balance(my_account.id, users, sponsorships);

    let sponsorship_code_display = if let Some(code) = &my_account.sponsorship_code {
        let refresh_code_cb = {
            let send = send.clone();
            Callback::from(move |e: MouseEvent| {
                e.prevent_default();
                send.emit(UserClientMessage::RegenerateSponsorshipCode);
            })
        };
        let close_sponsors_cb = {
            let send = send.clone();
            Callback::from(move |e: MouseEvent| {
                e.prevent_default();
                send.emit(UserClientMessage::SetIsAcceptingSponsorships(false));
            })
        };
        html!(<p>
                {"Sponsorship code: "}<code>{code}</code>
                <button class="btn btn-primary" onclick={refresh_code_cb}>{"Refresh"}</button>
                <button class="btn btn-outline-danger" onclick={close_sponsors_cb}>{"Disable new sponsors"}</button>
            </p>)
    } else {
        let open_sponsors_cb = {
            let send = send.clone();
            Callback::from(move |e: MouseEvent| {
                e.prevent_default();
                send.emit(UserClientMessage::SetIsAcceptingSponsorships(true));
            })
        };

        html!(<p>
                {"New sponsors disabled"}
                <button class="btn btn-primary" onclick={open_sponsors_cb}>{"Allow new sponsors"}</button>
            </p>)
    };

    let sponsors_data = html!(
        <p>{"Current sponsors: "}{my_sponsors_count}{"; total balance available for bids: "}<MoneyDisplay money={available_balance}/></p>
    );

    html!(
        <>
            {sponsorship_code_display}
            {sponsors_data}
        </>
    )
}
