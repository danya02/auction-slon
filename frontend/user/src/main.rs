use yew::prelude::*;
use yew_hooks::prelude::*;

#[function_component(App)]
fn app() -> Html {
    let history = use_list(vec![]);

    let loc = &use_location();
    let path = format!(
        "ws{}://{}/websocket",
        if loc.protocol == "https" { "s" } else { "" },
        loc.host,
    );

    let ws = use_websocket(path);
    let onclick = {
        let ws = ws.clone();
        let history = history.clone();
        Callback::from(move |_| {
            let message = "Hello, world!".to_string();
            ws.send(message.clone());
            history.push(format!("[send]: {}", message));
        })
    };
    {
        let history = history.clone();
        let ws = ws.clone();
        // Receive message by depending on `ws.message`.
        use_effect_with_deps(
            move |message| {
                if let Some(message) = &**message {
                    history.push(format!("[recv]: {}", message.clone()));
                }
                || ()
            },
            ws.message,
        );
    }

    html! {
        <>
            <p>
                <button {onclick} disabled={*ws.ready_state != UseWebSocketReadyState::Open}>{ "Send" }</button>
            </p>
            <p>
                <b>{ "Message history: " }</b>
            </p>
            {
                for history.current().iter().map(|message| {
                    html! {
                        <p>{ message }</p>
                    }
                })
            }
        </>
    }
}
fn main() {
    yew::Renderer::<App>::new().render();
}
