//! HTTP response encoder module
//!
//! This module provides functionality for encoding HTTP responses using a streaming approach.
//! It handles both header encoding and payload encoding through a state machine pattern.
//!
//! # Components
//!
//! - [`ResponseEncoder`]: Main encoder that coordinates header and payload encoding
//! - Header encoding: Uses [`HeaderEncoder`] for encoding response headers
//! - Payload handling: Uses [`PayloadEncoder`] for encoding response bodies
//!
//! # Example
//!
//! ```no_run
//! use micro_http::codec::ResponseEncoder;
//! use tokio_util::codec::Encoder;
//! use bytes::BytesMut;
//!
//! let mut encoder = ResponseEncoder::new();
//! let mut buffer = BytesMut::new();
//! // ... encode response data to buffer ...
//! ```

use crate::codec::body::PayloadEncoder;
use crate::codec::header::HeaderEncoder;
use crate::protocol::{Message, PayloadSize, ResponseHead, SendError};
use bytes::{Buf, BytesMut};
use std::io;
use std::io::ErrorKind;
use tokio_util::codec::Encoder;
use tracing::error;

/// A encoder for HTTP responses that handles both headers and payload
///
/// The encoder operates in two phases:
/// 1. Header encoding: Encodes the response headers using [`HeaderEncoder`]
/// 2. Payload encoding: If present, encodes the response body using [`PayloadEncoder`]
pub struct ResponseEncoder {
    /// Encoder for HTTP response headers
    header_encoder: HeaderEncoder,
    /// Encoder for HTTP response payload (body)
    payload_encoder: Option<PayloadEncoder>,
}

impl ResponseEncoder {
    /// Creates a new `ResponseEncoder` instance
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for ResponseEncoder {
    fn default() -> Self {
        Self { header_encoder: HeaderEncoder, payload_encoder: None }
    }
}

impl<D: Buf> Encoder<Message<(ResponseHead, PayloadSize), D>> for ResponseEncoder {
    type Error = SendError;

    /// Attempts to encode an HTTP response to the provided buffer
    ///
    /// # Arguments
    ///
    /// * `item` - The message to encode, either headers or payload
    /// * `dst` - The buffer to write the encoded data to
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Successfully encoded the message
    /// - `Err(_)`: Encountered an encoding error
    fn encode(&mut self, item: Message<(ResponseHead, PayloadSize), D>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            Message::Header((head, payload_size)) => {
                // If a payload encoder already exists, it's an error
                if self.payload_encoder.is_some() {
                    error!("expect payload item but receive response head");
                    return Err(io::Error::from(ErrorKind::InvalidInput).into());
                }

                // Create a payload encoder based on the payload size
                let payload_encoder = parse_payload_encoder(payload_size);
                self.payload_encoder = Some(payload_encoder);
                // Encode the response headers
                self.header_encoder.encode((head, payload_size), dst)
            }

            Message::Payload(payload_item) => {
                // Get the payload encoder, return error if it doesn't exist
                let payload_encoder = if let Some(encoder) = &mut self.payload_encoder {
                    encoder
                } else {
                    error!("expect response header but receive payload item");
                    return Err(io::Error::from(ErrorKind::InvalidInput).into());
                };

                // Encode the payload
                let result = payload_encoder.encode(payload_item, dst);

                // Check if the payload encoder is finished
                let is_eof = payload_encoder.is_finish();
                // If finished, remove the payload encoder
                if is_eof {
                    self.payload_encoder.take();
                }

                result
            }
        }
    }
}

/// Creates a payload encoder based on the payload size
///
/// # Arguments
///
/// * `payload_size` - The size specification for the payload
///
/// # Returns
///
/// Returns a [`PayloadEncoder`] configured according to the payload size
fn parse_payload_encoder(payload_size: PayloadSize) -> PayloadEncoder {
    match payload_size {
        PayloadSize::Length(size) => PayloadEncoder::fix_length(size),
        PayloadSize::Chunked => PayloadEncoder::chunked(),
        PayloadSize::Empty => PayloadEncoder::empty(),
    }
}
