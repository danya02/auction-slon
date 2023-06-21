use axum::{
    routing::get,
    Router, extract::{WebSocketUpgrade, ws::WebSocket}, response::Response,
};
use tower_http::services::ServeDir;

#[allow(unused_imports)]
use tracing::{error, warn, info, debug, trace};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();

    // build our application with a single route
    let app = Router::new().route("/websocket", get(handle_websocket_connection))
    .nest_service("/admin", ServeDir::new("frontend/admin/dist").append_index_html_on_directories(true))
    .nest_service("/", ServeDir::new("frontend/user/dist").append_index_html_on_directories(true));

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handle_websocket_connection(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        let msg = if let Ok(msg) = msg {
            msg
        } else {
            // client disconnected
            return;
        };

        info!("Recv: {msg:?}");

        if socket.send(msg).await.is_err() {
            // client disconnected
            return;
        }
    }
}