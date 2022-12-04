use gloo_net::http::*;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

pub struct Buyer;

pub enum BuyerMsg {
    PageLoad,
}

impl Component for Buyer {
    type Message = BuyerMsg;

    type Properties = ();

    fn create(ctx: &yew::Context<Self>) -> Self {
        ctx.link().send_message(BuyerMsg::PageLoad);
        Self
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            BuyerMsg::PageLoad => {
                spawn_local(async move {
                    if let Ok(_resp) = Request::get("/api/auth")
                        .credentials(RequestCredentials::Include)
                        .send()
                        .await
                    {
                        todo!("Verify user")
                    }
                });
            }
        }
        false
    }

    fn view(&self, _ctx: &yew::Context<Self>) -> yew::Html {
        html! {
            <div>
                <h1>{ "You are a buyer" }</h1>
                <h2>{ "Congrats" }</h2>
                <h3>{ "Go spend some money" }</h3>
            </div>
        }
    }
}
