#![deny(warnings)]
use warp::Filter;

pub async fn run(ip: [u8; 4], port: u16) {
    // Match any request and return hello world!
    let routes = warp::any().map(|| "Hello, World!");
    warp::serve(routes).run((ip, port)).await;
}
