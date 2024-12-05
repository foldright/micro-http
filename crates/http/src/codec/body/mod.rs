//! HTTP body handling module for processing request and response payloads
//! 
//! This module provides functionality for encoding and decoding HTTP message bodies
//! using different transfer strategies. It supports both chunked transfer encoding
//! and content-length based transfers.
//! 
//! # Components
//! 
//! ## Decoders
//! - [`ChunkedDecoder`]: Handles chunked transfer encoded payloads
//! - [`LengthDecoder`]: Processes fixed-length payloads
//! - [`PayloadDecoder`]: Main decoder that coordinates different decoding strategies
//! 
//! ## Encoders
//! - [`ChunkedEncoder`]: Implements chunked transfer encoding
//! - [`LengthEncoder`]: Handles fixed-length payload encoding
//! - [`PayloadEncoder`]: Main encoder that manages different encoding strategies
//! 
//! # Features
//! 
//! - Support for chunked transfer encoding (RFC 7230)
//! - Content-Length based payload handling
//! - Streaming processing of message bodies
//! - Efficient memory usage through BytesMut
//! - State machine based processing

mod chunked_decoder;
mod chunked_encoder;
mod length_decoder;
mod length_encoder;
mod payload_decoder;
mod payload_encoder;

pub use payload_decoder::PayloadDecoder;
pub use payload_encoder::PayloadEncoder;
