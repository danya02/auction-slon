use futures::{channel::mpsc::Sender, SinkExt, StreamExt};
use reqwasm::websocket::{futures::WebSocket, Message};

use wasm_bindgen_futures::spawn_local;

pub struct WebsocketService {
    pub tx: Sender<String>,
}

impl WebsocketService {
    pub fn new() -> Self {
        let path = if cfg!(debug_assertions) {
            "ws://127.0.0.1:3030/connect"
        } else {
            "..." // TODO: server path on release
        };

        let ws = WebSocket::open(path).unwrap();

        let (mut ws_tx, mut ws_rx) = ws.split();

        let (in_tx, mut in_rx) = futures::channel::mpsc::channel::<String>(1000);
        spawn_local(async move {
            while let Some(s) = in_rx.next().await {
                ws_tx
                    .send(Message::Text(s))
                    .await
                    .unwrap_or_else(|err| log::error!("WebSocket send error: {:?}", err));
            }
        });

        spawn_local(async move {
            while let Some(msg) = ws_rx.next().await {
                match msg {
                    Ok(Message::Text(data)) => {
                        log::debug!("From WebSocket as text: {}", data);
                    }
                    Ok(Message::Bytes(b)) => {
                        let decoded = std::str::from_utf8(&b);
                        if let Ok(val) = decoded {
                            log::debug!("From WebSocket as bytes: {}", val);
                        }
                    }
                    Err(e) => {
                        log::error!("WebSocket read error: {:?}", e)
                    }
                }
            }
            log::debug!("WebSocket closed");
        });

        Self { tx: in_tx }
    }
}
