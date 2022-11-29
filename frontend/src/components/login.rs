use crate::Route;
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

enum LoginMsg {
    BeginLogin,
    IsLoginSuccessful(bool),
    ReceivedNonce(Vec<u8>),
}

struct BuyerLoginOption {
    passcode_input_field: NodeRef,
}

impl Component for BuyerLoginOption {
    type Message = LoginMsg;

    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            passcode_input_field: NodeRef::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            LoginMsg::BeginLogin => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    if let Ok(resp) = Request::get("/api/nonce").send().await {
                        if let Ok(nonce) = resp.binary().await {
                            log::debug!("Received a nonce: {:?}", nonce);
                            link.send_message(LoginMsg::ReceivedNonce(nonce));
                        }
                    } else {
                        log::error!("Nonce request failed");
                        link.send_message(LoginMsg::IsLoginSuccessful(false));
                    }
                });
            }
            LoginMsg::IsLoginSuccessful(is_login_successful) => {
                if is_login_successful {
                    let history = ctx.link().history().unwrap();
                    history.push(Route::Buyer);
                }
            }
            LoginMsg::ReceivedNonce(nonce) => {
                log::info!("Login nonce: {:x?}", nonce);
                let link = ctx.link().clone();
                if let Some(passcode_input_el) =
                    self.passcode_input_field.cast::<HtmlInputElement>()
                {
                    let passcode = passcode_input_el.value();
                    let hmac = common::crypto::hmac(&nonce, &passcode);

                    let body = common::shared::BuyerLoginData { hmac, passcode };
                    log::info!("{:?}", body);
                    spawn_local(async move {
                        if let Ok(resp) = Request::post("/api/login/buyer")
                            .header("Content-Type", "application/json")
                            .body(body)
                            .credentials(RequestCredentials::Include)
                            .send()
                            .await
                        {
                            link.send_message(LoginMsg::IsLoginSuccessful(resp.ok()));
                        }
                    });
                }
            }
        }
        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div>
                <label for="buyer-code">{ "Enter the provided code" }</label>
                <input ref={self.passcode_input_field.clone()} type="text" name="buyer-code" />
                <button onclick={ctx.link().callback(|_| LoginMsg::BeginLogin)}>{ "As a buyer" }</button>
            </div>
        }
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
