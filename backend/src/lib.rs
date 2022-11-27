use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

use rand::prelude::*;
use warp::{
    http::{Response, StatusCode},
    Filter, Rejection, Reply,
};

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

    let nonce = warp::path("nonce").and(warp::get()).map(move || {
        let mut shared = shared_clone.lock().unwrap();
        shared.nonce = thread_rng().gen::<u32>();
        log::info!("sent nonce: {}", shared.nonce);
        Response::builder()
            .header("Access-Control-Allow-Origin", "http://127.0.0.1:8080")
            .body(format!("{}", shared.nonce))
            .unwrap()
    });

    let options = warp::any().and(warp::options()).map(|| {
        // warp::reply::with_header("", "Access-Control-Allow-Origin", "http://127.0.0.1:8080")
        Response::builder()
            .header("Access-Control-Allow-Origin", "http://127.0.0.1:8080")
            .header("Access-Control-Allow-Headers", "Content-Type")
            .body(warp::hyper::body::Body::empty())
            .unwrap()
    });

    let shared_clone = Arc::clone(&shared);
    let login_buyer = warp::path!("login" / "buyer")
        .and(warp::post())
        .and(warp::body::json())
        .map(move |data: common::shared::BuyerLoginData| {
            let shared = shared_clone.lock().unwrap();
            log::info!("buyer is trying to log in");
            log::debug!("{:?}", data);

            let hmac = common::crypto::hmac(&data.passcode, &format!("{}", shared.nonce));

            let num_diff_elements = hmac
                .iter()
                .zip(data.hmac.iter())
                .filter(|&(a, b)| a != b)
                .count();

            log::debug!("HMAC");
            log::debug!("passcode: {}", data.passcode);
            log::debug!("nonce: {}", shared.nonce);
            log::debug!("\treceived: {:?}", data.hmac);
            log::debug!("\tcalculated: {:?}", hmac);

            if num_diff_elements == 0 {
                Response::builder()
                    .header("Access-Control-Allow-Origin", "http://127.0.0.1:8080")
                    .status(StatusCode::OK)
                    .body(warp::hyper::body::Body::empty())
                    .unwrap()
            } else {
                Response::builder()
                    .header("Access-Control-Allow-Origin", "http://127.0.0.1:8080")
                    .status(StatusCode::UNAUTHORIZED)
                    .body(warp::hyper::body::Body::empty())
                    .unwrap()
            }
        });

    let shared_clone = Arc::clone(&shared);
    let check_nonce = warp::path("check").and(warp::get()).map(move || {
        let shared = shared_clone.lock().unwrap();
        format!("{}", shared.nonce)
    });

    let routes = nonce
        .or(options)
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
