# Micro Web

A lightweight, modular web framework built on top of micro-http, providing an elegant and efficient way to build web applications in Rust.

## Features

- Built on micro-http for high performance HTTP handling
- Flexible routing with path parameters
- Request/Response abstraction
- Async/await support
- Extensible architecture

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
use micro_web::wrapper::DateWrapper;
use micro_web::router::{get, Router}; 
use micro_web::{handler_fn, Server};

// Handler function
async fn hello_world() -> &'static str {
    "hello world"
}

// Default 404 handler
async fn default_handler() -> &'static str {
    "404 not found"
}

#[tokio::main]
async fn main() {
    // Create router
    let router = Router::builder()
        .route("/", get(handler_fn(hello_world)))
        .wrap(DateWrapper)
        .build();

    // Configure and start server
    Server::builder()
        .router(router)
        .bind("127.0.0.1:3000")
        .default_handler(handler_fn(default_handler))
        .build()
        .unwrap()
        .start()
        .await;
}
```

### Advanced Example 

Here's a more complete example showing different request handlers and data extraction:

```rust
use micro_web::extract::{Form, Json};
use micro_web::filter::header;
use micro_web::wrapper::EncodeWrapper;
use micro_web::router::{get, post, Router};
use micro_web::{handler_fn, Server};
use serde::Deserialize;

// Data structure for form/JSON extraction
#[derive(Deserialize)]
struct User {
    name: String,
    zip: String,
}

// Form data handler
async fn handle_form(Form(user): Form<User>) -> String {
    format!("Received user: {:?}", user)
}

// JSON data handler
async fn handle_json(Json(user): Json<User>) -> String {
    format!("Received user: {:?}", user)
}

#[tokio::main]
async fn main() {
    let router = Router::builder()
        // Basic GET route
        .route("/", get(handler_fn(hello_world)))
        
        // POST route for form data
        .route(
            "/user",
            post(handler_fn(handle_form))
                .with(header(http::header::CONTENT_TYPE, mime::APPLICATION_WWW_FORM_URLENCODED.as_ref())),
        )
        
        // POST route for JSON data
        .route(
            "/user",
            post(handler_fn(handle_json))
                .with(header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())),
        )
        
        .wrap(EncodeWrapper)
        .build();

    Server::builder()
        .router(router)
        .bind("127.0.0.1:8080")
        .build()?
        .start()
        .await;
}
```

## Core Components

### Router

The router provides flexible request routing with support for:
- Path parameters
- Query parameters
- HTTP method matching
- Wildcard routes
- Nested routers
- Request filters

### Middleware

Middleware provides a way to process requests/responses before they reach your handlers. Built-in middleware includes:

- `DateWrapper`: Adds date headers to responses
- `EncodeWrapper`: Handles response encoding
- you can custom others through `Wrapper`

Example of adding middleware:

```rust
router.builder()
    .wrap(DateWrapper)
    .wrap(EncodeWrapper)
    .build();
```

### Request Handlers

Handlers can be created from async functions using `handler_fn`:

```rust
async fn my_handler() -> String {
    "Hello".to_string()
}

router.route("/", get(handler_fn(my_handler)));
```

### Data Extraction

The framework supports extracting data from requests in different formats:

```rust
// Form data extraction
async fn handle_form(Form(data): Form<MyData>) -> String { ... }

// JSON extraction
async fn handle_json(Json(data): Json<MyData>) -> String { ... }
```

## Architecture

The framework is built with a modular architecture:

- `router`: Request routing and handler dispatch
- `wrapper`: Middleware processing pipeline
- `handler`: Request handler traits and implementations
- `response`: Response building and formatting

## Performance

Built on micro-http, the framework maintains high performance through:

- Zero-allocation routing where possible
- Efficient middleware chaining
- Minimal copying of request/response data
- Async I/O throughout
- Smart memory management

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

