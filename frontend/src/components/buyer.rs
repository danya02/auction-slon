use yew::prelude::*;

pub struct Buyer;

impl Component for Buyer {
    type Message = ();

    type Properties = ();

    fn create(_ctx: &yew::Context<Self>) -> Self {
        Self
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
