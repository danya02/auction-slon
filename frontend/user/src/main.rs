// These two attributes are needed for Yew function components and generated types
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::rc::Rc;

use auction_view::AuctionView;
use common::layout::{Container, VerticalStack};
use common::screens::fullscreen_message::FullscreenMsg;
use communication::auction::state::Sponsorship;
use communication::{auction::state::AuctionState, decode, encode, LoginRequest, ServerMessage};
use communication::{
    UserAccountData, UserAccountDataWithSecrets, UserClientMessage, WithTimestamp,
};
use gloo_storage::{SessionStorage, Storage};
use log::info;
use serde::Deserialize;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_hooks::prelude::*;

mod auction_view;
mod components;

#[derive(Deserialize, Debug)]
struct CloseState {
    pub code: u16,
    pub reason: String,
}

#[function_component(MainApp)]
fn main_app() -> Html {
    let loc = &use_location();
    let path = format!(
        "ws{}://{}/websocket",
        if loc.protocol == "https" { "s" } else { "" },
        loc.host,
    );

    let login_key: UseSessionStorageHandle<String> = use_session_storage("login_key".to_string());
    let login_key_value = &*login_key;
    let login_key_value = login_key_value
        .clone()
        .unwrap_or("INVALID LOGIN KEY".to_string());

    let close_state = use_state(|| None);

    let options = {
        let close_state = close_state.clone();
        UseWebSocketOptions {
            onopen: None,
            onmessage: None,
            onmessage_bytes: None,
            onerror: None,
            onclose: Some(Box::new(move |event| {
                let close_state_value: CloseState =
                    serde_wasm_bindgen::from_value(event.into()).unwrap();
                info!("Received close with {close_state_value:?}");
                close_state.set(Some(close_state_value));
            })),
            reconnect_limit: Some(u32::MAX), // Never give up!
            reconnect_interval: None,
            manual: None,
            protocols: None,
        }
    };
    let ws = use_websocket_with_options(path, options);
    {
        let ws = ws.clone();
        let state = ws.ready_state.clone();
        // Send a login packet whenever a connection is completed
        use_effect_with_deps(
            move |state| {
                if state == &UseWebSocketReadyState::Open {
                    ws.send_bytes(encode(&LoginRequest::AsUser {
                        key: login_key_value,
                    }))
                }
            },
            (*state).clone(),
        );
    }

    let auction_state =
        use_state(|| WithTimestamp::new_with_zero_time(AuctionState::WaitingForAuction));
    let user_account = use_state(|| None);
    let auction_members = use_state(|| WithTimestamp::new_with_zero_time(vec![]));
    let sponsorship_states = use_state(|| None);
    {
        let ws = ws.clone();
        let user_account = user_account.clone();
        let auction_members = auction_members.clone();
        let auction_state = auction_state.clone();
        let sponsorship_states = sponsorship_states.clone();

        // Receive message by depending on `ws.message_bytes`.
        use_effect_with_deps(
            move |message| {
                if let Some(message) = &**message {
                    match decode(message) {
                        Err(why) => eprintln!("Error receiving server message: {why}"),
                        Ok(msg) => match msg {
                            ServerMessage::YourAccount(acc) => user_account.set(Some(acc)),
                            ServerMessage::AuctionMembers(members) => auction_members.set(members),
                            ServerMessage::AuctionState(state) => auction_state.set(state),
                            ServerMessage::SponsorshipState(state) => {
                                sponsorship_states.set(Some(state))
                            }
                        },
                    }
                }
                || ()
            },
            ws.message_bytes,
        );
    }

    let send_cb = {
        let ws = ws.clone();
        Callback::from(move |data| ws.send_bytes(encode(&data)))
    };

    // If we closed with an unrecoverable error, do not attempt to reconnect;
    // instead erase the key used to log in, and show an error message suggesting to reload.
    // TODO: in the background, this
    match &*close_state {
        None => {}
        Some(CloseState { code, reason }) => {
            if code != &1006 && code != &1001 {
                // 1006 = the connection was abnormally lost.
                // 1001 = peer "went away": client navigated from page or server shutdown
                // Others = server closed connection

                // using gloo's SessionStorage to avoid rerenders
                if SessionStorage::get::<String>("login_key").is_err() {
                    // meaning we already deleted key
                    // evil hack: we cannot stop use_websocket from reconnecting
                    // other than by causing a panic while rendering a component,
                    // AND this panic must be caused at some time after the error message was presented
                    // (and its loop must be completed -> cannot panic on a rerender caused by initial render,
                    // because the initial render state would never get applied to the DOM)
                    panic!("Panic caused to stop reconnect loop in main component");
                }

                SessionStorage::delete("login_key");

                let msg = format!("WebSocket closed with: {code} {reason}");
                return html!(<FullscreenMsg message={msg} show_reload_button={true} />);
            }
        }
    };

    match *ws.ready_state {
        UseWebSocketReadyState::Open => {
            // We need to have the user info before continuing
            match (&*user_account, &*sponsorship_states) {
                (Some(acc), Some(sponsors)) => {
                    let ctx = AppCtx {
                        state: auction_state.data.clone(),
                        users: auction_members.data.clone(),
                        my_account: acc.clone(),
                        sponsorships: sponsors.data.clone(),
                        send: send_cb.clone(),
                    };
                    let ctx = Rc::new(ctx);
                    html!(
                        <ContextProvider<Rc<AppCtx>> context={ctx}>
                            <AuctionView />
                        </ContextProvider<Rc<AppCtx>>>
                    )
                }
                _ => {
                    html!(<FullscreenMsg message="Waiting for server to send initial info..." show_reload_button={true} />)
                }
            }
        }
        _ => {
            html!(<FullscreenMsg message={format!("WebSocket connection is not ready yet (state is {:?})", *ws.ready_state)} show_reload_button={true} />)
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct AppCtx {
    state: AuctionState,
    users: Vec<UserAccountData>,
    my_account: UserAccountDataWithSecrets,
    sponsorships: Vec<Sponsorship>,
    send: Callback<UserClientMessage>,
}

#[function_component(AppWrapper)]
fn app_wrapper() -> Html {
    // Using the raw API so that deleting the key does not cause a re-render.
    let login_key: Option<String> = SessionStorage::get("login_key").unwrap_or_default();

    let did_set_login_key = use_state(|| false);
    let pending_login_key = use_state(String::new);

    let pending_login_key_input = {
        let pending_login_key = pending_login_key.clone();
        Callback::from(move |e: InputEvent| {
            let event: Event = e.dyn_into().unwrap_throw();
            let event_target = event.target().unwrap_throw();
            let target: HtmlInputElement = event_target.dyn_into().unwrap_throw();
            pending_login_key.set(target.value());
        })
    };

    let pending_login_key_submit = {
        //let pending_login_key = pending_login_key.clone();
        let did_set_login_key = did_set_login_key.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            SessionStorage::set("login_key", (*pending_login_key).clone()).unwrap_throw();
            did_set_login_key.set(true);
        })
    };

    if !(*did_set_login_key) {
        // Either we need to retrieve the preset key, or we need to ask the user to provide one.
        if let Some(_key) = login_key {
            did_set_login_key.set(true);
            html!() // the line above should cause a rerender right away, which would fall through to the else clause.
        } else {
            // Show the login key entry box.
            html! {
                <Container><VerticalStack><div>
                    <h3>{"Please input your personal login code"}</h3>
                    <form class="input-group" onsubmit={pending_login_key_submit}>
                        <input type="text" class="form-control" oninput={pending_login_key_input}/>
                        <input type="submit" class="btn btn-outline-success" value="Login" />
                    </form>

                </div></VerticalStack></Container>
            }
        }
    } else {
        html!(<MainApp />)
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<AppWrapper>::new().render();
}
