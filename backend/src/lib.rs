use std::convert::Infallible;

use rand::prelude::*;
use warp::{http::StatusCode, Filter, Rejection, Reply};

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

    let nonce = warp::path("nonce").map(|| {
        let nonce = thread_rng().gen::<u32>();
        log::info!("sent nonce: {}", nonce);
        warp::reply::with_header(
            format!("{}", nonce),
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

    let routes = nonce.or(login_buyer).recover(handle_rejection);
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
