//! HTTP connection handling module
//! 
//! This module provides functionality for managing HTTP connections and processing
//! HTTP requests and responses. It implements the core connection handling logic
//! for the HTTP server.
//! 
//! # Components
//! 
//! - [`HttpConnection`]: Main connection handler that:
//!   - Manages the lifecycle of HTTP connections
//!   - Processes incoming requests
//!   - Handles response streaming
//!   - Supports keep-alive connections
//!   - Implements expect-continue handling
//! 
//! # Features
//! 
//! - Asynchronous I/O handling
//! - Streaming request and response processing
//! - Keep-alive connection support
//! - Error handling and recovery
//! - Expect-continue mechanism
//! - Efficient memory usage through buffering

mod http_connection;

pub use http_connection::HttpConnection;
