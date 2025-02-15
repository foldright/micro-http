//! HTTP request decoder module
//!
//! This module provides functionality for decoding HTTP requests using a streaming approach.
//! It handles both header parsing and payload decoding through a state machine pattern.
//!
//! # Components
//!
//! - [`RequestDecoder`]: Main decoder that coordinates header and payload parsing
//! - Header parsing: Uses [`HeaderDecoder`] for parsing request headers
//! - Payload handling: Uses [`PayloadDecoder`] for handling request bodies if any
//!
//! # Example
//!
//! ```no_run
//! use micro_http::codec::RequestDecoder;
//! use tokio_util::codec::Decoder;
//! use bytes::BytesMut;
//!
//! let mut decoder = RequestDecoder::new();
//! let mut buffer = BytesMut::new();
//! // ... add request data to buffer ...
//! let result = decoder.decode(&mut buffer);
//! ```

use crate::codec::body::PayloadDecoder;
use crate::codec::header::HeaderDecoder;
use crate::protocol::{Message, ParseError, PayloadItem, PayloadSize, RequestHeader};
use bytes::BytesMut;
use tokio_util::codec::Decoder;

/// A decoder for HTTP requests that handles both headers and payload
///
/// The decoder operates in two phases:
/// 1. Header parsing: Decodes the request headers using [`HeaderDecoder`]
/// 2. Payload parsing: If present, decodes the request body using [`PayloadDecoder`]
///
/// # State Machine
///
/// The decoder maintains its state through the `payload_decoder` field:
/// - `None`: Currently parsing headers
/// - `Some(PayloadDecoder)`: Currently parsing payload
pub struct RequestDecoder {
    header_decoder: HeaderDecoder,
    payload_decoder: Option<PayloadDecoder>,
}

impl RequestDecoder {
    /// Creates a new `RequestDecoder` instance
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for RequestDecoder {
    fn default() -> Self {
        Self { header_decoder: HeaderDecoder, payload_decoder: None }
    }
}

impl Decoder for RequestDecoder {
    type Item = Message<(RequestHeader, PayloadSize)>;
    type Error = ParseError;

    /// Attempts to decode an HTTP request from the provided buffer
    ///
    /// # Returns
    ///
    /// - `Ok(Some(Message::Header(_)))`: Successfully decoded request headers
    /// - `Ok(Some(Message::Payload(_)))`: Successfully decoded a payload chunk
    /// - `Ok(None)`: Need more data to proceed
    /// - `Err(_)`: Encountered a parsing error
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // parse payload if have payload_decoder
        if let Some(payload_decoder) = &mut self.payload_decoder {
            let message = match payload_decoder.decode(src)? {
                Some(item @ PayloadItem::Chunk(_)) => Some(Message::Payload(item)),
                Some(item @ PayloadItem::Eof) => {
                    // no need payload decoder in this request now
                    self.payload_decoder.take();
                    Some(Message::Payload(item))
                }
                None => None,
            };

            return Ok(message);
        }

        // parse request
        let message = match self.header_decoder.decode(src)? {
            Some((header, payload_size)) => {
                self.payload_decoder = Some(payload_size.into());
                Some(Message::Header((header, payload_size)))
            }
            None => None,
        };

        Ok(message)
    }
}
