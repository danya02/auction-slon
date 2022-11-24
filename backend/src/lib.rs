#![deny(warnings)]
use warp::Filter;

pub async fn run(ip: [u8; 4], port: u16) {
    let ws_filter = warp::path("connect")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| ws.on_upgrade(|socket| ws_connected(socket)));

    let hello_world = warp::any().map(|| "Hello, World!");

    let hello_name = warp::path("hello")
        .and(warp::path::param())
        .map(|name: String| format!("Hello, {}", name));

    let routes = hello_name.or(ws_filter).or(hello_world);
    warp::serve(routes).run((ip, port)).await;
}

async fn ws_connected(socket: warp::ws::WebSocket) {
    println!("WebSocket connection");
    use futures_util::{SinkExt, StreamExt};
    let (mut sink, mut stream) = socket.split();

    tokio::task::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            println!("received: {:?}", msg);
            sink.send(msg).await.unwrap_or_else(|err| {
                eprintln!("WebSocket send error: {}", err);
            })
        }
    });
}
