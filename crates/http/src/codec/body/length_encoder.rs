//! Encoder implementation for HTTP messages with Content-Length header.
//! 
//! This module provides functionality to encode HTTP messages where the payload size
//! is specified by the Content-Length header, ensuring the total bytes sent matches
//! the declared content length.

use crate::protocol::{PayloadItem, SendError};
use bytes::{Buf, BytesMut};
use tokio_util::codec::Encoder;
use tracing::warn;

/// An encoder for handling HTTP messages with a known content length.
///
/// The encoder tracks the remaining bytes to be sent and ensures the total
/// payload matches the specified content length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LengthEncoder {
    /// Indicates if the final EOF marker has been received
    received_eof: bool,
    /// The number of bytes remaining to be sent
    length: u64,
}

impl LengthEncoder {
    /// Creates a new LengthEncoder instance.
    ///
    /// # Arguments
    /// * `length` - The total content length to encode, specified by Content-Length header
    pub fn new(length: u64) -> Self {
        Self { received_eof: false, length }
    }

    /// Returns whether the encoder has finished sending all data.
    ///
    /// Returns true if all bytes have been sent and EOF has been received.
    pub fn is_finish(&self) -> bool {
        self.length == 0 && self.received_eof
    }
}

/// Implementation of the Encoder trait for content-length based encoding.
///
/// This implementation tracks the remaining bytes to send and ensures the total
/// payload matches the specified content length.
impl<D: Buf> Encoder<PayloadItem<D>> for LengthEncoder {
    type Error = SendError;

    /// Encodes a PayloadItem according to the content length.
    ///
    /// # Arguments
    /// * `item` - The PayloadItem to encode (either Chunk or Eof)
    /// * `dst` - The output buffer to write the encoded data to
    ///
    /// # Returns
    /// * `Ok(())` if encoding succeeds
    /// * `Err(SendError)` if encoding fails
    fn encode(&mut self, item: PayloadItem<D>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if self.length == 0 && !item.is_eof() {
            warn!("encode payload_item but no need to encode anymore");
            return Ok(());
        }

        match item {
            PayloadItem::Chunk(bytes) => {
                if !bytes.has_remaining() {
                    return Ok(());
                }
                dst.extend_from_slice(bytes.chunk());
                self.length -= bytes.remaining() as u64;
                Ok(())
            }
            PayloadItem::Eof => {
                self.received_eof = true;
                Ok(())
            }
        }
    }
}
