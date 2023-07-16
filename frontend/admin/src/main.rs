// These two attributes are needed for Yew function components and generated types
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use common::layout::{Container, VerticalStack};
use common::screens::fullscreen_message::FullscreenMsg;
use communication::{decode, encode, AdminClientMessage, AdminServerMessage, LoginRequest};
use gloo_storage::{SessionStorage, Storage};
use log::info;
use serde::Deserialize;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_hooks::prelude::*;

use crate::admin_ui::AdminUserInterface;

mod admin_ui;

#[derive(Deserialize, Debug)]
struct CloseState {
    pub code: u16,
    pub reason: String,
}

#[function_component(MainApp)]
fn main_app() -> Html {
    let loc = &use_location();
    let path = format!(
        "ws{}://{}/admin/websocket",
        if loc.protocol == "https" { "s" } else { "" },
        loc.host,
    );

    let login_key: UseSessionStorageHandle<String> =
        use_session_storage("admin_login_key".to_string());
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
                    ws.send_bytes(encode(&LoginRequest::AsAdmin {
                        key: login_key_value,
                    }))
                }
            },
            (*state).clone(),
        );
    }

    let auction_state = use_state_eq(|| None);
    let admin_state = use_state_eq(|| None);
    let auction_members = use_list(vec![]);
    let item_states = use_list(vec![]);

    {
        let ws = ws.clone();
        //let auction_members = auction_members.clone();
        let auction_state = auction_state.clone();
        let auction_members = auction_members.clone();
        let item_states = item_states.clone();
        let admin_state = admin_state.clone();
        // Receive message by depending on `ws.message_bytes`.
        use_effect_with_deps(
            move |message| {
                if let Some(message) = &**message {
                    match decode(message) {
                        Err(why) => eprintln!("Error receiving server message: {why}"),
                        Ok(msg) => match msg {
                            AdminServerMessage::AuctionMembers(members) => {
                                auction_members.set(members)
                            }
                            AdminServerMessage::AuctionState(state) => {
                                auction_state.set(Some(state))
                            }
                            AdminServerMessage::ItemStates(items) => item_states.set(items),
                            AdminServerMessage::AdminState(state) => admin_state.set(Some(state)),
                        },
                    }
                }
                || ()
            },
            ws.message_bytes,
        );
    }

    let send_cb: Callback<AdminClientMessage> = {
        let ws = ws.clone();
        Callback::from(move |data| ws.send_bytes(encode(&data)))
    };

    // If we closed with an unrecoverable error, do not attempt to reconnect;
    // instead erase the key used to log in, and show an error message suggesting to reload.
    match &*close_state {
        None => {}
        Some(CloseState { code, reason }) => {
            if code != &1006 && code != &1001 {
                // 1006 = the connection was abnormally lost.
                // 1001 = peer "went away": client navigated from page or server shutdown
                // Others = server closed connection

                // using gloo's SessionStorage to avoid rerenders
                if SessionStorage::get::<String>("admin_login_key").is_err() {
                    // meaning we already deleted key
                    // evil hack: we cannot stop use_websocket from reconnecting
                    // other than by causing a panic while rendering a component,
                    // AND this panic must be caused at some time after the error message was presented
                    // (and its loop must be completed -> cannot panic on a rerender caused by initial render,
                    // because the initial render state would never get applied to the DOM)
                    panic!("Panic caused to stop reconnect loop in main component");
                }

                SessionStorage::delete("admin_login_key");

                return html!(<FullscreenMsg message={format!("WebSocket closed with: {code} {reason}")} show_reload_button={true} />);
            }
        }
    };

    match *ws.ready_state {
        UseWebSocketReadyState::Open => {
            // We need to have the auction info before continuing
            match (&*auction_state, &*admin_state) {
                (Some(auction_state), Some(admin_state)) => {
                    html!(<AdminUserInterface auction_state={auction_state.clone()} admin_state={admin_state.clone()} send={send_cb} items={item_states.current().clone()} users={auction_members.current().clone()}/>)
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

#[function_component(AppWrapper)]
fn app_wrapper() -> Html {
    // Using the raw API so that deleting the key does not cause a re-render.
    let login_key: Option<String> = SessionStorage::get("admin_login_key").unwrap_or_default();

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
            SessionStorage::set("admin_login_key", (*pending_login_key).clone()).unwrap_throw();
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
                    <h3>{"Please input admin login code"}</h3>
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
