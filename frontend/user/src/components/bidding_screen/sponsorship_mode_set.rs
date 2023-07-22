use common::components::MoneyDisplay;
use communication::{
    auction::state::{Sponsorship, SponsorshipStatus},
    encode, UserAccountData, UserAccountDataWithSecrets, UserClientMessage,
};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SponsorshipModeSetProps {
    pub my_account: UserAccountDataWithSecrets,
    pub users: Vec<UserAccountData>,
    pub sponsorships: Vec<Sponsorship>,
    pub send: Callback<Vec<u8>>,
}

#[function_component]
pub fn SponsorshipModeSet(props: &SponsorshipModeSetProps) -> Html {
    let my_sponsors_count = props
        .sponsorships
        .iter()
        .filter(|s| s.status == SponsorshipStatus::Active && s.recepient_id == props.my_account.id)
        .count();

    let available_balance = Sponsorship::resolve_available_balance(
        props.my_account.id,
        &props.users,
        &props.sponsorships,
    );

    let sponsorship_code_display = if let Some(code) = &props.my_account.sponsorship_code {
        let refresh_code_cb = {
            let send = props.send.clone();
            Callback::from(move |e: MouseEvent| {
                e.prevent_default();
                send.emit(encode(&UserClientMessage::RegenerateSponsorshipCode));
            })
        };
        let close_sponsors_cb = {
            let send = props.send.clone();
            Callback::from(move |e: MouseEvent| {
                e.prevent_default();
                send.emit(encode(&UserClientMessage::SetIsAcceptingSponsorships(
                    false,
                )));
            })
        };
        html!(<p>
                {"Sponsorship code: "}<code>{code}</code>
                <button class="btn btn-primary" onclick={refresh_code_cb}>{"Refresh"}</button>
                <button class="btn btn-outline-danger" onclick={close_sponsors_cb}>{"Disable new sponsors"}</button>
            </p>)
    } else {
        let open_sponsors_cb = {
            let send = props.send.clone();
            Callback::from(move |e: MouseEvent| {
                e.prevent_default();
                send.emit(encode(&UserClientMessage::SetIsAcceptingSponsorships(true)));
            })
        };

        html!(<p>
                {"New sponsors disabled"}
                <button class="btn btn-primary" onclick={open_sponsors_cb}>{"Allow new sponsors"}</button>
            </p>)
    };

    let sponsors_data = html!(
        <p>{"Current sponsors: "}{my_sponsors_count}{"; total balance available for bids:"}<MoneyDisplay money={available_balance}/></p>
    );

    html!(
        <>
            {sponsorship_code_display}
            {sponsors_data}
        </>
    )
}
