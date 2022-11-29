use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

use rand::prelude::*;
use warp::{
    http::{header, Response, StatusCode},
    Filter, Rejection, Reply,
};

use serde::{Deserialize, Serialize};

use common::crypto::*;

struct ServerState {
    server_secret: [u8; 32],
}

impl ServerState {
    fn new() -> Self {
        let mut secret = [0; 32];
        thread_rng().fill_bytes(&mut secret);
        Self {
            server_secret: secret,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SessionCookie {
    nonce: [u8; 32],
}

impl SessionCookie {
    fn new() -> Self {
        Self {
            nonce: Default::default(),
        }
    }

    fn serialize_with_hmac(&self, state: &ServerState) -> String {
        let json_string = serde_json::to_string(&self).expect("could not serialize cookie");
        let mut base64_string = base64::encode_config(json_string, base64::URL_SAFE);
        let hmac = hmac(&state.server_secret, &base64_string);
        let hmac_string = base64::encode_config(&hmac, base64::URL_SAFE);
        base64_string.push('.');
        base64_string.push_str(&hmac_string);
        base64_string
    }

    fn serialize_as_set_cookie(&self, state: &ServerState) -> String {
        let mut out = "session=\"".to_string();
        out.push_str(&self.serialize_with_hmac(state));
        out.push_str("\"");
        out
    }

    fn deserialize_with_hmac(data: &str, state: &ServerState) -> Option<Self> {
        let dot_index = data.find('.')?;
        log::debug!("Dot is at index {}", dot_index);
        let (text, user_hmac) = data.split_at(dot_index);
        let user_hmac = user_hmac.replace(".", "");
        log::debug!("Parts are {:?} and {:?}", text, user_hmac);
        let user_hmac_bytes = base64::decode_config(&user_hmac, base64::URL_SAFE).ok()?;
        log::debug!("Cookie's HMAC is: {:x?}", user_hmac_bytes);
        let expected_hmac = hmac(&state.server_secret, &text);
        log::debug!("Expected HMAC is: {:x?}", expected_hmac);
        if !compare_digest(&user_hmac_bytes, &expected_hmac) {
            log::warn!("User's cookie has wrong HMAC");
            return None;
        }

        let json_bytes = base64::decode_config(&text, base64::URL_SAFE).ok()?;
        log::debug!("Cookie's HMAC is: {:x?}", user_hmac_bytes);
        let cookie = serde_json::from_slice(&json_bytes).ok()?;

        Some(cookie)
    }

    fn deserialize_as_cookie(data: &str, state: &ServerState) -> Option<Self> {
        for cookie in data.split(";") {
            log::debug!("Found cookie: {:?}", &cookie);
            let cookie = cookie.trim().to_string();
            if cookie.starts_with("session=") {
                let cookie = cookie.strip_prefix("session=")?.to_string();
                let cookie = cookie.replace("\"", "");
                log::debug!("This is the session cookie: {:?}", cookie);
                return Self::deserialize_with_hmac(&cookie, state);
            }
        }
        None
    }
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
    let state: Arc<Mutex<_>> = Arc::new(Mutex::new(ServerState::new()));
    let state_clone = Arc::clone(&state);

    let nonce = warp::path("nonce").and(warp::get()).map(move || {
        let shared = state_clone.lock().unwrap();
        let mut nonce = [0; 32];
        thread_rng().fill_bytes(&mut nonce);

        log::info!("sent nonce: {:x?}", nonce);
        let mut cookie = SessionCookie::new();
        cookie.nonce = nonce;

        Response::builder()
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "http://127.0.0.1:8080")
            .header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
            .header(header::SET_COOKIE, cookie.serialize_as_set_cookie(&shared))
            .body(nonce.to_vec())
            .unwrap()
    });

    let options = warp::any().and(warp::options()).map(|| {
        // warp::reply::with_header("", "Access-Control-Allow-Origin", "http://127.0.0.1:8080")
        Response::builder()
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "http://127.0.0.1:8080")
            .header(header::ACCESS_CONTROL_ALLOW_HEADERS, "Content-Type")
            .header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
            .body(warp::hyper::body::Body::empty())
            .unwrap()
    });

    let state_clone = Arc::clone(&state);
    let login_buyer = warp::path!("login" / "buyer")
        .and(warp::post())
        .and(warp::header::header("Cookie"))
        .and(warp::body::json())
        .map(
            move |header: String, data: common::shared::BuyerLoginData| {
                let state = state_clone.lock().unwrap();
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
            },
        );

    let routes = nonce.or(options).or(login_buyer).recover(handle_rejection);
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
