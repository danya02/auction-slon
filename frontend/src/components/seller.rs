use yew::prelude::*;

pub struct Seller;

impl Component for Seller {
    type Message = ();

    type Properties = ();

    fn create(_ctx: &yew::Context<Self>) -> Self {
        Self
    }

    fn view(&self, _ctx: &yew::Context<Self>) -> yew::Html {
        html! {
            <div>
                <h1>{ "You are a seller" }</h1>
                <h2>{ "Good job" }</h2>
                <h3>{ "Go sell stuff" }</h3>
            </div>
        }
    }
}
