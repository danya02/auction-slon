use web_sys::HtmlInputElement;
use yew::prelude::*;
mod websocket;
use websocket::WebsocketService;
use yew::html::NodeRef;

enum Msg {
    Send,
    Receive,
}

struct EchoClient {
    text: String,
    wss: WebsocketService,
    input_ref: NodeRef,
}

impl Component for EchoClient {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            text: String::new(),
            wss: WebsocketService::new(),
            input_ref: NodeRef::default(),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Send => {
                if let Some(input) = self.input_ref.cast::<HtmlInputElement>() {
                    let text = input.value();
                    log::info!("Seinding message {}", text);
                    if let Err(e) = self.wss.tx.try_send(text) {
                        log::error!("Error sending to channel: {:?}", e);
                    }
                    input.set_value("");
                }
                false
            }
            Msg::Receive => todo!(),
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        // let oninput = ctx.link().callback(|e: InputEvent| {
        //     let input: HtmlInputElement = e.target_unchecked_into();
        //     Msg::CaptureInput(input)
        // });

        html! {
            <div>
                <label for="text-input">{ "Enter some text:" }</label>
                <input ref={self.input_ref.clone()} type="text" name="text-input"/>
                <button onclick={ctx.link().callback(|_| Msg::Send)}>{ "Send" }</button>
                // <label for="echo-fileld">{ "Echo from server:" }</label>
                // <p>{ &self.text }</p>
            </div>
        }
    }
}

pub fn run() {
    yew::start_app::<EchoClient>();
}
