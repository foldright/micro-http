use http::Method;
use micro_web::extract::{Form, Json};
use micro_web::filter::header;
use micro_web::wrapper::EncodeWrapper;
use micro_web::router::{get, post, Router};
use micro_web::{handler_fn, Server};
use serde::Deserialize;


async fn hello_world() -> &'static str {
    "hello world"
}

async fn default_handler() -> &'static str {
    "404 not found"
}

#[tokio::main]
async fn main() {
    let router = Router::builder()
        .route("/", get(handler_fn(hello_world)))
        .build();

    Server::builder()
        .router(router)
        .bind("127.0.0.1:3000")
        .default_handler(handler_fn(default_handler))
        .build()
        .unwrap()
        .start()
        .await;
}
