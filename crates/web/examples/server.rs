use http::{Method, Request, Response, StatusCode};
use std::error::Error;

use async_trait::async_trait;
use matchit::{Params, Router};
use std::sync::Arc;

use tokio::net::TcpListener;

use micro_http::connection::HttpConnection;

use micro_http::handler::{make_handler, Handler};
use micro_http::protocol::body::ReqBody;
use micro_http::protocol::{ParseError, RequestHeader};
use micro_web::{FnHandler, FromRequest, OptionReqBody, PathParams, ResponseBody, Server};
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    Server::builder()
        .default_handler(FnHandler::new(default_handler))
        .route("/1", FnHandler::new(simple_handler))
        .route("/2", FnHandler::new(simple_handler_2))
        .build()
        .start().await;
}

async fn simple_handler(method: Method, str: Option<String>, str2: Option<String>) -> String {
    println!("receive body: {}, {}", str.is_some(), str2.is_some());
    format!("1: receive from method: {}\r\n", method)
}

async fn simple_handler_2(method: Method) -> String {
    format!("2: receive from method: {}\r\n", method)
}

async fn default_handler() -> &'static str {
    "404 not found"
}
