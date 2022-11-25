use crate::Route;
use gloo_dialogs::alert;
use reqwasm::http::*;
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
    let code = use_state(|| String::new());
    let oninput = {
        let curr_code = code.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            curr_code.set(input.value());
        })
    };

    let onclick = {
        Callback::from(move |_| {
            spawn_local(async move {
                if let Ok(resp) = Request::get("http://localhost:3030/nonce").send().await {
                    log::debug!("{:?}", resp.body());
                }
            });
            if false {
                let history = use_history().unwrap();
                log::info!("{:?}", code);
                history.clone().push(Route::Buyer)
            } else {
                alert("Not so fast!")
            }
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
