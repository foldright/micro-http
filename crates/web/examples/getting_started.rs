//! Getting Started Example
//!
//! This example demonstrates the basic usage of the micro_web framework, including:
//! - Route handling with different HTTP methods
//! - Form and JSON data extraction
//! - Request filtering
//! - Response encoding
//! - Default handler setup
//!
//! To run this example:
//! ```bash
//! cargo run --example getting_started
//! ```

use http::Method;
use micro_web::extract::{Form, Json};
use micro_web::router::filter::header;
use micro_web::router::{get, post, Router};
use micro_web::encoding::encoder::EncodeDecorator;
use micro_web::{handler_fn, Server};
use serde::Deserialize;

/// User struct for demonstrating data extraction
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct User {
    name: String,
    zip: String,
}

/// Simple GET handler that demonstrates method and optional string extraction
///
/// Example request:
/// ```bash
/// curl http://127.0.0.1:8080/
/// ```
async fn simple_get(method: &Method, str: Option<String>, str2: Option<String>) -> String {
    println!("receive body: {}, {}", str.is_some(), str2.is_some());
    format!("receive from method: {}\r\n", method)
}

/// Handler that extracts form data into a User struct
///
/// Example request:
/// ```bash
/// curl -v -H "Transfer-Encoding: chunked" \
///      -d "name=hello&zip=world&c=abc" \
///      http://127.0.0.1:8080/
/// ```
async fn simple_handler_form_data(method: &Method, Form(user): Form<User>) -> String {
    format!("receive from method: {}, receive use: {:?}\r\n", method, user)
}

/// Handler that extracts JSON data into a User struct
///
/// Example request:
/// ```bash
/// curl -v -H "Transfer-Encoding: chunked" \
///      -H 'Content-Type: application/json' \
///      -d '{"name":"hello","zip":"world"}' \
///      http://127.0.0.1:8080/
/// ```
async fn simple_handler_json_data(method: &Method, Json(user): Json<User>) -> String {
    format!("receive from method: {}, receive use: {:?}\r\n", method, user)
}

/// Simple POST handler demonstrating method and optional string extraction
///
/// Example request:
/// ```bash
/// curl -X POST http://127.0.0.1:8080/
/// ```
async fn simple_handler_post(method: &Method, str: Option<String>, str2: Option<String>) -> String {
    println!("receive body: {:?}, {:?}", str, str2);
    format!("receive from method: {}\r\n", method)
}

/// Another GET handler for a different route
///
/// Example request:
/// ```bash
/// curl http://127.0.0.1:8080/4
/// ```
async fn simple_another_get(method: &Method, str: Option<String>, str2: Option<String>) -> String {
    println!("receive body: {:?}, {:?}", str, str2);
    format!("receive from method: {}\r\n", method)
}

/// Default handler for unmatched routes
///
/// Example request:
/// ```bash
/// curl http://127.0.0.1:8080/non-existent-path
/// ```
async fn default_handler() -> &'static str {
    "404 not found"
}

#[tokio::main]
async fn main() {
    // Build router with multiple routes and handlers
    let router = Router::builder()
        // Basic GET route
        .route("/", get(handler_fn(simple_get)))
        // POST route for form data with content-type filter
        .route(
            "/",
            post(handler_fn(simple_handler_form_data))
                .with(header(http::header::CONTENT_TYPE, mime::APPLICATION_WWW_FORM_URLENCODED.as_ref())),
        )
        // POST route for JSON data with content-type filter
        .route(
            "/",
            post(handler_fn(simple_handler_json_data))
                .with(header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())),
        )
        // Default POST route
        .route("/", post(handler_fn(simple_handler_post)))
        // Additional GET route
        .route("/4", get(handler_fn(simple_another_get)))
        // Add response encoding wrapper
        .with_decorator(EncodeDecorator)
        .build();

    // Configure and start the server
    Server::builder()
        .router(router)
        .bind("127.0.0.1:8080")
        .default_handler(handler_fn(default_handler))
        .build()
        .unwrap()
        .start()
        .await;
}
