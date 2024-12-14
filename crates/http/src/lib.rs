//! An asynchronous micro HTTP server implementation
//! 
//! This crate provides a lightweight, efficient, and modular HTTP/1.1 server implementation
//! built on top of tokio. It focuses on providing a clean API while maintaining high performance
//! through careful memory management and asynchronous processing.
//! 
//! # Features
//! 
//! - Full HTTP/1.1 protocol support
//! - Asynchronous I/O using tokio
//! - Streaming request and response bodies
//! - Chunked transfer encoding
//! - Keep-alive connections
//! - Expect-continue mechanism
//! - Efficient memory usage through zero-copy parsing
//! - Clean error handling
//! 
//! 
//! # Example
//! 
//! ```no_run
//! use http::{Request, Response, StatusCode};
//! use http_body_util::BodyExt;
//! use std::error::Error;
//! use std::sync::Arc;
//! use tokio::net::TcpListener;
//! use tracing::{error, info, warn, Level};
//! use tracing_subscriber::FmtSubscriber;
//! use micro_http::connection::HttpConnection;
//! use micro_http::handler::make_handler;
//! use micro_http::protocol::body::ReqBody;
//! 
//! #[tokio::main]
//! async fn main() {
//!     // Initialize logging
//!     let subscriber = FmtSubscriber::builder()
//!         .with_max_level(Level::INFO)
//!         .finish();
//!     tracing::subscriber::set_global_default(subscriber)
//!         .expect("setting default subscriber failed");
//!     
//!     info!(port = 8080, "start listening");
//!     let tcp_listener = match TcpListener::bind("127.0.0.1:8080").await {
//!         Ok(tcp_listener) => tcp_listener,
//!         Err(e) => {
//!             error!(cause = %e, "bind server error");
//!             return;
//!         }
//!     };
//!     
//!     let handler = Arc::new(make_handler(hello_world));
//!     
//!     loop {
//!         let (tcp_stream, _remote_addr) = match tcp_listener.accept().await {
//!             Ok(stream_and_addr) => stream_and_addr,
//!             Err(e) => {
//!                 warn!(cause = %e, "failed to accept");
//!                 continue;
//!             }
//!         };
//!         
//!         let handler = handler.clone();
//!         
//!         tokio::spawn(async move {
//!             let (reader, writer) = tcp_stream.into_split();
//!             let connection = HttpConnection::new(reader, writer);
//!             match connection.process(handler).await {
//!                 Ok(_) => {
//!                     info!("finished process, connection shutdown");
//!                 }
//!                 Err(e) => {
//!                     error!("service has error, cause {}, connection shutdown", e);
//!                 }
//!             }
//!         });
//!     }
//! }
//! 
//! async fn hello_world(request: Request<ReqBody>) -> Result<Response<String>, Box<dyn Error + Send + Sync>> {
//!     let path = request.uri().path().to_string();
//!     info!("request path {}", path);
//!     
//!     let (_header, body) = request.into_parts();
//!     
//!     let body_bytes = body.collect().await?.to_bytes();
//!     info!(body = std::str::from_utf8(&body_bytes[..]).unwrap(), "receiving request body");
//!     
//!     let response_body = "Hello World!\r\n";
//!     let response = Response::builder()
//!         .status(StatusCode::OK)
//!         .header(http::header::CONTENT_LENGTH, response_body.len())
//!         .body(response_body.to_string())
//!         .unwrap();
//!     
//!     Ok(response)
//! }
//! ```
//! 
//! 
//! # Architecture
//! 
//! The crate is organized into several key modules:
//! 
//! - [`connection`]: Core connection handling and lifecycle management
//! - [`protocol`]: Protocol types and abstractions
//! - [`codec`]: Protocol encoding/decoding implementation
//! - [`handler`]: Request handler traits and utilities
//! 
//! 
//! 
//! # Core Components
//! 
//! ## Connection Handling
//! 
//! The [`connection::HttpConnection`] type is the main entry point for processing HTTP connections.
//! It manages the full lifecycle of connections including:
//! 
//! - Request parsing
//! - Body streaming
//! - Response generation
//! - Keep-alive handling
//! 
//! ## Request Processing
//! 
//! Requests are processed through handler functions that implement the [`handler::Handler`] trait.
//! The crate provides utilities for creating handlers from async functions through
//! [`handler::make_handler`].
//! 
//! ## Body Streaming
//! 
//! Request and response bodies are handled through streaming interfaces that implement
//! the `http_body::Body` trait. This enables efficient processing of large payloads
//! without buffering entire bodies in memory.
//! 
//! ## Error Handling
//! 
//! The crate uses custom error types that implement `std::error::Error`:
//! 
//! - [`protocol::HttpError`]: Top-level error type
//! - [`protocol::ParseError`]: Request parsing errors
//! - [`protocol::SendError`]: Response sending errors
//! 
//! # Performance Considerations
//! 
//! The implementation focuses on performance through:
//! 
//! - Zero-copy parsing where possible
//! - Efficient buffer management
//! - Streaming processing of bodies
//! - Concurrent request/response handling
//! - Connection keep-alive
//! 
//! # Limitations
//! 
//! - HTTP/1.1 only (currently HTTP/2 or HTTP/3 is not supported)
//! - No TLS support (use a reverse proxy for HTTPS)
//! - Maximum header size: 8KB
//! - Maximum number of headers: 64
//! 
//! # Safety
//! 
//! The crate uses unsafe code in a few well-documented places for performance
//! optimization, particularly in header parsing. All unsafe usage is carefully
//! reviewed and tested.


pub mod codec;
pub mod connection;
pub mod handler;
pub mod protocol;

mod utils;
pub(crate) use utils::ensure;
