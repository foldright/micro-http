use http::Method;
use micro_web::extract::{Form, Json};
use micro_web::filter::header;
use micro_web::router::{get, post};
use micro_web::{handler_fn, Router, Server};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct User {
    name: String,
    zip: String,
}

async fn simple_get(method: &Method, str: Option<String>, str2: Option<String>) -> String {
    println!("receive body: {}, {}", str.is_some(), str2.is_some());
    format!("receive from method: {}\r\n", method)
}

// curl -v  -H "Transfer-Encoding: chunked" -d "name=hello&zip=world&c=abc"  http://127.0.0.1:8080/
async fn simple_handler_form_data(method: &Method, Form(user): Form<User>) -> String {
    format!("receive from method: {}, receive use: {:#?}\r\n", method, user)
}

// curl -v  -H "Transfer-Encoding: chunked" -H 'Content-Type: application/json' -d '{"name":"hello","zip":"world"}'  http://127.0.0.1:8080/
async fn simple_handler_json_data(method: &Method, Json(user): Json<User>) -> String {
    format!("receive from method: {}, receive use: {:#?}\r\n", method, user)
}

async fn simple_handler_post(method: &Method, str: Option<String>, str2: Option<String>) -> String {
    println!("receive body: {}, {}", str.is_some(), str2.is_some());
    format!("receive from method: {}\r\n", method)
}

async fn simple_another_get(method: &Method, str: Option<String>, str2: Option<String>) -> String {
    println!("receive body: {}, {}", str.is_some(), str2.is_some());
    format!("receive from method: {}\r\n", method)
}

async fn default_handler() -> &'static str {
    "404 not found"
}

#[tokio::main]
async fn main() {
    let router = Router::builder()
        .route("/", get(handler_fn(simple_get)))
        .route(
            "/",
            post(handler_fn(simple_handler_form_data))
                .with(header(http::header::CONTENT_TYPE, mime::APPLICATION_WWW_FORM_URLENCODED.as_ref())),
        )
        .route(
            "/",
            post(handler_fn(simple_handler_json_data))
                .with(header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())),
        )
        .route("/", post(handler_fn(simple_handler_post)))
        .route("/4", get(handler_fn(simple_another_get)))
        .build();

    Server::builder()
        .router(router)
        .address("127.0.0.1:8080")
        .default_handler(handler_fn(default_handler))
        .build()
        .unwrap()
        .start()
        .await;
}
