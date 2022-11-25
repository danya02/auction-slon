use crate::Route;
use yew::prelude::*;
use yew_router::prelude::*;

#[function_component(HomePage)]
pub fn home() -> Html {
    let history = use_history().unwrap();
    let onclick = Callback::from(move |_| history.push(Route::Login));

    html! {
        <div>
            <h1>{ "Welcome to Auction Slon" }</h1>
            <button {onclick}>{ "Log in" }</button>
        </div>
    }
}

#[function_component(PageNotFound)]
pub fn not_found() -> Html {
    let history = use_history().unwrap();
    let onclick = Callback::from(move |_| history.push(Route::Home));

    html! {
        <div>
            <h1>{ "Oops..." }</h1>
            <h2>{ "There is no such page" }</h2>
            <button {onclick}>{ "Country roads take me home" }</button>
        </div>
    }
}
