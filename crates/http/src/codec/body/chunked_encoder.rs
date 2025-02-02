//! Encoder implementation for HTTP chunked transfer encoding.
//!
//! This module provides functionality to encode HTTP messages using chunked transfer encoding
//! as specified in [RFC 7230 Section 4.1](https://tools.ietf.org/html/rfc7230#section-4.1).
//!
//! The chunked encoding allows the sender to transmit message data in a series of chunks,
//! where each chunk is prefixed with its size in hexadecimal format.

use crate::protocol::{PayloadItem, SendError};
use bytes::{Buf, BytesMut};
use std::io::Write;
use tokio_util::codec::Encoder;

/// An encoder for handling HTTP chunked transfer encoding.
///
/// The encoder converts message data into chunks according to the chunked format:
/// - Each chunk starts with its size in hexadecimal
/// - Followed by CRLF
/// - Then the chunk data and CRLF
/// - A zero-sized chunk indicates the end of the message
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkedEncoder {
    /// Indicates if the final zero-length chunk has been sent
    eof: bool,
    /// Size of the current chunk being sent
    send_size: usize,
}

impl ChunkedEncoder {
    /// Creates a new ChunkedEncoder instance.
    ///
    /// The encoder starts in a non-EOF state, ready to encode chunks.
    pub fn new() -> Self {
        Self { eof: false, send_size: 0 }
    }

    /// Returns whether the encoder has finished sending all chunks.
    ///
    /// Returns true if the final zero-length chunk has been sent.
    pub fn is_finish(&self) -> bool {
        self.eof
    }
}

/// Implementation of the Encoder trait for chunked transfer encoding.
///
/// This implementation handles encoding of PayloadItems into chunked format:
/// - For PayloadItem::Chunk, writes the chunk size, data and terminating CRLF
/// - For PayloadItem::Eof, writes the final zero-length chunk
impl<D: Buf> Encoder<PayloadItem<D>> for ChunkedEncoder {
    type Error = SendError;

    /// Encodes a PayloadItem into chunked transfer encoding format.
    ///
    /// # Arguments
    /// * `item` - The PayloadItem to encode (either Chunk or Eof)
    /// * `dst` - The output buffer to write the encoded data to
    ///
    /// # Returns
    /// * `Ok(())` if encoding succeeds
    /// * `Err(SendError)` if encoding fails
    fn encode(&mut self, item: PayloadItem<D>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if self.eof {
            return Ok(());
        }

        match item {
            PayloadItem::Chunk(bytes) => {
                // Write chunk size in hex followed by CRLF
                write!(helper::Writer(dst), "{:X}\r\n", bytes.remaining())?;
                dst.reserve(bytes.remaining() + 2);
                // Write chunk data
                dst.extend_from_slice(bytes.chunk());
                // Write chunk terminating CRLF
                dst.extend_from_slice(b"\r\n");
                Ok(())
            }
            PayloadItem::Eof => {
                self.eof = true;
                // Write final zero-length chunk
                dst.extend_from_slice(b"0\r\n\r\n");
                Ok(())
            }
        }
    }
}

/// Helper module providing a Writer implementation for BytesMut.
///
/// This allows using std::fmt::Write with BytesMut for writing
/// chunk sizes in hexadecimal format.
mod helper {
    use bytes::{BufMut, BytesMut};
    use std::io;

    pub struct Writer<'a>(pub &'a mut BytesMut);

    impl io::Write for Writer<'_> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.put_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}
