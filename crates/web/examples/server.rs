use http::{Method};













use micro_web::{FnHandler, Server};



#[tokio::main]
async fn main() {
    Server::builder()
        .default_handler(FnHandler::new(default_handler))
        .route("/1", FnHandler::new(simple_handler))
        .route("/2", FnHandler::new(simple_handler_2))
        .build()
        .start().await;
}

async fn simple_handler(method: &Method, str: Option<String>, str2: Option<String>) -> String {
    println!("receive body: {}, {}", str.is_some(), str2.is_some());
    format!("1: receive from method: {}\r\n", method)
}

async fn simple_handler_2(method: &Method) -> String {
    format!("2: receive from method: {}\r\n", method)
}

async fn default_handler() -> &'static str {
    "404 not found"
}
