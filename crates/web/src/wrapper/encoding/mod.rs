//! Module for handling HTTP response body encoding.
//!
//! This module provides functionality for encoding HTTP response bodies using different compression
//! algorithms like gzip, deflate, zstd, and brotli. It works in conjunction with the encoder
//! module to provide a complete encoding solution.
//!
//! The main components are:
//! - `Writer`: An internal buffer implementation for collecting encoded data
//! - `encoder`: A sub-module containing the encoding logic and request handler wrapper
//!
//! The implementation is inspired by the actix-http crate's encoding functionality.

use bytes::{Bytes, BytesMut};
use std::io;

pub mod encoder;

// inspired by from actix-http
pub(crate) struct Writer {
    buf: BytesMut,
}

impl Writer {
    fn new() -> Self {
        Self { buf: BytesMut::with_capacity(4096) }
    }

    fn take(&mut self) -> Bytes {
        self.buf.split().freeze()
    }
}

impl io::Write for Writer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
