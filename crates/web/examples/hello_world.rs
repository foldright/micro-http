//! Basic example demonstrating how to create a simple web server using micro_web.
//! This example shows:
//! - How to define route handlers
//! - How to set up a router with middleware
//! - How to configure and start a server

use micro_web::date::DateServiceDecorator;
use micro_web::router::{get, Router};
use micro_web::{handler_fn, Server};

/// A simple handler that returns "hello world"
async fn hello_world() -> &'static str {
    "hello world"
}

/// Default handler for unmatched handlers (404 responses)
///
/// This handler is called when no other handlers match the incoming request
async fn default_handler() -> &'static str {
    "404 not found"
}

#[tokio::main]
async fn main() {
    // Create a new router using the builder
    let router = Router::builder()
        // Add a route that matches GET requests to the root path "/"
        // handler_fn converts our async function into a handler
        .route("/", get(handler_fn(hello_world)))
        // Add middleware that will add date headers to responses
        .with_global_decorator(DateServiceDecorator)
        .build();

    // Configure and start the server
    Server::builder()
        // Attach our router to handle incoming requests
        .router(router)
        // Set the address and port to listen on
        .bind("127.0.0.1:3000")
        // Set a handler for requests that don't match any routes
        .default_handler(handler_fn(default_handler))
        // Build the server
        .build()
        .unwrap()
        // Start the server and wait for it to finish
        .start()
        .await;
}
