use micro_web::wrapper::DateWrapper;
use micro_web::router::{get, Router};
use micro_web::{handler_fn, Server};


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
        .wrap(DateWrapper)
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
