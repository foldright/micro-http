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
//! use std::error::Error;
//! use std::sync::Arc;
//! use tokio::net::TcpListener;
//! use micro_http::connection::HttpConnection;
//! use micro_http::handler::make_handler;
//! use micro_http::protocol::body::ReqBody;
//! 
//! #[tokio::main]
//! async fn main() {
//!     // Create TCP listener
//!     let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
//!     
//!     // Create handler
//!     let handler = Arc::new(make_handler(hello_world));
//!     
//!     // Accept connections
//!     loop {
//!         let (stream, _) = listener.accept().await.unwrap();
//!         let handler = handler.clone();
//!         
//!         // Spawn connection handler
//!         tokio::spawn(async move {
//!             let (reader, writer) = stream.into_split();
//!             let connection = HttpConnection::new(reader, writer);
//!             connection.process(handler).await.unwrap();
//!         });
//!     }
//! }
//! 
//! // Request handler
//! async fn hello_world(req: Request<ReqBody>) -> Result<Response<String>, Box<dyn Error + Send + Sync>> {
//!     Ok(Response::new("Hello World!".to_string()))
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
