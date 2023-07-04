use crate::{
    components::UserAccountCard,
    layout::{Container, VerticalStack},
};
use communication::UserAccountData;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct FullscreenMsgProps {
    pub message: AttrValue,

    #[prop_or(false)]
    pub show_reload_button: bool,

    #[prop_or_default]
    pub user_account: Option<UserAccountData>,
}

#[function_component]
pub fn FullscreenMsg(props: &FullscreenMsgProps) -> Html {
    let maybe_reload_button = if props.show_reload_button {
        let reload = Callback::from(|_| {
            let window = web_sys::window().unwrap();
            let location = window.location();
            location.reload().unwrap();
        });
        html!(
            <button class="btn btn-warning" onclick={reload}>{"Reload"}</button>
        )
    } else {
        html!()
    };

    let maybe_user_data = match &props.user_account {
        Some(data) => html!(<UserAccountCard account={data.clone()} />),
        None => html!(),
    };

    html! {
        <Container>
            <VerticalStack>
                <h1>{&props.message}</h1>
                {maybe_user_data}
                {maybe_reload_button}
           </VerticalStack>
        </Container>
    }
}
