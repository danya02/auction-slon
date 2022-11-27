use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

use rand::prelude::*;
use warp::{http::StatusCode, Filter, Rejection, Reply};

struct SharedData {
    nonce: u32,
}

pub async fn run(ip: [u8; 4], port: u16) {
    // let ws_filter = warp::path("connect")
    //     .and(warp::ws())
    //     .map(|ws: warp::ws::Ws| ws.on_upgrade(|socket| ws_connected(socket)));
    //
    // let hello_world = warp::any().map(|| "Hello, World!");
    //
    // let hello_name = warp::path("hello")
    //     .and(warp::path::param())
    //     .map(|name: String| format!("Hello, {}", name));
    //
    // let routes = hello_name.or(ws_filter).or(hello_world);
    let shared: Arc<Mutex<_>> = Arc::new(Mutex::new(SharedData { nonce: 0 }));
    let shared_clone = Arc::clone(&shared);

    let nonce = warp::path("nonce").map(move || {
        let mut shared = shared_clone.lock().unwrap();
        shared.nonce = thread_rng().gen::<u32>();
        log::info!("sent nonce: {}", shared.nonce);
        warp::reply::with_header(
            format!("{}", shared.nonce),
            "Access-Control-Allow-Origin",
            "http://127.0.0.1:8080",
        )
    });

    let login_buyer =
        warp::path!("login" / "buyer")
            .and(warp::path::param())
            .map(|_hmac: String| {
                log::info!("buyer is trying to log in");
                warp::reply::with_status("OK", StatusCode::OK)
            });

    let shared_clone = Arc::clone(&shared);
    let check_nonce = warp::path("check").map(move || {
        let shared = shared_clone.lock().unwrap();
        format!("{}", shared.nonce)
    });

    let routes = nonce
        .or(login_buyer)
        .or(check_nonce)
        .recover(handle_rejection);
    warp::serve(routes).run((ip, port)).await;
}

async fn handle_rejection(_err: Rejection) -> Result<impl Reply, Infallible> {
    Ok(warp::reply::with_status(
        "BAD_REQUEST",
        StatusCode::BAD_REQUEST,
    ))
}

// async fn ws_connected(socket: warp::ws::WebSocket) {
//     println!("WebSocket connection");
//     use futures_util::{SinkExt, StreamExt};
//     let (mut sink, mut stream) = socket.split();
//
//     tokio::task::spawn(async move {
//         while let Some(Ok(msg)) = stream.next().await {
//             println!("received: {:?}", msg);
//             sink.send(msg).await.unwrap_or_else(|err| {
//                 eprintln!("WebSocket send error: {}", err);
//             })
//         }
//     });
// }
