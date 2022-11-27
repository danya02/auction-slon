use std::collections::HashMap;

use crate::Route;
use gloo_dialogs::alert;
use gloo_net::http::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

#[function_component(SellerLoginOption)]
fn seller_login() -> Html {
    let onclick = {
        let history = use_history().unwrap();
        Callback::from(move |_| history.clone().push(Route::Seller))
    };

    html! {
        <div>
            <button {onclick}>{ "As a seller" }</button>
        </div>
    }
}

#[function_component(BuyerLoginOption)]
fn buyer_login() -> Html {
    let passcode = use_state(|| String::new());
    let oninput = {
        let passcode = passcode.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            passcode.set(input.value());
        })
    };

    let onclick = {
        let passcode = passcode.clone();
        Callback::from(move |_| {
            let passcode = (*passcode).clone();
            spawn_local(async move {
                // Todo: get rid of all the unwraps
                let nonce =
                    if let Ok(resp) = Request::get("http://localhost:3030/nonce").send().await {
                        resp.text().await.unwrap()
                    } else {
                        panic!("\"nonce\" request failed");
                    };
                log::debug!("Received a nonce: {:?}", nonce);

                let hmac = common::crypto::hmac(&format!("{:?}", nonce), &passcode);

                let body = common::shared::BuyerLoginData { hmac, passcode };
                log::info!("{:?}", body);

                if let Ok(resp) = Request::post("http://localhost:3030/login/buyer")
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await
                {
                    if resp.status_text() == "OK" {
                        let history = use_history().unwrap();
                        history.clone().push(Route::Buyer)
                    } else {
                        alert("Failed to log in! Please check your pass code")
                    }
                }
            });
        })
    };

    html! {
        <div>
            <label for="buyer-code">{ "Enter the provided code" }</label>
            <input {oninput} type="text" name="buyer-code" />
            <button {onclick}>{ "As a buyer" }</button>
        </div>
    }
}

#[function_component(LoginPage)]
pub fn login() -> Html {
    html! {
        <div>
            <BuyerLoginOption />
            <SellerLoginOption />
        </div>
    }
}
