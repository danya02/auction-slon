#![feature(async_closure)]

use std::{borrow::Cow, env, path::PathBuf};

use auction::AuctionSyncHandle;
use axum::{
    extract::{
        ws::{close_code, CloseCode, CloseFrame, Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
    routing::get,
    Router,
};
use communication::{decode, LoginRequest};
use sqlx::SqlitePool;
use test_data::make_test_data;
use tower_http::services::ServeDir;

#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

mod admin;
mod auction;
mod test_data;
mod user;

trait Ignorable {
    fn ignore(self);
}

impl<T, E> Ignorable for Result<T, E> {
    fn ignore(self) {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    dotenvy::from_path(PathBuf::from("backend/.env"))?;

    let pool_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL in .env file must be set to absolute path to SQLite database");
    let pool = SqlitePool::connect(&pool_url).await?;
    sqlx::migrate!().run(&pool).await?;

    // If there are no users, create test data.
    {
        if sqlx::query!("SELECT * FROM auction_user LIMIT 1")
            .fetch_optional(&pool)
            .await?
            .is_none()
        {
            warn!("Creating test data because database is empty!");
            make_test_data(&pool).await?;
        }
    }

    // This future will will listen for a termination signal,
    // and then close the pool.
    // This is needed to ensure that data gets written out to disk.
    let termination_fut = {
        let pool = pool.clone();
        async move {
            let mut sigterm_stream =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
            // Wait for whichever of these comes first.
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {},
                _ = sigterm_stream.recv() => {},
            }

            info!("Shutting down application due to signal!");

            pool.close().await;
            pool.close_event().await;
        }
    };

    let sync_handle = AuctionSyncHandle::new(pool).await;

    let app = Router::new()
        .route("/websocket", get(handle_websocket_connection))
        .route("/admin/websocket", get(handle_websocket_connection))
        .nest_service(
            "/admin",
            ServeDir::new("frontend/admin/dist").append_index_html_on_directories(true),
        )
        .nest_service(
            "/",
            ServeDir::new("frontend/user/dist").append_index_html_on_directories(true),
        )
        .with_state(sync_handle);

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .with_graceful_shutdown(termination_fut)
        .await
        .unwrap();

    Ok(())
}

async fn handle_websocket_connection(
    State(sync_handle): State<AuctionSyncHandle>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(async move |s| handle_socket(s, sync_handle).await)
}

pub async fn close_socket(mut socket: WebSocket, code: CloseCode, reason: &str) {
    #[allow(unused_must_use)]
    {
        socket
            .send(Message::Close(Some(CloseFrame {
                code,
                reason: Cow::from(reason.to_string()),
            })))
            .await;
        // We do not care whether this message is received, as we're closing the connection.
    }
    drop(socket);
}

async fn handle_socket(mut socket: WebSocket, sync_handle: AuctionSyncHandle) {
    while let Some(msg) = socket.recv().await {
        let msg = if let Ok(msg) = msg {
            msg
        } else {
            // client disconnected
            return;
        };
        match msg {
            Message::Binary(data) => {
                // At this time, we are expecting only a login request
                // Any other message would be an error.
                let login_req: Result<LoginRequest, _> = decode(&data);
                match login_req {
                    Err(e) => {
                        return close_socket(
                            socket,
                            close_code::PROTOCOL,
                            &format!("Error parsing login: {e}"),
                        )
                        .await;
                    }
                    Ok(req) => match req {
                        LoginRequest::AsAdmin { key } => {
                            match admin::handle_socket(socket, key, sync_handle).await {
                                Ok(_) => {}
                                Err(why) => error!("Handling socket failed: {why} {why:?}"),
                            };
                            return;
                        }
                        LoginRequest::AsUser { key } => {
                            match user::handle_socket(socket, key, sync_handle).await {
                                Ok(_) => {}
                                Err(why) => error!("Handling socket failed: {why} {why:?}"),
                            }
                            return;
                        }
                    },
                }
            }
            Message::Ping(_) => {}
            Message::Pong(_) => {}
            _ => {
                // Not expecting any other type of message (specifically text and close)
                return close_socket(
                    socket,
                    close_code::PROTOCOL,
                    "Only expected binary messages",
                )
                .await;
            }
        }
    }
}
