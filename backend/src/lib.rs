use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

use warp::{
    http::{header, Response},
    Filter,
};

mod business;
use business::*;

pub async fn run(ip: [u8; 4], port: u16) {
    let state = Arc::new(Mutex::new(ServerState::new()));

    let ws = warp::path("ws")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| ws.on_upgrade(|socket| handle_ws(socket)));

    let nonce = warp::path("nonce")
        .and(warp::get())
        .and(with_state(state.clone()))
        .then(handle_nonce);

    let options = warp::any().and(warp::options()).map(|| {
        Response::builder()
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "http://127.0.0.1:8080")
            .header(header::ACCESS_CONTROL_ALLOW_HEADERS, "Content-Type")
            .header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
            .body(warp::hyper::body::Body::empty())
            .unwrap()
    });

    let login = warp::path("login")
        .and(warp::post())
        .and(warp::header::header("Cookie"))
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .then(handle_login);

    let routes = nonce.or(options).or(login).or(ws).recover(handle_rejection);
    warp::serve(routes).run((ip, port)).await;
}

fn with_state(
    state: Arc<Mutex<ServerState>>,
) -> impl Filter<Extract = (Arc<Mutex<ServerState>>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&state))
}
