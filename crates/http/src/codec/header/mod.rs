//! HTTP header processing module for encoding and decoding headers
//!
//! This module provides functionality for handling HTTP headers in both requests
//! and responses. It includes efficient parsing and encoding mechanisms for HTTP
//! header fields.
//!
//! # Components
//!
//! - [`HeaderDecoder`]: Decodes HTTP headers from raw bytes
//!   - Supports standard HTTP/1.1 header format
//!   - Handles header field validation
//!   - Manages header size limits
//!
//! - [`HeaderEncoder`]: Encodes HTTP headers to bytes
//!   - Implements standard HTTP/1.1 header formatting
//!   - Handles header field serialization
//!   - Manages content-length and transfer-encoding headers
//!
//!
//! # Features
//!
//! - Efficient header parsing and encoding
//! - Support for standard HTTP headers
//! - Memory-efficient processing
//! - Header validation
//! - Size limit enforcement

mod header_decoder;
mod header_encoder;

pub use header_decoder::HeaderDecoder;
pub use header_encoder::HeaderEncoder;
