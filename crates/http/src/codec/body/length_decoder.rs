//! Decoder implementation for HTTP messages with Content-Length header.
//!
//! This module provides functionality to decode HTTP messages where the payload size
//! is specified by the Content-Length header, as defined in
//! [RFC 7230 Section 3.3.2](https://tools.ietf.org/html/rfc7230#section-3.3.2).

use std::cmp;

use crate::protocol::{ParseError, PayloadItem};
use bytes::BytesMut;
use tokio_util::codec::Decoder;

/// A decoder for handling HTTP messages with a known content length.
///
/// The decoder tracks the remaining bytes to be read and ensures the total
/// payload matches the specified content length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LengthDecoder {
    /// The number of bytes remaining to be read from the payload
    length: u64,
}

impl LengthDecoder {
    /// Creates a new LengthDecoder instance.
    ///
    /// # Arguments
    /// * `length` - The total content length to decode, specified by Content-Length header
    pub fn new(length: u64) -> Self {
        Self { length }
    }
}

/// Implementation of the Decoder trait for content-length based decoding.
///
/// This implementation tracks the remaining bytes to read and ensures the total
/// payload matches the specified content length.
impl Decoder for LengthDecoder {
    type Item = PayloadItem;
    type Error = ParseError;

    /// Decodes bytes from the input buffer according to the content length.
    ///
    /// # Arguments
    /// * `src` - Source buffer containing the payload data
    ///
    /// # Returns
    /// * `Ok(Some(PayloadItem::Eof))` when all bytes have been read
    /// * `Ok(Some(PayloadItem::Chunk(bytes)))` when a chunk is successfully decoded
    /// * `Ok(None)` when more data is needed
    /// * `Err(ParseError)` if decoding fails
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if self.length == 0 {
            return Ok(Some(PayloadItem::Eof));
        }

        if src.is_empty() {
            return Ok(None);
        }

        // Read the minimum of remaining length and available bytes
        let len = cmp::min(self.length, src.len() as u64);
        let bytes = src.split_to(len as usize).freeze();

        self.length -= bytes.len() as u64;
        Ok(Some(PayloadItem::Chunk(bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut buffer: BytesMut = BytesMut::from(&b"101234567890abcdef\r\n\r\n"[..]);

        let mut length_decoder = LengthDecoder::new(10);
        let item = length_decoder.decode(&mut buffer);

        let payload = item.unwrap().unwrap();
        assert!(payload.is_chunk());

        let bytes = payload.as_bytes().unwrap();

        assert_eq!(bytes.len(), 10);

        assert_eq!(&bytes[..], b"1012345678");
        assert_eq!(&buffer[..], b"90abcdef\r\n\r\n");
    }
}
