# Micro Web

A lightweight, modular web framework built on top of micro-http, providing an elegant and efficient way to build web applications in Rust.

## Features

- Built on micro-http for high performance HTTP handling
- Flexible routing with path parameters and filters
- Type-safe request/response handling with automatic data extraction
- Async/await support throughout
- Extensible architecture with decorators and middleware
- Built-in support for common tasks (compression, date headers, etc.)

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
micro-web = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "net", "io-util", "macros", "sync", "signal"] }
```

### Basic Example

Here's a simple hello world example:

```rust
use micro_web::router::{get, Router};
use micro_web::{handler_fn, Server};
use micro_web::date::DateServiceDecorator;

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
```

### Advanced Example 

Here's a more complete example showing different request handlers, data extraction, and middleware:

```rust
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
        .with_global_decorator(EncodeDecorator)
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
```

## Core Components

### Router

The router provides flexible request routing with support for:
- Path parameters and pattern matching
- Query parameters extraction
- HTTP method filtering
- Header-based filtering
- Composable filter chains
- Nested routers

### Request Handlers

Handlers can be created from async functions using `handler_fn`. The framework supports automatic type conversion for both request data extraction and response generation:

```rust
// Simple handler returning a string
async fn hello() -> &'static str {
    "Hello"
}

// Handler with path parameter
async fn user_info(Path(user_id): Path<String>) -> String {
    format!("User ID: {}", user_id)
}

// Handler with JSON request body
async fn create_user(Json(user): Json<User>) -> impl Responder {
    // Process user...
    StatusCode::CREATED
}
```

### Data Extraction

The framework provides type-safe extractors for common data formats:

- Path parameters (`Path<T>`)
- Query parameters (`Query<T>`)
- Form data (`Form<T>`)
- JSON (`Json<T>`)
- Headers (`HeaderMap`)
- Custom extractors via `FromRequest` trait

### Middleware and Decorators

The framework uses a flexible decorator pattern for middleware:

```rust
// Add multiple decorators
router.builder()
    .with_global_decorator(DateServiceDecorator) // Adds Date header
    .with_global_decorator(CompressionDecorator) // Handles response compression
    .build();
```

Built-in decorators include:
- `DateServiceDecorator`: Adds date headers to responses
- `CompressionDecorator`: Handles response compression
- Custom decorators via `Decorator` trait

## Architecture

The framework is built with a modular architecture:

- `router`: Request routing and filter system
- `handler`: Request handler traits and implementations
- `extract`: Type-safe data extraction
- `decorator`: Middleware and response transformation
- `responder`: Response generation

## Performance

Built on micro-http, the framework maintains high performance through:

- Zero-allocation routing where possible
- Efficient middleware chaining via decorators
- Minimal copying of request/response data
- Async I/O throughout
- Smart date handling with cached timestamps
- Optimized header handling

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Development

1. Clone the repository
2. Run tests: `cargo test`
3. Run examples: `cargo run --example hello_world`

### Guidelines

- Write tests for new functionality
- Follow Rust best practices
- Document public APIs
- Keep performance in mind

## License

This project is licensed under the MIT License or Apache-2.0 License, pick one.

## Safety

This crate uses `unsafe` code in limited, well-documented cases for performance optimization. All unsafe usage is carefully reviewed and tested.

## Status

This project is in alpha stage. APIs may change between versions.

## Related Projects

- [micro-http](../http/README.md): The underlying HTTP server implementation

