use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

use warp::Filter;

mod data;
mod database;
mod handlers;

use data::internal::ServerState;
use database::Database;
use handlers::*;

pub use data::models;
pub use data::schema;

pub async fn run(ip: [u8; 4], port: u16) {
    let state = Arc::new(Mutex::new(ServerState::new()));
    let db = Arc::new(Mutex::new(Database::init()));

    let ws = warp::path("ws")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| ws.on_upgrade(|socket| handle_ws(socket)));

    let nonce = warp::path("nonce")
        .and(warp::get())
        .and(with_state(state.clone()))
        .then(handle_nonce);

    let login = warp::path("login")
        .and(warp::post())
        .and(warp::cookie("session"))
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and(with_db(db.clone()))
        .then(handle_login);

    // let auth = warp::path("auth")
    //     .and(warp::cookie("session"))
    //     .and(warp::tra)

    let api_paths = ws.or(nonce).or(login);

    let api = warp::path("api").and(api_paths);

    let file_path = warp::path::param().and(warp::get()).then(handle_file);

    let index = warp::get().then(handle_index);

    let routes = api.or(file_path).or(index).recover(handle_rejection);
    warp::serve(routes).run((ip, port)).await;
}

fn with_state(
    state: Arc<Mutex<ServerState>>,
) -> impl Filter<Extract = (Arc<Mutex<ServerState>>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&state))
}

fn with_db(
    db: Arc<Mutex<Database>>,
) -> impl Filter<Extract = (Arc<Mutex<Database>>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&db))
}
