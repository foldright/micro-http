//! HTTP codec module for encoding and decoding HTTP messages
//! 
//! This module provides functionality for streaming HTTP message processing,
//! including request decoding and response encoding. It uses a state machine
//! pattern to handle both headers and payload data efficiently.
//! 
//! # Architecture
//! 
//! The codec module is organized into several components:
//! 
//! - Request handling:
//!   - [`RequestDecoder`]: Decodes incoming HTTP requests
//!   - Header parsing via [`header`] module
//!   - Payload decoding via [`body`] module
//! 
//! - Response handling:
//!   - [`ResponseEncoder`]: Encodes outgoing HTTP responses
//!   - Header encoding via [`header`] module
//!   - Payload encoding via [`body`] module
//! 
//! # Example
//! 
//! ```no_run
//! use micro_http::codec::{RequestDecoder, ResponseEncoder};
//! use tokio_util::codec::{Decoder, Encoder};
//! use bytes::BytesMut;
//! 
//! // Decode incoming request
//! let mut decoder = RequestDecoder::new();
//! let mut request_buffer = BytesMut::new();
//! let request = decoder.decode(&mut request_buffer);
//! 
//! // Encode outgoing response
//! let mut encoder = ResponseEncoder::new();
//! let mut response_buffer = BytesMut::new();
//! // ... encode response ...
//! ```
//! 
//! # Features
//! 
//! - Streaming processing of HTTP messages
//! - Support for chunked transfer encoding
//! - Content-Length based payload handling
//! - Efficient header parsing and encoding
//! - State machine based processing

mod body;
mod header;
mod request_decoder;
mod response_encoder;

pub use request_decoder::RequestDecoder;
pub use response_encoder::ResponseEncoder;
