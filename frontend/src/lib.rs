use yew::prelude::*;
use yew_router::prelude::*;
mod components;
use components::*;

#[derive(Debug, Clone, Copy, PartialEq, Routable)]
pub(crate) enum Route {
    #[at("/")]
    Home,
    #[at("/login")]
    Login,
    #[at("/buyer")]
    Buyer,
    #[at("/seller")]
    Seller,

    #[not_found]
    #[at("/404")]
    NotFound,
}

fn switch(routes: &Route) -> Html {
    match routes {
        Route::Home => html! {<HomePage />},
        Route::Login => html! {<LoginPage />},
        Route::Buyer => html! {<Buyer />},
        Route::Seller => html! {<Seller />},
        Route::NotFound => html! {<PageNotFound />},
    }
}

#[function_component(AuctionApp)]
pub fn app() -> Html {
    html! {
        <BrowserRouter>
            <Switch<Route> render={Switch::render(switch)} />
        </BrowserRouter>
    }
}
