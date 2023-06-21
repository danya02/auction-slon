use std::borrow::Cow;

use axum::{
    extract::{
        ws::{close_code, CloseFrame, Message, WebSocket, CloseCode},
        WebSocketUpgrade,
    },
    response::Response,
    routing::get,
    Router,
};
use communication::{decode, LoginRequest};
use tower_http::services::ServeDir;

#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

mod admin;
mod user;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // build our application with a single route
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
        );

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handle_websocket_connection(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

pub async fn close_socket(mut socket: WebSocket, code: CloseCode, reason: &str) {
    #[allow(unused_must_use)]
    {
        socket
            .send(Message::Close(Some(CloseFrame {
                code: code,
                reason: Cow::from(reason.to_string()),
            })))
            .await;
        // We do not care whether this message is received, as we're closing the connection.
    }
    return;
} 

async fn handle_socket(mut socket: WebSocket) {
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
                        return close_socket(socket, close_code::PROTOCOL, &format!("Error parsing login: {e}")).await;
                    }
                    Ok(req) => match req {
                        LoginRequest::AsAdmin { key } => {
                            return admin::handle_socket(socket, key).await
                        }
                        LoginRequest::AsUser { key } => {
                            return user::handle_socket(socket, key).await
                        }
                    },
                }
            }
            Message::Ping(_) => {}
            Message::Pong(_) => {}
            _ => {
                // Not expecting any other type of message (specifically text and close)
                return close_socket(socket, close_code::PROTOCOL, "Only expected binary messages").await;
            }
        }
    }
}
