//! Encoder implementation for HTTP message payloads.
//!
//! This module provides a unified encoder for handling different types of HTTP message bodies:
//! - Content-Length based payloads
//! - Chunked transfer encoding
//! - Messages with no body
//!
//! The encoder automatically handles the appropriate encoding strategy based on the message headers.

use crate::codec::body::chunked_encoder::ChunkedEncoder;
use crate::codec::body::length_encoder::LengthEncoder;
use crate::protocol::{PayloadItem, SendError};
use bytes::{Buf, BytesMut};
use tokio_util::codec::Encoder;

/// A unified encoder for handling HTTP message payloads.
///
/// This encoder supports three payload types:
/// - Fixed length payloads (using Content-Length)
/// - Chunked transfer encoding
/// - No body
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadEncoder {
    /// The specific encoding strategy to use
    kind: Kind,
}

/// Enum representing different payload encoding strategies.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    /// Encode payload with a fixed content length
    Length(LengthEncoder),

    /// Encode payload using chunked transfer encoding
    Chunked(ChunkedEncoder),

    /// Handle messages with no body
    NoBody,
}

impl PayloadEncoder {
    /// Creates a PayloadEncoder for messages with no body.
    pub fn empty() -> Self {
        Self { kind: Kind::NoBody }
    }

    /// Creates a PayloadEncoder for chunked transfer encoding.
    pub fn chunked() -> Self {
        Self { kind: Kind::Chunked(ChunkedEncoder::new()) }
    }

    /// Creates a PayloadEncoder for a fixed-length payload.
    ///
    /// # Arguments
    /// * `size` - The expected content length in bytes
    #[allow(unused)]
    pub fn fix_length(size: u64) -> Self {
        Self { kind: Kind::Length(LengthEncoder::new(size)) }
    }

    /// Returns whether this encoder handles chunked transfer encoding.
    #[allow(unused)]
    pub fn is_chunked(&self) -> bool {
        match &self.kind {
            Kind::Length(_) => false,
            Kind::Chunked(_) => true,
            Kind::NoBody => false,
        }
    }

    /// Returns whether this encoder handles messages with no body.
    #[allow(unused)]
    pub fn is_empty(&self) -> bool {
        match &self.kind {
            Kind::Length(_) => false,
            Kind::Chunked(_) => false,
            Kind::NoBody => true,
        }
    }

    /// Returns whether this encoder handles fixed-length payloads.
    #[allow(unused)]
    pub fn is_fix_length(&self) -> bool {
        match &self.kind {
            Kind::Length(_) => true,
            Kind::Chunked(_) => false,
            Kind::NoBody => false,
        }
    }

    /// Returns whether the encoder has finished sending all data.
    pub fn is_finish(&self) -> bool {
        match &self.kind {
            Kind::Length(encoder) => encoder.is_finish(),
            Kind::Chunked(encoder) => encoder.is_finish(),
            Kind::NoBody => true,
        }
    }
}

/// Implementation of the Encoder trait for HTTP payloads.
///
/// Delegates to the appropriate encoder based on the payload type.
impl<D: Buf> Encoder<PayloadItem<D>> for PayloadEncoder {
    type Error = SendError;

    /// Encodes a PayloadItem using the appropriate strategy.
    ///
    /// # Arguments
    /// * `item` - The PayloadItem to encode
    /// * `dst` - The output buffer to write the encoded data to
    ///
    /// # Returns
    /// * Delegates to the specific encoder implementation, or
    /// * Returns Ok(()) immediately for no-body messages
    fn encode(&mut self, item: PayloadItem<D>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match &mut self.kind {
            Kind::Length(encoder) => encoder.encode(item, dst),
            Kind::Chunked(encoder) => encoder.encode(item, dst),
            Kind::NoBody => Ok(()),
        }
    }
}
