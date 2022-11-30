use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

use crate::business::data::*;
use rand::prelude::*;
use warp::{
    http::{header, Response, StatusCode},
    Rejection, Reply,
};

pub async fn handle_nonce(state: Arc<Mutex<ServerState>>) -> Response<Vec<u8>> {
    let state = state.lock().unwrap();
    let mut nonce = [0; 32];
    thread_rng().fill_bytes(&mut nonce);

    log::info!("sent nonce: {:x?}", nonce);
    let mut cookie = SessionCookie::new();
    cookie.nonce = nonce;

    Response::builder()
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "http://127.0.0.1:8080")
        .header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
        .header(header::SET_COOKIE, cookie.serialize_as_set_cookie(&state))
        .body(nonce.to_vec())
        .unwrap()
}

pub async fn handle_login(
    header: String,
    data: common::shared::BuyerLoginData,
    state: Arc<Mutex<ServerState>>,
) -> Response<warp::hyper::Body> {
    let state = state.lock().unwrap();
    log::info!("buyer is trying to log in");
    log::debug!("{:?}", data);

    if let Some(cookie) = SessionCookie::deserialize_as_cookie(&header, &state) {
        let _expected_nonce_hmac = common::crypto::hmac(&cookie.nonce, &data.passcode);
        return Response::builder()
            .header("Access-Control-Allow-Origin", "http://127.0.0.1:8080")
            .header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
            .status(StatusCode::OK)
            .body(warp::hyper::body::Body::empty())
            .unwrap();
    }

    Response::builder()
        .header("Access-Control-Allow-Origin", "http://127.0.0.1:8080")
        .header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
        .status(StatusCode::UNAUTHORIZED)
        .body(warp::hyper::body::Body::empty())
        .unwrap()
}

pub async fn handle_rejection(_err: Rejection) -> Result<impl Reply, Infallible> {
    Ok(warp::reply::with_status(
        "BAD_REQUEST",
        StatusCode::BAD_REQUEST,
    ))
}

pub async fn handle_ws(ws_connection: warp::ws::WebSocket) {
    println!("WebSocket connection");
    use futures_util::{SinkExt, StreamExt};
    let (mut sink, mut stream) = ws_connection.split();

    tokio::task::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            println!("received: {:?}", msg);
            sink.send(msg).await.unwrap_or_else(|err| {
                eprintln!("WebSocket send error: {}", err);
            })
        }
    });
}
