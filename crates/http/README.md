# Micro HTTP

A lightweight, efficient, and modular HTTP/1.1 server implementation built on top of tokio.

## Features

- Full HTTP/1.1 protocol support
- Asynchronous I/O using tokio
- Streaming request and response bodies
- Chunked transfer encoding
- Keep-alive connections
- Expect-continue mechanism
- Efficient memory usage through zero-copy parsing
- Clean error handling
- Structured logging with tracing

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
micro-http = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "net", "io-util", "macros", "sync", "signal", "test-util"] }
http = "1"
http-body = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
```

## Example

Here's a simple HTTP server that responds with "Hello World!":

```rust
use http::{Request, Response, StatusCode};
use http_body_util::BodyExt;
use std::error::Error;
use std::sync::Arc;

use tokio::net::TcpListener;

use micro_http::connection::HttpConnection;
use micro_http::handler::make_handler;
use micro_http::protocol::body::ReqBody;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!(port = 8080, "start listening");
    let tcp_listener = match TcpListener::bind("127.0.0.1:8080").await {
        Ok(tcp_listener) => tcp_listener,
        Err(e) => {
            error!(cause = %e, "bind server error");
            return;
        }
    };

    let handler = make_handler(simple_handler);
    let handler = Arc::new(handler);
    loop {
        let (tcp_stream, _remote_addr) = match tcp_listener.accept().await {
            Ok(stream_and_addr) => stream_and_addr,
            Err(e) => {
                warn!(cause = %e, "failed to accept");
                continue;
            }
        };

        let handler = handler.clone();

        // one connection per task
        tokio::spawn(async move {
            let (reader, writer) = tcp_stream.into_split();
            let connection = HttpConnection::new(reader, writer);
            match connection.process(handler).await {
                Ok(_) => {
                    info!("finished process, connection shutdown");
                }
                Err(e) => {
                    error!("service has error, cause {}, connection shutdown", e);
                }
            }
        });
    }
}

async fn simple_handler(request: Request<ReqBody>) -> Result<Response<String>, Box<dyn Error + Send + Sync>> {
    let path = request.uri().path().to_string();
    info!("request path {}", path);

    let (_header, body) = request.into_parts();

    let body_bytes = body.collect().await?.to_bytes();
    info!(body = std::str::from_utf8(&body_bytes[..]).unwrap(), "receiving request body");

    let response_body = "Hello World!\r\n";
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(http::header::CONTENT_LENGTH, response_body.len())
        .body(response_body.to_string())
        .unwrap();

    Ok(response)
}

```

## Architecture

The crate is organized into several key modules:

- `connection`: Core connection handling and lifecycle management
- `protocol`: Protocol types and abstractions
- `codec`: Protocol encoding/decoding implementation
- `handler`: Request handler traits and utilities

## Performance Considerations

The implementation focuses on performance through:

- Zero-copy parsing where possible
- Efficient buffer management
- Streaming processing of bodies
- Concurrent request/response handling
- Connection keep-alive

## Limitations

- HTTP/1.1 only (HTTP/2 or HTTP/3 not supported)
- No TLS support (use a reverse proxy for HTTPS)
- Maximum header size: 8KB
- Maximum number of headers: 64

## Safety

The crate uses unsafe code in a few well-documented places for performance optimization, particularly in header parsing. All unsafe usage is carefully reviewed and tested.

## License

This project is licensed under the MIT License or Apache-2.0 License, pick one.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
