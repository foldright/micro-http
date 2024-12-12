//! Decoder implementation for HTTP message payloads.
//!
//! This module provides a unified decoder for handling different types of HTTP message bodies:
//! - Content-Length based payloads
//! - Chunked transfer encoding
//! - Messages with no body
//!
//! The decoder automatically handles the appropriate decoding strategy based on the message headers.

use crate::codec::body::chunked_decoder::ChunkedDecoder;
use crate::codec::body::length_decoder::LengthDecoder;
use crate::protocol::{ParseError, PayloadItem};
use bytes::BytesMut;
use tokio_util::codec::Decoder;

/// A unified decoder for handling HTTP message payloads.
///
/// This decoder supports three payload types:
/// - Fixed length payloads (using Content-Length)
/// - Chunked transfer encoding
/// - No body
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadDecoder {
    /// The specific decoding strategy to use
    kind: Kind,
}

/// Enum representing different payload decoding strategies.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    /// Decode payload with a fixed content length
    Length(LengthDecoder),

    /// Decode payload using chunked transfer encoding
    Chunked(ChunkedDecoder),

    /// Handle messages with no body
    NoBody,
}

impl PayloadDecoder {
    /// Creates a PayloadDecoder for messages with no body.
    #[allow(unused)]
    pub fn empty() -> Self {
        Self { kind: Kind::NoBody }
    }

    /// Creates a PayloadDecoder for chunked transfer encoding.
    pub fn chunked() -> Self {
        Self { kind: Kind::Chunked(ChunkedDecoder::new()) }
    }

    /// Creates a PayloadDecoder for a fixed-length payload.
    ///
    /// # Arguments
    /// * `size` - The expected content length in bytes
    #[allow(unused)]
    pub fn fix_length(size: u64) -> Self {
        Self { kind: Kind::Length(LengthDecoder::new(size)) }
    }

    /// Returns whether this decoder handles chunked transfer encoding.
    #[allow(unused)]
    pub fn is_chunked(&self) -> bool {
        match &self.kind {
            Kind::Length(_) => false,
            Kind::Chunked(_) => true,
            Kind::NoBody => false,
        }
    }

    /// Returns whether this decoder handles messages with no body.
    #[allow(unused)]
    pub fn is_empty(&self) -> bool {
        match &self.kind {
            Kind::Length(_) => false,
            Kind::Chunked(_) => false,
            Kind::NoBody => true,
        }
    }

    /// Returns whether this decoder handles fixed-length payloads.
    #[allow(unused)]
    pub fn is_fix_length(&self) -> bool {
        match &self.kind {
            Kind::Length(_) => true,
            Kind::Chunked(_) => false,
            Kind::NoBody => false,
        }
    }
}

/// Implementation of the Decoder trait for HTTP payloads.
///
/// Delegates to the appropriate decoder based on the payload type.
impl Decoder for PayloadDecoder {
    type Item = PayloadItem;
    type Error = ParseError;

    /// Decodes bytes from the input buffer using the appropriate strategy.
    ///
    /// # Arguments
    /// * `src` - Source buffer containing the payload data
    ///
    /// # Returns
    /// * Delegates to the specific decoder implementation, or
    /// * Returns EOF immediately for no-body messages
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match &mut self.kind {
            Kind::Length(length_decoder) => length_decoder.decode(src),
            Kind::Chunked(chunked_decoder) => chunked_decoder.decode(src),
            Kind::NoBody => Ok(Some(PayloadItem::Eof)),
        }
    }
}
