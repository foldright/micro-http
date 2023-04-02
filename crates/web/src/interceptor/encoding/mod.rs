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
