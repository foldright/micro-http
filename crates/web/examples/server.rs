use http::Method;

use micro_web::{get, handler_fn, post, resource, Server};

#[tokio::main]
async fn main() {
    Server::builder()
        .default_handler(handler_fn(default_handler))
        .route(
            "/1",
            resource()
                    .when(get()).to(handler_fn(simple_handler))
                    .when(post()).to(handler_fn(simple_handler_3))
                    .build(),
        )
        .route("/2", resource().when(get()).to(handler_fn(simple_handler_2)).build())
        .build()
        .start()
        .await;
}

async fn simple_handler(method: &Method, str: Option<String>, str2: Option<String>) -> String {
    println!("receive body: {}, {}", str.is_some(), str2.is_some());
    format!("1: receive from method: {}\r\n", method)
}

async fn simple_handler_3(method: &Method, str: Option<String>, str2: Option<String>) -> String {
    println!("receive body: {}, {}", str.is_some(), str2.is_some());
    format!("3: receive from method: {}\r\n", method)
}

async fn simple_handler_2(method: &Method) -> String {
    format!("2: receive from method: {}\r\n", method)
}

async fn default_handler() -> &'static str {
    "404 not found"
}
