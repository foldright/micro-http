//! HTTP header encoder implementation for serializing HTTP response headers
//!
//! This module provides functionality for encoding HTTP response headers into raw bytes.
//! It handles serialization of status line, headers and manages content length or
//! transfer encoding headers according to HTTP/1.1 specification.
//!
//! # Features
//!
//! - Efficient header serialization
//! - Automatic handling of Content-Length and Transfer-Encoding headers
//! - Support for HTTP/1.1 responses
//! - Chunked transfer encoding support

use crate::protocol::{PayloadSize, ResponseHead, SendError};

use bytes::{BufMut, BytesMut};

use http::{header, HeaderValue, Version};
use std::io;
use std::io::{ErrorKind, Write};
use tokio_util::codec::Encoder;
use tracing::error;

/// Initial buffer size allocated for header serialization
const INIT_HEADER_SIZE: usize = 4 * 1024;

/// Encoder for HTTP response headers implementing the [`Encoder`] trait.
///
/// This encoder serializes a [`ResponseHead`] and [`PayloadSize`] into raw bytes,
/// automatically handling Content-Length or Transfer-Encoding headers based on the
/// payload size.
pub struct HeaderEncoder;

impl Encoder<(ResponseHead, PayloadSize)> for HeaderEncoder {
    type Error = SendError;

    /// Encodes HTTP response headers into the provided bytes buffer.
    ///
    /// # Arguments
    ///
    /// * `item` - Tuple of response header and payload size information
    /// * `dst` - Mutable reference to the destination buffer
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if encoding succeeds, or `Err(SendError)` if encoding fails
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - HTTP version is not supported (only HTTP/1.1 supported)
    /// - Writing to buffer fails
    fn encode(&mut self, item: (ResponseHead, PayloadSize), dst: &mut BytesMut) -> Result<(), Self::Error> {
        let (mut header, payload_size) = item;

        dst.reserve(INIT_HEADER_SIZE);
        match header.version() {
            Version::HTTP_11 => {
                write!(
                    FastWrite(dst),
                    "HTTP/1.1 {} {}\r\n",
                    header.status().as_str(),
                    header.status().canonical_reason().unwrap()
                )?;
            }
            v => {
                error!(http_version = ?v, "unsupported http version");
                return Err(io::Error::from(ErrorKind::Unsupported).into());
            }
        }

        // Set appropriate content length or transfer encoding header
        match payload_size {
            PayloadSize::Length(n) => match header.headers_mut().get_mut(header::CONTENT_LENGTH) {
                Some(value) => *value = n.into(),
                None => {
                    header.headers_mut().insert(header::CONTENT_LENGTH, n.into());
                }
            },
            PayloadSize::Chunked => match header.headers_mut().get_mut(header::TRANSFER_ENCODING) {
                Some(value) => *value = "chunked".parse().unwrap(),
                None => {
                    header.headers_mut().insert(header::TRANSFER_ENCODING, "chunked".parse().unwrap());
                }
            },
            PayloadSize::Empty => match header.headers_mut().get_mut(header::CONTENT_LENGTH) {
                Some(value) => *value = 0.into(),
                None => {
                    const ZERO_VALUE: HeaderValue =  HeaderValue::from_static("0");
                    header.headers_mut().insert(header::CONTENT_LENGTH, ZERO_VALUE);
                }
            },
        }

        // Write all headers
        for (header_name, header_value) in header.headers().iter() {
            dst.put_slice(header_name.as_ref());
            dst.put_slice(b": ");
            dst.put_slice(header_value.as_ref());
            dst.put_slice(b"\r\n");
        }
        dst.put_slice(b"\r\n");
        Ok(())
    }
}

/// Fast writer implementation for writing to BytesMut.
///
/// This is an optimization to avoid unnecessary bounds checking when writing
/// to the bytes buffer, since we've already reserved enough space.
struct FastWrite<'a>(&'a mut BytesMut);

impl Write for FastWrite<'_> {
    /// Writes a buffer into this writer, returning how many bytes were written.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.put_slice(buf);
        Ok(buf.len())
    }

    /// Flush this output stream, ensuring that all intermediately buffered contents reach their destination.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
